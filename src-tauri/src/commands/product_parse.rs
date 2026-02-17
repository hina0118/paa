use sqlx::sqlite::SqlitePool;
use std::sync::{Arc, Mutex};
use tauri::Manager;

use crate::orchestration;

/// 商品名パースの多重実行ガード用状態
#[derive(Clone, Default)]
pub struct ProductNameParseState {
    is_running: Arc<Mutex<bool>>,
}

impl ProductNameParseState {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    pub fn try_start(&self) -> Result<(), String> {
        let mut running = self
            .is_running
            .lock()
            .map_err(|e| format!("Lock error: {e}"))?;
        if *running {
            return Err("商品名解析は既に実行中です。完了するまでお待ちください。".to_string());
        }
        *running = true;
        Ok(())
    }

    pub fn finish(&self) {
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }
    }
}

/// 商品名パース進捗イベント（後方互換性のため残す）
/// 新しいコードでは BatchProgressEvent を使用してください
#[derive(Debug, Clone, serde::Serialize)]
#[deprecated(note = "Use BatchProgressEvent instead")]
pub struct ProductNameParseProgress {
    pub total_items: usize,
    pub parsed_count: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub status_message: String,
    pub is_complete: bool,
    pub error: Option<String>,
}

/// product_masterに未登録の商品名をGemini APIで解析して登録
/// BatchRunner<ProductNameParseTask> を使用
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
