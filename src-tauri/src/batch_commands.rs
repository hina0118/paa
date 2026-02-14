//! バッチ処理の本体ロジック。コマンド・トレイメニュー両方から呼び出される。
//!
//! メインウィンドウを開かずにトレイから直接バッチを実行するため、
//! ロジックを共通関数として抽出している。

use std::sync::Arc;

use sqlx::sqlite::SqlitePool;
use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Mutex;

use crate::batch_runner::BatchEventEmitter;
use crate::batch_runner::{BatchProgressEvent, BatchRunner};
use crate::config;
use crate::e2e_mocks::{
    is_e2e_mock_mode, E2EMockGmailClient, GeminiClientForE2E, GmailClientForE2E,
};
use crate::gemini::{
    create_product_parse_input, GeminiClient, ProductNameParseCache, ProductNameParseContext,
    ProductNameParseTask, PRODUCT_NAME_PARSE_EVENT_NAME, PRODUCT_NAME_PARSE_TASK_NAME,
};
use crate::gmail::{
    create_sync_input, fetch_all_message_ids, GmailSyncContext, GmailSyncTask,
    ShopSettingsCacheForSync, SyncGuard, SyncState, GMAIL_SYNC_EVENT_NAME, GMAIL_SYNC_TASK_NAME,
};
use crate::logic::sync_logic;
use crate::parsers::EmailRow;
use crate::parsers::{
    EmailParseContext, EmailParseTask, ShopSettingsCache, EMAIL_PARSE_EVENT_NAME,
    EMAIL_PARSE_TASK_NAME,
};
use crate::repository::{
    EmailRepository, ParseRepository, ShopSettingsRepository, SqliteEmailRepository,
    SqliteOrderRepository, SqliteParseRepository, SqliteProductMasterRepository,
    SqliteShopSettingsRepository,
};

#[async_trait::async_trait]
trait BatchCommandsApp: BatchEventEmitter {
    fn notify(&self, title: &str, body: &str);
    fn app_config_dir(&self) -> Result<std::path::PathBuf, String>;
    fn app_data_dir(&self) -> Option<std::path::PathBuf>;
    async fn create_gmail_client(&self) -> Result<GmailClientForE2E, String>;
}

struct TauriBatchCommandsApp {
    app: tauri::AppHandle,
}

impl BatchEventEmitter for TauriBatchCommandsApp {
    fn emit_event<S: serde::Serialize + Clone>(&self, event: &str, payload: S) {
        let _ = self.app.emit(event, payload);
    }
}

#[async_trait::async_trait]
impl BatchCommandsApp for TauriBatchCommandsApp {
    fn notify(&self, title: &str, body: &str) {
        let _ = self
            .app
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show();
    }

    fn app_config_dir(&self) -> Result<std::path::PathBuf, String> {
        self.app
            .path()
            .app_config_dir()
            .map_err(|e| format!("Failed to get app config dir: {e}"))
    }

    fn app_data_dir(&self) -> Option<std::path::PathBuf> {
        self.app.path().app_data_dir().ok()
    }

    async fn create_gmail_client(&self) -> Result<GmailClientForE2E, String> {
        if is_e2e_mock_mode() {
            log::info!("Using E2E mock Gmail client");
            return Ok(GmailClientForE2E::Mock(E2EMockGmailClient));
        }
        crate::gmail::GmailClient::new(&self.app)
            .await
            .map(|c| GmailClientForE2E::Real(Box::new(c)))
    }
}

/// config.parse.batch_size (i64) を usize へ安全に変換。
/// 0 以下は default にフォールバック。変換失敗時（32-bit で i64 が大きい等）も default。
/// 上限はクランプしない（大きい i64 は usize::try_from で失敗→default）。
pub(crate) fn clamp_batch_size(v: i64, default: usize) -> usize {
    if v <= 0 {
        default
    } else {
        usize::try_from(v).unwrap_or(default)
    }
}

/// Gmail同期タスクの本体。コマンド・トレイ両方から呼ぶ。
pub async fn run_sync_task(app: tauri::AppHandle, pool: SqlitePool, sync_state: SyncState) {
    let app = TauriBatchCommandsApp { app };
    run_sync_task_with(&app, pool, sync_state).await
}

async fn run_sync_task_with<A: BatchCommandsApp>(app: &A, pool: SqlitePool, sync_state: SyncState) {
    log::info!("Starting Gmail sync with BatchRunner<GmailSyncTask>...");

    if !sync_state.try_start() {
        log::warn!("Sync is already in progress");
        let message = "Sync is already in progress".to_string();
        let error_event = BatchProgressEvent::error(GMAIL_SYNC_TASK_NAME, 0, 0, 0, 0, message);
        app.emit_event(GMAIL_SYNC_EVENT_NAME, error_event);
        return;
    }

    let _guard = SyncGuard::new(&sync_state);

    let email_repo = SqliteEmailRepository::new(pool.clone());
    let shop_repo = SqliteShopSettingsRepository::new(pool.clone());

    let enabled_shops = match shop_repo.get_enabled().await {
        Ok(shops) => shops,
        Err(e) => {
            log::error!("Failed to fetch shop settings: {}", e);
            sync_state.set_error(&e);
            let error_event = BatchProgressEvent::error(
                GMAIL_SYNC_TASK_NAME,
                0,
                0,
                0,
                0,
                format!("Failed to fetch shop settings: {}", e),
            );
            app.emit_event(GMAIL_SYNC_EVENT_NAME, error_event);
            return;
        }
    };

    let sender_addresses: Vec<String> = enabled_shops
        .iter()
        .map(|s| s.sender_address.clone())
        .collect();

    log::info!(
        "Starting sync with {} enabled sender addresses",
        sender_addresses.len()
    );

    let app_config_dir = match app.app_config_dir() {
        Ok(dir) => dir,
        Err(message) => {
            log::error!("{}", message);
            sync_state.set_error(&message);
            let error_event = BatchProgressEvent::error(GMAIL_SYNC_TASK_NAME, 0, 0, 0, 0, message);
            app.emit_event(GMAIL_SYNC_EVENT_NAME, error_event);
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
            log::error!("Failed to create Gmail client: {}", e);
            sync_state.set_error(&e);
            let error_event = BatchProgressEvent::error(
                GMAIL_SYNC_TASK_NAME,
                0,
                0,
                0,
                0,
                format!("Failed to create Gmail client: {}", e),
            );
            app.emit_event(GMAIL_SYNC_EVENT_NAME, error_event);
            return;
        }
    };

    let query = sync_logic::build_sync_query(&sender_addresses, &None);
    let max_results = (config.sync.max_results_per_page.clamp(1, 500)) as u32;

    let all_ids = match fetch_all_message_ids(&gmail_client, &query, max_results, None).await {
        Ok(ids) => ids,
        Err(e) => {
            log::error!("Failed to fetch message IDs: {}", e);
            sync_state.set_error(&e);
            let error_event = BatchProgressEvent::error(
                GMAIL_SYNC_TASK_NAME,
                0,
                0,
                0,
                0,
                format!("Failed to fetch message IDs: {}", e),
            );
            app.emit_event(GMAIL_SYNC_EVENT_NAME, error_event);
            return;
        }
    };

    log::info!("Fetched {} message IDs from Gmail", all_ids.len());

    let new_ids: Vec<String> = match email_repo.filter_new_message_ids(&all_ids).await {
        Ok(ids) => ids,
        Err(e) => {
            log::error!("Failed to filter new message IDs: {}", e);
            sync_state.set_error(&e);
            let error_event = BatchProgressEvent::error(
                GMAIL_SYNC_TASK_NAME,
                0,
                0,
                0,
                0,
                format!("Failed to filter new message IDs: {}", e),
            );
            app.emit_event(GMAIL_SYNC_EVENT_NAME, error_event);
            return;
        }
    };

    log::info!("Found {} new messages to sync", new_ids.len());

    if new_ids.is_empty() {
        log::info!("No new messages to sync");
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
        .run(&app, inputs, &context, || {
            sync_state_for_cancel.should_stop()
        })
        .await
    {
        Ok(batch_result) => {
            log::info!(
                "Gmail sync completed: success={}, failed={}",
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
            log::error!("BatchRunner failed: {}", e);
            sync_state.set_error(&e);
            let error_event = BatchProgressEvent::error(
                GMAIL_SYNC_TASK_NAME,
                total_items,
                0,
                0,
                0,
                format!("Sync error: {}", e),
            );
            app.emit_event(GMAIL_SYNC_EVENT_NAME, error_event);
        }
    }
}

/// メールパースタスクの本体。コマンド・トレイ両方から呼ぶ。
pub async fn run_batch_parse_task(
    app: tauri::AppHandle,
    pool: SqlitePool,
    parse_state: crate::parsers::ParseState,
    batch_size: usize,
) {
    let app = TauriBatchCommandsApp { app };
    run_batch_parse_task_with(&app, pool, parse_state, batch_size).await
}

async fn run_batch_parse_task_with<A: BatchCommandsApp>(
    app: &A,
    pool: SqlitePool,
    parse_state: crate::parsers::ParseState,
    batch_size: usize,
) {
    log::info!("Starting batch parse with BatchRunner<EmailParseTask>...");

    let batch_size = batch_size.max(1);

    if let Err(e) = parse_state.start() {
        let msg = e.to_string();
        if msg.contains("Parse is already running") {
            log::warn!("Parse already running, skip starting new parse: {}", msg);
            let error_event = BatchProgressEvent::error(
                EMAIL_PARSE_TASK_NAME,
                0,
                0,
                0,
                0,
                format!("Parse already running: {}", msg),
            );
            app.emit_event(EMAIL_PARSE_EVENT_NAME, error_event);
        } else {
            log::error!("Failed to start parse: {}", msg);
            parse_state.set_error(&e);
            let error_event = BatchProgressEvent::error(
                EMAIL_PARSE_TASK_NAME,
                0,
                0,
                0,
                0,
                format!("Parse error: {}", msg),
            );
            app.emit_event(EMAIL_PARSE_EVENT_NAME, error_event);
        }
        return;
    }

    let parse_repo = SqliteParseRepository::new(pool.clone());
    let order_repo = SqliteOrderRepository::new(pool.clone());
    let shop_settings_repo = SqliteShopSettingsRepository::new(pool.clone());

    log::info!("Clearing order_emails, deliveries, items, and orders tables for fresh parse...");
    if let Err(e) = parse_repo.clear_order_tables().await {
        log::error!("Failed to clear order tables: {}", e);
        parse_state.finish();
        parse_state.set_error(&e);
        let error_event = BatchProgressEvent::error(
            EMAIL_PARSE_TASK_NAME,
            0,
            0,
            0,
            0,
            format!("Failed to clear order tables: {}", e),
        );
        app.emit_event(EMAIL_PARSE_EVENT_NAME, error_event);
        return;
    }

    let enabled_settings = match shop_settings_repo.get_enabled().await {
        Ok(settings) => settings,
        Err(e) => {
            log::error!("Failed to fetch shop settings: {}", e);
            parse_state.finish();
            parse_state.set_error(&e);
            let error_event = BatchProgressEvent::error(
                EMAIL_PARSE_TASK_NAME,
                0,
                0,
                0,
                0,
                format!("Failed to fetch shop settings: {}", e),
            );
            app.emit_event(EMAIL_PARSE_EVENT_NAME, error_event);
            return;
        }
    };

    let parser_types: Vec<_> = enabled_settings
        .iter()
        .map(|s| s.parser_type.as_str())
        .collect();
    log::info!("[parse] shop_settings parsers: {:?}", parser_types);

    if enabled_settings.is_empty() {
        log::warn!("No enabled shop settings found");
        parse_state.finish();
        parse_state.set_error("No enabled shop settings found");
        let error_event = BatchProgressEvent::error(
            EMAIL_PARSE_TASK_NAME,
            0,
            0,
            0,
            0,
            "No enabled shop settings found".to_string(),
        );
        app.emit_event(EMAIL_PARSE_EVENT_NAME, error_event);
        return;
    }

    let total_email_count = match parse_repo.get_total_email_count().await {
        Ok(count) => count as usize,
        Err(e) => {
            log::error!("Failed to count emails: {}", e);
            parse_state.finish();
            parse_state.set_error(&e);
            let error_event = BatchProgressEvent::error(
                EMAIL_PARSE_TASK_NAME,
                0,
                0,
                0,
                0,
                format!("Failed to count emails: {}", e),
            );
            app.emit_event(EMAIL_PARSE_EVENT_NAME, error_event);
            return;
        }
    };

    log::info!("Total emails to parse: {}", total_email_count);

    if total_email_count == 0 {
        log::info!("No emails to parse");
        parse_state.finish();
        let complete_event = BatchProgressEvent::complete(
            EMAIL_PARSE_TASK_NAME,
            0,
            0,
            0,
            "パース対象のメールがありません".to_string(),
        );
        app.emit_event(EMAIL_PARSE_EVENT_NAME, complete_event);
        return;
    }

    let all_unparsed_emails = match parse_repo.get_unparsed_emails(total_email_count).await {
        Ok(emails) => emails,
        Err(e) => {
            log::error!("Failed to fetch unparsed emails: {}", e);
            parse_state.finish();
            parse_state.set_error(&e);
            let error_event = BatchProgressEvent::error(
                EMAIL_PARSE_TASK_NAME,
                total_email_count,
                0,
                0,
                0,
                format!("Failed to fetch unparsed emails: {}", e),
            );
            app.emit_event(EMAIL_PARSE_EVENT_NAME, error_event);
            return;
        }
    };

    let inputs: Vec<_> = all_unparsed_emails
        .into_iter()
        .map(|row: EmailRow| row.into())
        .collect();
    let inputs_len = inputs.len();
    log::info!("Fetched {} unparsed emails", inputs_len);
    if !inputs.is_empty() {
        let first: &crate::parsers::EmailParseInput = &inputs[0];
        let last = inputs.last().unwrap();
        log::debug!(
            "[batch] first email_id={} internal_date={:?} subject={:?}",
            first.email_id,
            first.internal_date,
            first.subject
        );
        if inputs_len > 1 {
            log::debug!(
                "[batch] last email_id={} internal_date={:?} subject={:?}",
                last.email_id,
                last.internal_date,
                last.subject
            );
        }
    }

    let task: EmailParseTask<
        SqliteOrderRepository,
        SqliteParseRepository,
        SqliteShopSettingsRepository,
    > = EmailParseTask::new();

    let image_save_ctx = app
        .app_data_dir()
        .map(|dir| (std::sync::Arc::new(pool.clone()), dir.join("images")));

    let context = EmailParseContext {
        order_repo: Arc::new(order_repo),
        parse_repo: Arc::new(parse_repo),
        shop_settings_repo: Arc::new(shop_settings_repo),
        shop_settings_cache: Arc::new(Mutex::new(ShopSettingsCache::default())),
        parse_state: Arc::new(parse_state.clone()),
        image_save_ctx,
    };

    let runner = BatchRunner::new(task, batch_size, 0);
    let parse_state_for_cancel = parse_state.clone();

    match runner
        .run(&app, inputs, &context, || {
            parse_state_for_cancel.is_cancelled()
        })
        .await
    {
        Ok(_batch_result) => {
            log::info!(
                "Email parse completed: success={}, failed={}",
                _batch_result.success_count,
                _batch_result.failed_count
            );

            // 補正(override)・除外(exclusion)は表示クエリ側の COALESCE / LEFT JOIN で対応。
            // テーブルへの UPDATE は行わない。
        }
        Err(e) => {
            log::error!("BatchRunner failed: {}", e);
            parse_state.set_error(&e);
        }
    }

    parse_state.finish();
}

/// 商品名パースタスクの本体。コマンド・トレイ両方から呼ぶ。
///
/// `caller_did_try_start`: 呼び出し元で既に try_start 済みなら true（コマンド経由）。
/// false の場合は本関数内で try_start を行う（トレイ経由）。
pub async fn run_product_name_parse_task(
    app: tauri::AppHandle,
    pool: SqlitePool,
    parse_state: crate::ProductNameParseState,
    caller_did_try_start: bool,
) {
    let app = TauriBatchCommandsApp { app };
    run_product_name_parse_task_with(&app, pool, parse_state, caller_did_try_start).await
}

async fn run_product_name_parse_task_with<A: BatchCommandsApp>(
    app: &A,
    pool: SqlitePool,
    parse_state: crate::ProductNameParseState,
    caller_did_try_start: bool,
) {
    log::info!("Starting product name parse with BatchRunner<ProductNameParseTask>...");

    let app_data_dir = match app.app_data_dir() {
        Some(p) => p,
        None => {
            let msg = "Failed to get app data dir".to_string();
            log::error!("{}", msg);
            let error_event =
                BatchProgressEvent::error(PRODUCT_NAME_PARSE_TASK_NAME, 0, 0, 0, 0, msg);
            app.emit_event(PRODUCT_NAME_PARSE_EVENT_NAME, error_event);
            if caller_did_try_start {
                parse_state.finish();
            }
            return;
        }
    };

    let gemini_client = if is_e2e_mock_mode() {
        log::info!("Using E2E mock Gemini client");
        GeminiClientForE2E::Mock(crate::e2e_mocks::E2EMockGeminiClient)
    } else {
        if !crate::gemini::has_api_key(&app_data_dir) {
            let error_event = BatchProgressEvent::error(
                PRODUCT_NAME_PARSE_TASK_NAME,
                0,
                0,
                0,
                0,
                "Gemini APIキーが設定されていません。設定画面でAPIキーを設定してください。"
                    .to_string(),
            );
            app.emit_event(PRODUCT_NAME_PARSE_EVENT_NAME, error_event);
            if caller_did_try_start {
                parse_state.finish();
            }
            return;
        }
        match crate::gemini::load_api_key(&app_data_dir) {
            Ok(api_key) => match GeminiClient::new(api_key) {
                Ok(client) => GeminiClientForE2E::Real(Box::new(client)),
                Err(e) => {
                    log::error!("Failed to create Gemini client: {}", e);
                    let error_event = BatchProgressEvent::error(
                        PRODUCT_NAME_PARSE_TASK_NAME,
                        0,
                        0,
                        0,
                        0,
                        format!("Failed to create Gemini client: {}", e),
                    );
                    app.emit_event(PRODUCT_NAME_PARSE_EVENT_NAME, error_event);
                    if caller_did_try_start {
                        parse_state.finish();
                    }
                    return;
                }
            },
            Err(e) => {
                log::error!("Failed to load Gemini API key: {}", e);
                let error_event = BatchProgressEvent::error(
                    PRODUCT_NAME_PARSE_TASK_NAME,
                    0,
                    0,
                    0,
                    0,
                    format!("Failed to load API key: {}", e),
                );
                app.emit_event(PRODUCT_NAME_PARSE_EVENT_NAME, error_event);
                if caller_did_try_start {
                    parse_state.finish();
                }
                return;
            }
        }
    };

    if !caller_did_try_start {
        if let Err(e) = parse_state.try_start() {
            log::error!("Product name parse already running: {}", e);
            let error_event =
                BatchProgressEvent::error(PRODUCT_NAME_PARSE_TASK_NAME, 0, 0, 0, 0, e);
            app.emit_event(PRODUCT_NAME_PARSE_EVENT_NAME, error_event);
            return;
        }
    }

    let product_repo = SqliteProductMasterRepository::new(pool.clone());

    let items: Vec<(String, Option<String>)> = match sqlx::query_as(
        r#"
        SELECT
          TRIM(i.item_name) AS item_name,
          MIN(o.shop_domain) AS shop_domain
        FROM items i
        JOIN orders o ON i.order_id = o.id
        LEFT JOIN product_master pm ON TRIM(i.item_name) = pm.raw_name
        WHERE i.item_name IS NOT NULL
          AND i.item_name != ''
          AND TRIM(i.item_name) != ''
          AND pm.id IS NULL
        GROUP BY TRIM(i.item_name)
        "#,
    )
    .fetch_all(&pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            log::error!("Failed to fetch unparsed items: {}", e);
            let error_event = BatchProgressEvent::error(
                PRODUCT_NAME_PARSE_TASK_NAME,
                0,
                0,
                0,
                0,
                format!("商品情報の取得に失敗: {}", e),
            );
            app.emit_event(PRODUCT_NAME_PARSE_EVENT_NAME, error_event);
            parse_state.finish();
            return;
        }
    };

    let total_items = items.len();
    log::info!(
        "Found {} unparsed items (not in product_master)",
        total_items
    );

    if total_items == 0 {
        let complete_event = BatchProgressEvent::complete(
            PRODUCT_NAME_PARSE_TASK_NAME,
            0,
            0,
            0,
            "未解析の商品はありません（すべてproduct_masterに登録済み）".to_string(),
        );
        app.emit_event(PRODUCT_NAME_PARSE_EVENT_NAME, complete_event);
        parse_state.finish();
        return;
    }

    let inputs: Vec<_> = items
        .into_iter()
        .map(|(raw_name, platform_hint)| create_product_parse_input(raw_name, platform_hint))
        .collect();

    let config = app
        .app_config_dir()
        .ok()
        .and_then(|dir| config::load(&dir).ok())
        .unwrap_or_else(|| {
            log::warn!("Failed to load config, using Gemini defaults");
            config::AppConfig::default()
        });
    let gemini_batch_size = (config.gemini.batch_size.clamp(1, 50)) as usize;
    let gemini_delay_ms = (config.gemini.delay_seconds.clamp(0, 60)) as u64 * 1000;

    let task: ProductNameParseTask<GeminiClientForE2E, SqliteProductMasterRepository> =
        ProductNameParseTask::new();
    let context = ProductNameParseContext {
        gemini_client: Arc::new(gemini_client),
        repository: Arc::new(product_repo),
        cache: Arc::new(Mutex::new(ProductNameParseCache::default())),
    };

    let runner = BatchRunner::new(task, gemini_batch_size, gemini_delay_ms);

    match runner.run(&app, inputs, &context, || false).await {
        Ok(batch_result) => {
            log::info!(
                "Product name parse completed: success={}, failed={}",
                batch_result.success_count,
                batch_result.failed_count
            );
        }
        Err(e) => {
            log::error!("BatchRunner failed: {}", e);
            let error_event = BatchProgressEvent::error(
                PRODUCT_NAME_PARSE_TASK_NAME,
                total_items,
                0,
                0,
                total_items,
                format!("バッチ処理エラー: {}", e),
            );
            app.emit_event(PRODUCT_NAME_PARSE_EVENT_NAME, error_event);
        }
    }

    parse_state.finish();
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex as StdMutex;
    use tempfile::TempDir;

    #[test]
    fn test_clamp_batch_size() {
        // clamp_batch_size は本番コード（tray_parse, start_batch_parse）で使用
        assert_eq!(clamp_batch_size(0, 100), 100);
        assert_eq!(clamp_batch_size(-1, 100), 100);
        assert_eq!(clamp_batch_size(50, 100), 50);
        assert_eq!(clamp_batch_size(200, 100), 200);
        assert_eq!(clamp_batch_size(i64::MIN, 100), 100);
    }

    struct FakeApp {
        config_dir: std::path::PathBuf,
        data_dir: Option<std::path::PathBuf>,
        emitted_events: StdMutex<Vec<String>>,
        notify_count: AtomicUsize,
        fail_create_gmail_client: bool,
    }

    impl BatchEventEmitter for FakeApp {
        fn emit_event<S: serde::Serialize + Clone>(&self, event: &str, _payload: S) {
            self.emitted_events.lock().unwrap().push(event.to_string());
        }
    }

    #[async_trait::async_trait]
    impl BatchCommandsApp for FakeApp {
        fn notify(&self, _title: &str, _body: &str) {
            self.notify_count.fetch_add(1, Ordering::SeqCst);
        }

        fn app_config_dir(&self) -> Result<std::path::PathBuf, String> {
            Ok(self.config_dir.clone())
        }

        fn app_data_dir(&self) -> Option<std::path::PathBuf> {
            self.data_dir.clone()
        }

        async fn create_gmail_client(&self) -> Result<GmailClientForE2E, String> {
            if self.fail_create_gmail_client {
                return Err("boom".to_string());
            }
            // 常に E2E モック相当を返す（ネットワークや認証に依存しない）
            Ok(GmailClientForE2E::Mock(E2EMockGmailClient))
        }
    }

    async fn create_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap()
    }

    async fn create_shop_settings_table(pool: &SqlitePool) {
        sqlx::query(
            r#"
            CREATE TABLE shop_settings (
              id INTEGER PRIMARY KEY,
              shop_name TEXT NOT NULL,
              sender_address TEXT NOT NULL,
              parser_type TEXT NOT NULL,
              is_enabled INTEGER NOT NULL,
              subject_filters TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await
        .unwrap();
    }

    async fn insert_enabled_shop(pool: &SqlitePool) {
        sqlx::query(
            r#"
            INSERT INTO shop_settings (id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at)
            VALUES (1, 'TestShop', 'shop@example.com', 'hobbysearch_confirm', 1, NULL, '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')
            "#,
        )
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn run_sync_task_emits_error_when_already_running() {
        let pool = create_pool().await;
        let tmp = TempDir::new().unwrap();
        let app = FakeApp {
            config_dir: tmp.path().to_path_buf(),
            data_dir: Some(tmp.path().to_path_buf()),
            emitted_events: StdMutex::new(Vec::new()),
            notify_count: AtomicUsize::new(0),
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
            emitted_events: StdMutex::new(Vec::new()),
            notify_count: AtomicUsize::new(0),
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
            emitted_events: StdMutex::new(Vec::new()),
            notify_count: AtomicUsize::new(0),
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
    async fn run_batch_parse_task_emits_error_when_already_running() {
        let pool = create_pool().await;
        let tmp = TempDir::new().unwrap();
        let app = FakeApp {
            config_dir: tmp.path().to_path_buf(),
            data_dir: Some(tmp.path().to_path_buf()),
            emitted_events: StdMutex::new(Vec::new()),
            notify_count: AtomicUsize::new(0),
            fail_create_gmail_client: false,
        };
        let parse_state = crate::parsers::ParseState::new();
        parse_state.start().unwrap();

        run_batch_parse_task_with(&app, pool, parse_state, 10).await;

        let emitted = app.emitted_events.lock().unwrap();
        assert!(!emitted.is_empty());
        assert_eq!(emitted[0], EMAIL_PARSE_EVENT_NAME);
    }

    #[tokio::test]
    async fn run_batch_parse_task_finishes_and_emits_error_when_clear_tables_fails() {
        let pool = create_pool().await;
        // order tables を作らない → clear_order_tables が失敗する
        create_shop_settings_table(&pool).await;
        insert_enabled_shop(&pool).await;

        let tmp = TempDir::new().unwrap();
        let app = FakeApp {
            config_dir: tmp.path().to_path_buf(),
            data_dir: Some(tmp.path().to_path_buf()),
            emitted_events: StdMutex::new(Vec::new()),
            notify_count: AtomicUsize::new(0),
            fail_create_gmail_client: false,
        };
        let parse_state = crate::parsers::ParseState::new();

        run_batch_parse_task_with(&app, pool, parse_state.clone(), 10).await;

        // finish されて idle に戻る
        assert!(!*parse_state.is_running.lock().unwrap());

        let emitted = app.emitted_events.lock().unwrap();
        assert!(!emitted.is_empty());
        assert_eq!(emitted[0], EMAIL_PARSE_EVENT_NAME);
    }

    #[tokio::test]
    async fn run_product_name_parse_task_emits_error_when_app_data_dir_missing_and_finishes() {
        let pool = create_pool().await;
        let tmp = TempDir::new().unwrap();
        let app = FakeApp {
            config_dir: tmp.path().to_path_buf(),
            data_dir: None,
            emitted_events: StdMutex::new(Vec::new()),
            notify_count: AtomicUsize::new(0),
            fail_create_gmail_client: false,
        };
        let parse_state = crate::ProductNameParseState::new();
        parse_state.try_start().unwrap();

        run_product_name_parse_task_with(&app, pool, parse_state.clone(), true).await;

        // caller_did_try_start=true のため finish されている → 再度 try_start できる
        assert!(parse_state.try_start().is_ok());

        let emitted = app.emitted_events.lock().unwrap();
        assert!(!emitted.is_empty());
        assert_eq!(emitted[0], PRODUCT_NAME_PARSE_EVENT_NAME);
    }
}
