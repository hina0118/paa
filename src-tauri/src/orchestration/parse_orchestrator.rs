//! メール解析オーケストレーション。

use std::sync::Arc;

use sqlx::sqlite::SqlitePool;
use tokio::sync::Mutex;

use super::error_handler::ErrorReporter;
use super::{BatchCommandsApp, TauriBatchCommandsApp};
use crate::batch_runner::{BatchProgressEvent, BatchRunner, BatchTask};
use crate::parsers::EmailRow;
use crate::parsers::{
    EmailParseContext, EmailParseTask, HtmlParseContext, HtmlParseInput, HtmlParseTask,
    ShopSettingsCache, SurugayaHtmlParseTask, EMAIL_PARSE_EVENT_NAME, EMAIL_PARSE_TASK_NAME,
};
use crate::repository::{
    ParseRepository, ShopSettingsRepository, SqliteParseRepository, SqliteShopSettingsRepository,
};

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

    let err = ErrorReporter::new(app, EMAIL_PARSE_TASK_NAME, EMAIL_PARSE_EVENT_NAME);
    let batch_size = batch_size.max(1);

    if let Err(e) = parse_state.try_start() {
        // try_start の Err は常に「既に実行中」を意味する
        log::warn!("Parse already running, skip starting new parse: {}", e);
        err.report_zero(&format!("Parse already running: {}", e));
        return;
    }

    let parse_repo = SqliteParseRepository::new(pool.clone());
    let shop_settings_repo = SqliteShopSettingsRepository::new(pool.clone());

    log::info!("Clearing order_emails, deliveries, items, and orders tables for fresh parse...");
    if let Err(e) = parse_repo.clear_order_tables().await {
        let msg = format!("Failed to clear order tables: {}", e);
        err.report_zero(&msg);
        parse_state.finish();
        parse_state.set_error(&e);
        return;
    }

    let enabled_settings = match shop_settings_repo.get_enabled().await {
        Ok(settings) => settings,
        Err(e) => {
            let msg = format!("Failed to fetch shop settings: {}", e);
            err.report_zero(&msg);
            parse_state.finish();
            parse_state.set_error(&e);
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
        err.report_zero("No enabled shop settings found");
        parse_state.finish();
        parse_state.set_error("No enabled shop settings found");
        return;
    }

    let total_email_count = match parse_repo.get_total_email_count().await {
        Ok(count) => count as usize,
        Err(e) => {
            let msg = format!("Failed to count emails: {}", e);
            err.report_zero(&msg);
            parse_state.finish();
            parse_state.set_error(&e);
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
            let msg = format!("Failed to fetch unparsed emails: {}", e);
            err.report(&msg, total_email_count, 0, 0, 0);
            parse_state.finish();
            parse_state.set_error(&e);
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

    let task: EmailParseTask<SqliteParseRepository, SqliteShopSettingsRepository> =
        EmailParseTask::new();

    let image_save_ctx = app
        .app_data_dir()
        .ok()
        .map(|dir| (std::sync::Arc::new(pool.clone()), dir.join("images")));

    let context = EmailParseContext {
        pool: Arc::new(pool.clone()),
        parse_repo: Arc::new(parse_repo),
        shop_settings_repo: Arc::new(shop_settings_repo),
        shop_settings_cache: Arc::new(Mutex::new(ShopSettingsCache::default())),
        parse_state: Arc::new(parse_state.clone()),
        image_save_ctx,
    };

    let runner = BatchRunner::new(task, batch_size, 0);
    let parse_state_for_cancel = parse_state.clone();

    match runner
        .run(app, inputs, &context, || {
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

    // 駿河屋 HTML パース
    // html_content が保存済みのレコードをすべてパースする。冪等なので何度でも再実行可能。
    if !parse_state.is_cancelled() {
        run_surugaya_html_parse_step(app, &pool, &parse_state, batch_size).await;
    }

    // Amazon 注文詳細 HTML パース
    if !parse_state.is_cancelled() {
        run_html_parse_step(app, &pool, &parse_state, batch_size).await;
    }

    parse_state.finish();
}

/// 駿河屋マイページ HTML のパースステップ
async fn run_surugaya_html_parse_step<A: BatchCommandsApp>(
    app: &A,
    pool: &SqlitePool,
    parse_state: &crate::parsers::ParseState,
    batch_size: usize,
) {
    run_stored_html_parse_step(
        app,
        pool,
        parse_state,
        batch_size,
        SurugayaHtmlParseTask,
        "surugaya_html_parse",
        "url LIKE 'https://www.suruga-ya.jp/pcmypage/%'",
    )
    .await;
}

/// Amazon 注文詳細 HTML のパースステップ
async fn run_html_parse_step<A: BatchCommandsApp>(
    app: &A,
    pool: &SqlitePool,
    parse_state: &crate::parsers::ParseState,
    batch_size: usize,
) {
    run_stored_html_parse_step(
        app,
        pool,
        parse_state,
        batch_size,
        HtmlParseTask,
        "html_parse",
        "(url LIKE 'https://www.amazon.co.jp/your-orders/order-details%' \
          OR url LIKE 'https://www.amazon.co.jp/gp/your-account/order-details%')",
    )
    .await;
}

/// `htmls` テーブルに保存済みの HTML をパースする共通ステップ
///
/// `url_condition` は対象 URL を絞り込む SQL 条件（固定文字列のみ渡すこと）。
async fn run_stored_html_parse_step<A, T>(
    app: &A,
    pool: &SqlitePool,
    parse_state: &crate::parsers::ParseState,
    batch_size: usize,
    task: T,
    log_tag: &str,
    url_condition: &'static str,
) where
    A: BatchCommandsApp,
    T: BatchTask<Input = HtmlParseInput, Context = HtmlParseContext>,
{
    let query = format!(
        "SELECT id, url, html_content FROM htmls \
         WHERE html_content IS NOT NULL AND {url_condition} \
         ORDER BY id"
    );
    let targets: Vec<(i64, String, String)> = match sqlx::query_as(&query).fetch_all(pool).await {
        Ok(rows) => rows,
        Err(e) => {
            log::error!("[{log_tag}] Failed to fetch html targets: {e}");
            return;
        }
    };

    if targets.is_empty() {
        log::info!("[{log_tag}] No stored HTML to parse");
        let complete_event = BatchProgressEvent::complete(
            task.name(),
            0,
            0,
            0,
            "パース対象の HTML がありません".to_string(),
        );
        app.emit_event(task.event_name(), complete_event);
        return;
    }

    log::info!("[{log_tag}] {} HTML(s) to parse", targets.len());

    let inputs: Vec<HtmlParseInput> = targets
        .into_iter()
        .map(|(html_id, url, html_content)| HtmlParseInput {
            html_id,
            url,
            html_content,
        })
        .collect();

    let context = HtmlParseContext {
        pool: Arc::new(pool.clone()),
    };
    let runner = BatchRunner::new(task, batch_size, 0);

    match runner
        .run(app, inputs, &context, || parse_state.is_cancelled())
        .await
    {
        Ok(result) => {
            log::info!(
                "[{log_tag}] Completed: success={}, failed={}",
                result.success_count,
                result.failed_count
            );
        }
        Err(e) => {
            log::error!("[{log_tag}] BatchRunner failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::test_helpers::*;
    use crate::parsers::EMAIL_PARSE_EVENT_NAME;

    #[tokio::test]
    async fn run_batch_parse_task_emits_error_when_already_running() {
        let pool = create_pool().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let app = FakeApp {
            config_dir: tmp.path().to_path_buf(),
            data_dir: Some(tmp.path().to_path_buf()),
            emitted_events: std::sync::Mutex::new(Vec::new()),
            notify_count: std::sync::atomic::AtomicUsize::new(0),
            fail_create_gmail_client: false,
        };
        let parse_state = crate::parsers::ParseState::new();
        parse_state.try_start().unwrap();

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

        let tmp = tempfile::TempDir::new().unwrap();
        let app = FakeApp {
            config_dir: tmp.path().to_path_buf(),
            data_dir: Some(tmp.path().to_path_buf()),
            emitted_events: std::sync::Mutex::new(Vec::new()),
            notify_count: std::sync::atomic::AtomicUsize::new(0),
            fail_create_gmail_client: false,
        };
        let parse_state = crate::parsers::ParseState::new();

        run_batch_parse_task_with(&app, pool, parse_state.clone(), 10).await;

        // finish されて idle に戻る
        assert!(!parse_state.is_running());

        let emitted = app.emitted_events.lock().unwrap();
        assert!(!emitted.is_empty());
        assert_eq!(emitted[0], EMAIL_PARSE_EVENT_NAME);
    }

    #[tokio::test]
    async fn stored_html_steps_select_only_their_own_urls() {
        let pool = create_pool().await;
        sqlx::query(
            "CREATE TABLE htmls (id INTEGER PRIMARY KEY, url TEXT NOT NULL, html_content TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();
        // Amazon 注文詳細 URL（orderID が無いため process は失敗するが、対象には選ばれる）
        sqlx::query(
            "INSERT INTO htmls (url, html_content) VALUES \
             ('https://www.amazon.co.jp/your-orders/order-details', '<html></html>'), \
             ('https://www.suruga-ya.jp/pcmypage/x', NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let tmp = tempfile::TempDir::new().unwrap();
        let new_app = || FakeApp {
            config_dir: tmp.path().to_path_buf(),
            data_dir: Some(tmp.path().to_path_buf()),
            emitted_events: std::sync::Mutex::new(Vec::new()),
            notify_count: std::sync::atomic::AtomicUsize::new(0),
            fail_create_gmail_client: false,
        };
        let parse_state = crate::parsers::ParseState::new();

        // Amazon: 1件処理 → 進捗 + 完了の 2 イベント
        let app = new_app();
        run_html_parse_step(&app, &pool, &parse_state, 10).await;
        assert_eq!(app.emitted_events.lock().unwrap().len(), 2);

        // 駿河屋: html_content が NULL の行は対象外 → 「対象なし」完了イベントのみ
        let app = new_app();
        run_surugaya_html_parse_step(&app, &pool, &parse_state, 10).await;
        assert_eq!(app.emitted_events.lock().unwrap().len(), 1);
    }
}
