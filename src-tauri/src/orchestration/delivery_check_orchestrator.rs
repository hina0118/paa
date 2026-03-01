//! 配送状況確認オーケストレーション

use sqlx::sqlite::SqlitePool;

use super::{BatchCommandsApp, TauriBatchCommandsApp};
use crate::batch_runner::{BatchProgressEvent, BatchRunner};
use crate::commands::DeliveryCheckState;
use crate::delivery_check::{
    DeliveryCheckContext, DeliveryCheckInput, DeliveryCheckTask, DELIVERY_CHECK_EVENT_NAME,
    DELIVERY_CHECK_TASK_NAME,
};

/// 配送状況確認タスクの本体。コマンドから呼ばれる。
pub async fn run_delivery_check_task(
    app: tauri::AppHandle,
    pool: SqlitePool,
    check_state: DeliveryCheckState,
) {
    let app = TauriBatchCommandsApp { app };
    run_delivery_check_task_with(&app, pool, check_state).await
}

async fn run_delivery_check_task_with<A: BatchCommandsApp>(
    app: &A,
    pool: SqlitePool,
    check_state: DeliveryCheckState,
) {
    use crate::orchestration::error_handler::ErrorReporter;

    log::info!("Starting delivery check with BatchRunner<DeliveryCheckTask>...");

    let err = ErrorReporter::new(app, DELIVERY_CHECK_TASK_NAME, DELIVERY_CHECK_EVENT_NAME);

    // HTTP クライアント作成
    let ctx = match DeliveryCheckContext::new(pool.clone()) {
        Ok(c) => c,
        Err(e) => {
            err.report_zero(&format!("HTTPクライアントの作成に失敗: {e}"));
            check_state.finish();
            return;
        }
    };

    // 対象: 未配達かつ追跡番号あり
    let rows: Vec<(i64, String, String)> = match sqlx::query_as(
        r#"
        SELECT id, tracking_number, carrier
        FROM deliveries
        WHERE delivery_status NOT IN ('delivered', 'cancelled', 'returned')
          AND tracking_number IS NOT NULL
          AND tracking_number != ''
          AND carrier IS NOT NULL
          AND carrier != ''
        ORDER BY updated_at ASC
        "#,
    )
    .fetch_all(&pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            err.report_zero(&format!("配送情報の取得に失敗: {e}"));
            check_state.finish();
            return;
        }
    };

    let total_items = rows.len();
    log::info!("[DeliveryCheck] {} deliveries to check", total_items);

    if total_items == 0 {
        let complete = BatchProgressEvent::complete(
            DELIVERY_CHECK_TASK_NAME,
            0,
            0,
            0,
            "確認対象の配送情報がありません".to_string(),
        );
        app.emit_event(DELIVERY_CHECK_EVENT_NAME, complete);
        check_state.finish();
        return;
    }

    let inputs: Vec<DeliveryCheckInput> = rows
        .into_iter()
        .map(|(id, tracking_number, carrier)| DeliveryCheckInput {
            delivery_id: id,
            tracking_number,
            carrier,
        })
        .collect();

    // バッチサイズ 5・バッチ間 3 秒（配送業者サイトへの負荷を抑える）
    let runner = BatchRunner::new(DeliveryCheckTask, 5, 3_000);
    let check_state_for_cancel = check_state.clone();

    match runner
        .run(app, inputs, &ctx, move || {
            check_state_for_cancel.should_cancel()
        })
        .await
    {
        Ok(result) => {
            log::info!(
                "[DeliveryCheck] completed: success={}, failed={}",
                result.success_count,
                result.failed_count
            );
        }
        Err(e) => {
            err.report(
                &format!("バッチ処理エラー: {e}"),
                total_items,
                0,
                0,
                total_items,
            );
        }
    }

    check_state.finish();
}
