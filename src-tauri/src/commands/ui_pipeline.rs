//! UI 一括パースパイプライン Tauri コマンド。

use sqlx::SqlitePool;

/// UI 一括パースパイプラインを開始する。
///
/// ① メールパース → ② 駿河屋HTMLパース → ③ 商品名パース → ④ 配送確認
/// をベストエフォート方式で順番に実行する。
///
/// 各ステップは既に手動実行中の場合はスキップされる。
/// 駿河屋ステップは `surugaya-session` ウィンドウが開いていない場合はスキップされる。
///
/// ## イベント
/// - `full-parse:step_started { step }` – 各ステップ開始時
/// - `full-parse:complete` – 全ステップ完了時
#[tauri::command]
pub async fn start_full_parse_pipeline(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    let pool_clone = pool.inner().clone();
    tauri::async_runtime::spawn(crate::orchestration::run_full_parse_pipeline(
        app_handle,
        pool_clone,
    ));
    Ok(())
}
