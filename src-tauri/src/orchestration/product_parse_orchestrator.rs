//! 商品名解析オーケストレーション。

use std::sync::Arc;

use sqlx::sqlite::SqlitePool;
use tokio::sync::Mutex;

use super::error_handler::ErrorReporter;
use super::{BatchCommandsApp, TauriBatchCommandsApp};
use crate::batch_runner::{BatchProgressEvent, BatchRunner};
use crate::config;
use crate::e2e_mocks::{is_e2e_mock_mode, GeminiClientForE2E};
use crate::gemini::{
    create_product_parse_input, GeminiClient, ProductNameParseCache, ProductNameParseContext,
    ProductNameParseTask, PRODUCT_NAME_PARSE_EVENT_NAME, PRODUCT_NAME_PARSE_TASK_NAME,
};
use crate::repository::SqliteProductMasterRepository;

/// 商品名パースタスクの本体。コマンド・トレイ両方から呼ぶ。
///
/// `caller_did_try_start`: 呼び出し元で既に try_start 済みなら true（コマンド経由）。
/// false の場合は本関数内で try_start を行う（トレイ経由）。
pub async fn run_product_name_parse_task(
    app: tauri::AppHandle,
    pool: SqlitePool,
    parse_state: crate::commands::ProductNameParseState,
    caller_did_try_start: bool,
) {
    let app = TauriBatchCommandsApp { app };
    run_product_name_parse_task_with(&app, pool, parse_state, caller_did_try_start).await
}

async fn run_product_name_parse_task_with<A: BatchCommandsApp>(
    app: &A,
    pool: SqlitePool,
    parse_state: crate::commands::ProductNameParseState,
    caller_did_try_start: bool,
) {
    log::info!("Starting product name parse with BatchRunner<ProductNameParseTask>...");

    let err = ErrorReporter::new(
        app,
        PRODUCT_NAME_PARSE_TASK_NAME,
        PRODUCT_NAME_PARSE_EVENT_NAME,
    );

    let app_data_dir = match app.app_data_dir() {
        Ok(p) => p,
        Err(e) => {
            err.report_zero(&e);
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
            err.report_zero(
                "Gemini APIキーが設定されていません。設定画面でAPIキーを設定してください。",
            );
            if caller_did_try_start {
                parse_state.finish();
            }
            return;
        }
        match crate::gemini::load_api_key(&app_data_dir) {
            Ok(api_key) => match GeminiClient::new(api_key) {
                Ok(client) => GeminiClientForE2E::Real(Box::new(client)),
                Err(e) => {
                    err.report_zero(&format!("Failed to create Gemini client: {}", e));
                    if caller_did_try_start {
                        parse_state.finish();
                    }
                    return;
                }
            },
            Err(e) => {
                err.report_zero(&format!("Failed to load API key: {}", e));
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
            err.report_zero(&e);
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
            err.report_zero(&format!("商品情報の取得に失敗: {}", e));
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

    match runner.run(app, inputs, &context, || false).await {
        Ok(batch_result) => {
            log::info!(
                "Product name parse completed: success={}, failed={}",
                batch_result.success_count,
                batch_result.failed_count
            );
        }
        Err(e) => {
            err.report(
                &format!("バッチ処理エラー: {}", e),
                total_items,
                0,
                0,
                total_items,
            );
        }
    }

    parse_state.finish();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gemini::PRODUCT_NAME_PARSE_EVENT_NAME;
    use crate::orchestration::test_helpers::*;

    #[tokio::test]
    async fn run_product_name_parse_task_emits_error_when_app_data_dir_missing_and_finishes() {
        let pool = create_pool().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let app = FakeApp {
            config_dir: tmp.path().to_path_buf(),
            data_dir: None,
            emitted_events: std::sync::Mutex::new(Vec::new()),
            notify_count: std::sync::atomic::AtomicUsize::new(0),
            fail_create_gmail_client: false,
        };
        let parse_state = crate::commands::ProductNameParseState::new();
        parse_state.try_start().unwrap();

        run_product_name_parse_task_with(&app, pool, parse_state.clone(), true).await;

        // caller_did_try_start=true のため finish されている → 再度 try_start できる
        assert!(parse_state.try_start().is_ok());

        let emitted = app.emitted_events.lock().unwrap();
        assert!(!emitted.is_empty());
        assert_eq!(emitted[0], PRODUCT_NAME_PARSE_EVENT_NAME);
    }
}
