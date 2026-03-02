//! Gmail 同期オーケストレーション。

use std::sync::Arc;

use sqlx::sqlite::SqlitePool;
use tokio::sync::Mutex;

use super::error_handler::ErrorReporter;
use super::{clamp_batch_size, BatchCommandsApp, TauriBatchCommandsApp};
use crate::batch_runner::{BatchProgressEvent, BatchRunner};
use crate::config;
use crate::e2e_mocks::GmailClientForE2E;
use crate::gmail::{
    create_sync_input, fetch_all_message_ids, GmailSyncContext, GmailSyncTask,
    ShopSettingsCacheForSync, SyncGuard, SyncState, GMAIL_SYNC_EVENT_NAME, GMAIL_SYNC_TASK_NAME,
};
use crate::logic::sync_logic;
use crate::repository::{
    EmailRepository, ShopSettingsRepository, SqliteEmailRepository, SqliteShopSettingsRepository,
};

/// Gmail全件同期タスクの本体。コマンド・トレイ両方から呼ぶ。
pub async fn run_sync_task(app: tauri::AppHandle, pool: SqlitePool, sync_state: SyncState) {
    let app = TauriBatchCommandsApp { app };
    run_sync_task_with(&app, pool, sync_state).await
}

/// Gmail差分同期タスクの本体。DB内の最新日時以降のメールのみ取得する。
pub async fn run_incremental_sync_task(
    app: tauri::AppHandle,
    pool: SqlitePool,
    sync_state: SyncState,
) {
    let app = TauriBatchCommandsApp { app };
    run_incremental_sync_task_with(&app, pool, sync_state).await
}

async fn run_sync_task_with<A: BatchCommandsApp>(app: &A, pool: SqlitePool, sync_state: SyncState) {
    run_sync_core(app, pool, sync_state, false).await
}

async fn run_incremental_sync_task_with<A: BatchCommandsApp>(
    app: &A,
    pool: SqlitePool,
    sync_state: SyncState,
) {
    run_sync_core(app, pool, sync_state, true).await
}

async fn run_sync_core<A: BatchCommandsApp>(
    app: &A,
    pool: SqlitePool,
    sync_state: SyncState,
    incremental: bool,
) {
    let mode_label = if incremental {
        "incremental (requested, may fallback to full)"
    } else {
        "full"
    };
    log::info!("Starting Gmail sync ({mode_label}) with BatchRunner<GmailSyncTask>...");

    let err = ErrorReporter::new(app, GMAIL_SYNC_TASK_NAME, GMAIL_SYNC_EVENT_NAME);

    if !sync_state.try_start() {
        log::warn!("Sync is already in progress");
        err.report_zero("Sync is already in progress");
        return;
    }

    let _guard = SyncGuard::new(&sync_state);

    let email_repo = SqliteEmailRepository::new(pool.clone());
    let shop_repo = SqliteShopSettingsRepository::new(pool.clone());

    let enabled_shops = match shop_repo.get_enabled().await {
        Ok(shops) => shops,
        Err(e) => {
            let msg = format!("Failed to fetch shop settings: {}", e);
            err.report_zero(&msg);
            sync_state.set_error(&e);
            return;
        }
    };

    let sender_addresses: Vec<String> = enabled_shops
        .iter()
        .map(|s| s.sender_address.clone())
        .collect();

    log::info!(
        "Starting sync ({mode_label}) with {} enabled sender addresses",
        sender_addresses.len()
    );

    let app_config_dir = match app.app_config_dir() {
        Ok(dir) => dir,
        Err(message) => {
            err.report_zero(&message);
            sync_state.set_error(&message);
            return;
        }
    };
    let config = config::load(&app_config_dir).unwrap_or_else(|e| {
        log::error!("Failed to load config: {}", e);
        config::AppConfig::default()
    });
    let batch_size = clamp_batch_size(config.sync.batch_size, 50);

    let gmail_client = match app.create_gmail_client().await {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("Failed to create Gmail client: {}", e);
            err.report_zero(&msg);
            sync_state.set_error(&e);
            return;
        }
    };

    // 差分同期の場合、DB内の最新 internal_date を起点にする
    let after_date = if incremental {
        match email_repo.get_latest_internal_date().await {
            Ok(Some(ts)) => {
                // 安全マージンとして1日（86,400,000ms）前にずらす（Gmail API の after: は日単位のため）
                let margin_ms = 86_400_000i64;
                let safe_ts = ts.saturating_sub(margin_ms);
                match chrono::DateTime::from_timestamp_millis(safe_ts) {
                    Some(dt) => {
                        let rfc = dt.to_rfc3339();
                        log::info!(
                            "Incremental sync: using after_date={} (original latest={})",
                            rfc,
                            ts
                        );
                        Some(rfc)
                    }
                    None => {
                        log::warn!("Invalid latest internal_date {ts}, falling back to full sync");
                        None
                    }
                }
            }
            Ok(None) => {
                log::info!("No existing emails in DB, falling back to full sync");
                None
            }
            Err(e) => {
                log::warn!("Failed to get latest internal_date: {e}, falling back to full sync");
                None
            }
        }
    } else {
        None
    };

    let query = sync_logic::build_sync_query(&sender_addresses, &None, &after_date);
    let max_results = (config.sync.max_results_per_page.clamp(1, 500)) as u32;

    let all_ids = match fetch_all_message_ids(&gmail_client, &query, max_results, None).await {
        Ok(ids) => ids,
        Err(e) => {
            let msg = format!("Failed to fetch message IDs: {}", e);
            err.report_zero(&msg);
            sync_state.set_error(&e);
            return;
        }
    };

    log::info!("Fetched {} message IDs from Gmail ({mode_label})", all_ids.len());

    let new_ids: Vec<String> = match email_repo.filter_new_message_ids(&all_ids).await {
        Ok(ids) => ids,
        Err(e) => {
            let msg = format!("Failed to filter new message IDs: {}", e);
            err.report_zero(&msg);
            sync_state.set_error(&e);
            return;
        }
    };

    log::info!("Found {} new messages to sync ({mode_label})", new_ids.len());

    if new_ids.is_empty() {
        log::info!("No new messages to sync ({mode_label})");
        let complete_event = BatchProgressEvent::complete(
            GMAIL_SYNC_TASK_NAME,
            0,
            0,
            0,
            "同期対象の新規メッセージがありません".to_string(),
        );
        app.emit_event(GMAIL_SYNC_EVENT_NAME, complete_event);
        app.notify("Gmail同期完了", "新規メッセージはありませんでした");
        return;
    }

    let inputs: Vec<_> = new_ids.into_iter().map(create_sync_input).collect();
    let total_items = inputs.len();

    let task = GmailSyncTask::<
        GmailClientForE2E,
        SqliteEmailRepository,
        SqliteShopSettingsRepository,
    >::new();

    let context = GmailSyncContext {
        gmail_client: Arc::new(gmail_client),
        email_repo: Arc::new(email_repo),
        shop_settings_repo: Arc::new(shop_repo),
        shop_settings_cache: Arc::new(Mutex::new(ShopSettingsCacheForSync::default())),
    };

    let timeout_minutes = config.sync.timeout_minutes.clamp(1, 120);
    let runner = BatchRunner::new(task, batch_size, 0).with_timeout(timeout_minutes as u64);
    let sync_state_for_cancel = sync_state.clone();

    match runner
        .run(app, inputs, &context, || {
            sync_state_for_cancel.should_stop()
        })
        .await
    {
        Ok(batch_result) => {
            log::info!(
                "Gmail sync ({mode_label}) completed: success={}, failed={}",
                batch_result.success_count,
                batch_result.failed_count
            );
            if !sync_state.should_stop() {
                let notification_body = format!(
                    "同期完了：新たに{}件のメールを取り込みました",
                    batch_result.success_count
                );
                app.notify("Gmail同期完了", &notification_body);
            }
        }
        Err(e) => {
            sync_state.set_error(&e);
            err.report(&format!("Sync error: {}", e), total_items, 0, 0, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gmail::{SyncState, GMAIL_SYNC_EVENT_NAME};
    use crate::orchestration::test_helpers::*;
    use std::sync::atomic::Ordering;
    use tempfile::TempDir;

    #[tokio::test]
    async fn run_sync_task_emits_error_when_already_running() {
        let pool = create_pool().await;
        let tmp = TempDir::new().unwrap();
        let app = FakeApp {
            config_dir: tmp.path().to_path_buf(),
            data_dir: Some(tmp.path().to_path_buf()),
            emitted_events: std::sync::Mutex::new(Vec::new()),
            notify_count: std::sync::atomic::AtomicUsize::new(0),
            fail_create_gmail_client: false,
        };
        let sync_state = SyncState::new();
        assert!(sync_state.try_start());

        run_sync_task_with(&app, pool, sync_state).await;

        let emitted = app.emitted_events.lock().unwrap();
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0], GMAIL_SYNC_EVENT_NAME);
        assert_eq!(app.notify_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn run_sync_task_handles_no_new_messages_with_mock_gmail() {
        let pool = create_pool().await;
        create_shop_settings_table(&pool).await;
        insert_enabled_shop(&pool).await;

        let tmp = TempDir::new().unwrap();
        let app = FakeApp {
            config_dir: tmp.path().to_path_buf(),
            data_dir: Some(tmp.path().to_path_buf()),
            emitted_events: std::sync::Mutex::new(Vec::new()),
            notify_count: std::sync::atomic::AtomicUsize::new(0),
            fail_create_gmail_client: false,
        };
        let sync_state = SyncState::new();

        run_sync_task_with(&app, pool, sync_state).await;

        let emitted = app.emitted_events.lock().unwrap();
        assert!(
            !emitted.is_empty(),
            "should emit at least one progress event"
        );
        assert_eq!(app.notify_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn run_sync_task_emits_error_when_gmail_client_factory_fails() {
        let pool = create_pool().await;
        create_shop_settings_table(&pool).await;
        insert_enabled_shop(&pool).await;

        let tmp = TempDir::new().unwrap();
        let app = FakeApp {
            config_dir: tmp.path().to_path_buf(),
            data_dir: Some(tmp.path().to_path_buf()),
            emitted_events: std::sync::Mutex::new(Vec::new()),
            notify_count: std::sync::atomic::AtomicUsize::new(0),
            fail_create_gmail_client: true,
        };
        let sync_state = SyncState::new();

        run_sync_task_with(&app, pool, sync_state).await;

        let emitted = app.emitted_events.lock().unwrap();
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0], GMAIL_SYNC_EVENT_NAME);
        assert_eq!(app.notify_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn run_incremental_sync_task_emits_error_when_already_running() {
        let pool = create_pool().await;
        let tmp = TempDir::new().unwrap();
        let app = FakeApp {
            config_dir: tmp.path().to_path_buf(),
            data_dir: Some(tmp.path().to_path_buf()),
            emitted_events: std::sync::Mutex::new(Vec::new()),
            notify_count: std::sync::atomic::AtomicUsize::new(0),
            fail_create_gmail_client: false,
        };
        let sync_state = SyncState::new();
        assert!(sync_state.try_start());

        run_incremental_sync_task_with(&app, pool, sync_state).await;

        let emitted = app.emitted_events.lock().unwrap();
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0], GMAIL_SYNC_EVENT_NAME);
        assert_eq!(app.notify_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn run_incremental_sync_falls_back_to_full_when_db_empty() {
        let pool = create_pool().await;
        create_shop_settings_table(&pool).await;
        insert_enabled_shop(&pool).await;

        // emails テーブルを作成（空のまま）
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS emails (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id TEXT UNIQUE NOT NULL,
                body_plain TEXT,
                body_html TEXT,
                analysis_status TEXT NOT NULL DEFAULT 'pending',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                internal_date INTEGER,
                from_address TEXT,
                subject TEXT
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let tmp = TempDir::new().unwrap();
        let app = FakeApp {
            config_dir: tmp.path().to_path_buf(),
            data_dir: Some(tmp.path().to_path_buf()),
            emitted_events: std::sync::Mutex::new(Vec::new()),
            notify_count: std::sync::atomic::AtomicUsize::new(0),
            fail_create_gmail_client: false,
        };
        let sync_state = SyncState::new();

        // DB が空なので全件同期にフォールバック → E2Eモック（空結果）→ complete イベント
        run_incremental_sync_task_with(&app, pool, sync_state).await;

        let emitted = app.emitted_events.lock().unwrap();
        assert!(
            !emitted.is_empty(),
            "should emit at least one progress event"
        );
        assert_eq!(app.notify_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn run_incremental_sync_handles_no_new_messages() {
        let pool = create_pool().await;
        create_shop_settings_table(&pool).await;
        insert_enabled_shop(&pool).await;

        // emails テーブルを作成して既存データを挿入
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS emails (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id TEXT UNIQUE NOT NULL,
                body_plain TEXT,
                body_html TEXT,
                analysis_status TEXT NOT NULL DEFAULT 'pending',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                internal_date INTEGER,
                from_address TEXT,
                subject TEXT
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO emails (message_id, internal_date) VALUES ('existing', 1704067200000)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let tmp = TempDir::new().unwrap();
        let app = FakeApp {
            config_dir: tmp.path().to_path_buf(),
            data_dir: Some(tmp.path().to_path_buf()),
            emitted_events: std::sync::Mutex::new(Vec::new()),
            notify_count: std::sync::atomic::AtomicUsize::new(0),
            fail_create_gmail_client: false,
        };
        let sync_state = SyncState::new();

        run_incremental_sync_task_with(&app, pool, sync_state).await;

        let emitted = app.emitted_events.lock().unwrap();
        assert!(
            !emitted.is_empty(),
            "should emit at least one progress event"
        );
        assert_eq!(app.notify_count.load(Ordering::SeqCst), 1);
    }
}
