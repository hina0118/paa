use sqlx::sqlite::SqlitePool;
use tauri::Manager;

use crate::orchestration;

/// 商品名パースの多重実行ガード・キャンセル制御用状態（`BatchRunState` の薄いラッパー）
#[derive(Clone, Default)]
pub struct ProductNameParseState(crate::BatchRunState);

impl ProductNameParseState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn try_start(&self) -> Result<(), String> {
        self.0
            .try_start()
            .map_err(|_| "商品名解析は既に実行中です。完了するまでお待ちください。".to_string())
    }

    pub fn finish(&self) {
        self.0.finish();
    }

    pub fn request_cancel(&self) {
        self.0.request_cancel();
    }

    pub fn should_cancel(&self) -> bool {
        self.0.should_cancel()
    }
}

/// product_master に未登録の商品名を Gemini API で解析して登録
/// `BatchRunner<ProductNameParseTask>` を使用
#[tauri::command]
pub async fn start_product_name_parse(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    parse_state: tauri::State<'_, ProductNameParseState>,
) -> Result<(), String> {
    // spawn 前の事前チェック（APIキー有無等）で Err を返せるようにする
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    if !crate::e2e_mocks::is_e2e_mock_mode() && !crate::gemini::has_api_key(&app_data_dir) {
        return Err(
            "Gemini APIキーが設定されていません。設定画面でAPIキーを設定してください。".to_string(),
        );
    }

    if let Err(e) = parse_state.try_start() {
        return Err(e.to_string());
    }

    let pool_clone = pool.inner().clone();
    let parse_state_clone = parse_state.inner().clone();
    tauri::async_runtime::spawn(orchestration::run_product_name_parse_task(
        app_handle,
        pool_clone,
        parse_state_clone,
        true, // caller で try_start 済み
    ));
    Ok(())
}

/// 商品名パースバッチをキャンセル
#[tauri::command]
pub async fn cancel_product_name_parse(
    parse_state: tauri::State<'_, ProductNameParseState>,
) -> Result<(), String> {
    log::info!("Cancelling product name parse...");
    parse_state.request_cancel();
    Ok(())
}
