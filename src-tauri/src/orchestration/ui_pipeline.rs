//! UI 一括パースパイプライン。
//!
//! `start_full_parse_pipeline` コマンドから呼ばれ、
//! ① メールパース → ② 駿河屋HTMLパース → ③ 商品名パース → ④ 配送確認
//! をベストエフォート方式で順番に実行する。
//!
//! 各ステップの実装は [`super::pipeline_steps`] で共通化されており、
//! スケジューラ用 [`super::pipeline_orchestrator`] と共有する。

use sqlx::sqlite::SqlitePool;
use tauri::{Emitter, Manager};

use super::pipeline_steps::{
    run_delivery_check_step, run_parse_step, run_product_parse_step, run_surugaya_step,
    StepOutcome,
};

/// 各ステップの名前（`full-parse:step_started` イベントのペイロード）
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStep {
    Parse,
    Surugaya,
    ProductParse,
    DeliveryCheck,
}

/// UI 一括パースパイプラインを実行する。`start_full_parse_pipeline` コマンドから呼ばれる。
///
/// ## 実行順序
/// ① メールパース → ② 駿河屋HTMLパース → ③ 商品名パース → ④ 配送確認
///
/// ## 方針
/// - ベストエフォート：各ステップの成否に関わらず次のステップへ進む
/// - 駿河屋ステップは `surugaya-session` ウィンドウが開いていない場合はスキップ
/// - 各ステップ開始前に `full-parse:step_started` イベントを emit する
/// - 全ステップ完了後に `full-parse:complete` イベントを emit する
pub async fn run_full_parse_pipeline(app: tauri::AppHandle, pool: SqlitePool) {
    log::info!("[UI Pipeline] Starting full parse pipeline");

    // Step 1: メールパース
    emit_step_started(&app, PipelineStep::Parse);
    let parse_outcome = run_parse_step(&app, &pool).await;
    log::info!(
        "[UI Pipeline] Step 1/4 parse: {}",
        outcome_label(&parse_outcome)
    );

    // Step 2: 駿河屋HTMLパース
    emit_step_started(&app, PipelineStep::Surugaya);
    let surugaya_outcome = run_surugaya_step(&app, &pool).await;
    log::info!(
        "[UI Pipeline] Step 2/4 surugaya: {}",
        outcome_label(&surugaya_outcome)
    );

    // Step 3: 商品名パース
    emit_step_started(&app, PipelineStep::ProductParse);
    run_product_parse_step(&app, &pool).await;
    log::info!("[UI Pipeline] Step 3/4 product_parse: done");

    // Step 4: 配送状況確認
    emit_step_started(&app, PipelineStep::DeliveryCheck);
    run_delivery_check_step(&app, &pool).await;
    log::info!("[UI Pipeline] Step 4/4 delivery_check: done");

    // 完了イベント
    let _ = app.emit("full-parse:complete", ());
    log::info!("[UI Pipeline] Full parse pipeline completed");
}

fn emit_step_started(app: &tauri::AppHandle, step: PipelineStep) {
    let _ = app.emit("full-parse:step_started", step);
}

fn outcome_label(outcome: &StepOutcome) -> &'static str {
    match outcome {
        StepOutcome::Ran { .. } => "ran",
        StepOutcome::Skipped => "skipped",
        StepOutcome::Unknown => "unknown",
    }
}
