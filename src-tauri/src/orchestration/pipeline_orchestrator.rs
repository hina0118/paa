//! スケジューラ用パイプラインオーケストレーション。
//!
//! 「差分同期 → メールパース → 商品名解析 → 配達状況確認」を順番に実行する。
//! 各ステップの成否に関わらず次のステップへ進む（ベストエフォート方式）。
//! 手動実行中のステップはスキップして次へ進む。
//!
//! メールパース以降は前ステップで新規データが生まれた場合のみ実行する。
//! ただし手動実行中でスキップされたステップは「結果不明」として後続ステップに進む。
//!
//! 各ステップの実装は [`super::pipeline_steps`] で共通化されており、
//! UI 用パイプライン（#290）と共有する。

use sqlx::sqlite::SqlitePool;
use tauri::Manager;

use super::pipeline_steps::{
    run_delivery_check_step, run_parse_step, run_product_parse_step, run_sync_step, StepOutcome,
};

/// パイプラインを実行する。スケジューラから呼ばれる。
pub async fn run_pipeline(app: &tauri::AppHandle) {
    let pool = match app.try_state::<SqlitePool>() {
        Some(p) => p.inner().clone(),
        None => {
            log::error!("[Pipeline] SqlitePool not available, aborting");
            return;
        }
    };

    // Step 1: 差分同期
    let sync_outcome = run_sync_step(app, &pool).await;
    match &sync_outcome {
        StepOutcome::Ran { new_count: 0 } => {
            log::info!("[Pipeline] No new emails synced, skipping subsequent steps");
            return;
        }
        StepOutcome::Ran { new_count } => {
            log::info!("[Pipeline] {new_count} new email(s) synced, proceeding to parse");
        }
        StepOutcome::Skipped => {
            log::info!("[Pipeline] Sync was skipped, proceeding to parse anyway");
        }
        StepOutcome::Unknown => {
            log::info!(
                "[Pipeline] Sync ran but email count is unknown, proceeding to parse anyway"
            );
        }
    }

    // Step 2: メールパース
    let parse_outcome = run_parse_step(app, &pool).await;
    match &parse_outcome {
        StepOutcome::Ran { new_count: 0 } => {
            log::info!("[Pipeline] No new orders after parse, skipping subsequent steps");
            return;
        }
        StepOutcome::Ran { new_count } => {
            log::info!("[Pipeline] {new_count} new order(s) after parse, proceeding");
        }
        StepOutcome::Skipped => {
            log::info!("[Pipeline] Parse was skipped, proceeding anyway");
        }
        StepOutcome::Unknown => {
            log::info!("[Pipeline] Parse ran but order count is unknown, proceeding anyway");
        }
    }

    // Step 3: 商品名解析
    run_product_parse_step(app, &pool).await;

    // Step 4: 配達状況確認
    run_delivery_check_step(app, &pool).await;
}
