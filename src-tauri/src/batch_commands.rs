//! バッチ処理の本体ロジック。コマンド・トレイメニュー両方から呼び出される。
//!
//! メインウィンドウを開かずにトレイから直接バッチを実行するため、
//! ロジックを共通関数として抽出している。

use std::sync::Arc;

use sqlx::sqlite::SqlitePool;
use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Mutex;

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
    log::info!("Starting Gmail sync with BatchRunner<GmailSyncTask>...");

    if !sync_state.try_start() {
        log::warn!("Sync is already in progress");
        let message = "Sync is already in progress".to_string();
        let error_event = BatchProgressEvent::error(GMAIL_SYNC_TASK_NAME, 0, 0, 0, 0, message);
        let _ = app.emit(GMAIL_SYNC_EVENT_NAME, error_event);
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
            let _ = app.emit(GMAIL_SYNC_EVENT_NAME, error_event);
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

    let app_config_dir = match app.path().app_config_dir() {
        Ok(dir) => dir,
        Err(e) => {
            let message = format!("Failed to get app config dir: {}", e);
            log::error!("{}", message);
            sync_state.set_error(&message);
            let error_event = BatchProgressEvent::error(GMAIL_SYNC_TASK_NAME, 0, 0, 0, 0, message);
            let _ = app.emit(GMAIL_SYNC_EVENT_NAME, error_event);
            return;
        }
    };
    let config = config::load(&app_config_dir).unwrap_or_else(|e| {
        log::error!("Failed to load config: {}", e);
        config::AppConfig::default()
    });
    let batch_size = clamp_batch_size(config.sync.batch_size, 50);

    let gmail_client = if is_e2e_mock_mode() {
        log::info!("Using E2E mock Gmail client");
        GmailClientForE2E::Mock(E2EMockGmailClient)
    } else {
        match crate::gmail::GmailClient::new(&app).await {
            Ok(c) => GmailClientForE2E::Real(Box::new(c)),
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
                let _ = app.emit(GMAIL_SYNC_EVENT_NAME, error_event);
                return;
            }
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
            let _ = app.emit(GMAIL_SYNC_EVENT_NAME, error_event);
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
            let _ = app.emit(GMAIL_SYNC_EVENT_NAME, error_event);
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
        let _ = app.emit(GMAIL_SYNC_EVENT_NAME, complete_event);
        let _ = app
            .notification()
            .builder()
            .title("Gmail同期完了")
            .body("新規メッセージはありませんでした")
            .show();
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
                let _ = app
                    .notification()
                    .builder()
                    .title("Gmail同期完了")
                    .body(&notification_body)
                    .show();
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
            let _ = app.emit(GMAIL_SYNC_EVENT_NAME, error_event);
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
            let _ = app.emit(EMAIL_PARSE_EVENT_NAME, error_event);
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
            let _ = app.emit(EMAIL_PARSE_EVENT_NAME, error_event);
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
        let _ = app.emit(EMAIL_PARSE_EVENT_NAME, error_event);
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
            let _ = app.emit(EMAIL_PARSE_EVENT_NAME, error_event);
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
        let _ = app.emit(EMAIL_PARSE_EVENT_NAME, error_event);
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
            let _ = app.emit(EMAIL_PARSE_EVENT_NAME, error_event);
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
        let _ = app.emit(EMAIL_PARSE_EVENT_NAME, complete_event);
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
            let _ = app.emit(EMAIL_PARSE_EVENT_NAME, error_event);
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
        .path()
        .app_data_dir()
        .ok()
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
    log::info!("Starting product name parse with BatchRunner<ProductNameParseTask>...");

    let app_data_dir = match app.path().app_data_dir() {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to get app data dir: {}", e);
            let error_event = BatchProgressEvent::error(
                PRODUCT_NAME_PARSE_TASK_NAME,
                0,
                0,
                0,
                0,
                format!("Failed to get app data dir: {}", e),
            );
            let _ = app.emit(PRODUCT_NAME_PARSE_EVENT_NAME, error_event);
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
            let _ = app.emit(PRODUCT_NAME_PARSE_EVENT_NAME, error_event);
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
                    let _ = app.emit(PRODUCT_NAME_PARSE_EVENT_NAME, error_event);
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
                let _ = app.emit(PRODUCT_NAME_PARSE_EVENT_NAME, error_event);
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
            let _ = app.emit(PRODUCT_NAME_PARSE_EVENT_NAME, error_event);
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
            let _ = app.emit(PRODUCT_NAME_PARSE_EVENT_NAME, error_event);
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
        let _ = app.emit(PRODUCT_NAME_PARSE_EVENT_NAME, complete_event);
        parse_state.finish();
        return;
    }

    let inputs: Vec<_> = items
        .into_iter()
        .map(|(raw_name, platform_hint)| create_product_parse_input(raw_name, platform_hint))
        .collect();

    let config = app
        .path()
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
            let _ = app.emit(PRODUCT_NAME_PARSE_EVENT_NAME, error_event);
        }
    }

    parse_state.finish();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_batch_size() {
        // clamp_batch_size は本番コード（tray_parse, start_batch_parse）で使用
        assert_eq!(clamp_batch_size(0, 100), 100);
        assert_eq!(clamp_batch_size(-1, 100), 100);
        assert_eq!(clamp_batch_size(50, 100), 50);
        assert_eq!(clamp_batch_size(200, 100), 200);
        assert_eq!(clamp_batch_size(i64::MIN, 100), 100);
    }
}
