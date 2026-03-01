use sqlx::sqlite::SqlitePool;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use crate::orchestration;

/// 配送状況確認バッチの多重実行ガード＋キャンセル制御
#[derive(Clone, Default)]
pub struct DeliveryCheckState {
    is_running: Arc<Mutex<bool>>,
    should_cancel: Arc<AtomicBool>,
}

impl DeliveryCheckState {
    pub fn new() -> Self {
        Self::default()
    }

    /// バッチを開始する（既に実行中なら Err）
    pub fn try_start(&self) -> Result<(), String> {
        let mut running = self
            .is_running
            .lock()
            .map_err(|e| format!("Lock error: {e}"))?;
        if *running {
            return Err("配送状況確認は既に実行中です。完了するまでお待ちください。".to_string());
        }
        *running = true;
        self.should_cancel.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// バッチ完了時に呼ぶ
    pub fn finish(&self) {
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }
        self.should_cancel.store(false, Ordering::SeqCst);
    }

    /// キャンセルを要求する
    pub fn request_cancel(&self) {
        self.should_cancel.store(true, Ordering::SeqCst);
    }

    /// BatchRunner の should_cancel クロージャ用
    pub fn should_cancel(&self) -> bool {
        self.should_cancel.load(Ordering::SeqCst)
    }
}

/// 配送状況確認バッチを開始
#[tauri::command]
pub async fn start_delivery_check(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    check_state: tauri::State<'_, DeliveryCheckState>,
) -> Result<(), String> {
    check_state.try_start()?;

    let pool_clone = pool.inner().clone();
    let check_state_clone = check_state.inner().clone();
    tauri::async_runtime::spawn(orchestration::run_delivery_check_task(
        app_handle,
        pool_clone,
        check_state_clone,
    ));
    Ok(())
}

/// 配送状況確認バッチをキャンセル
#[tauri::command]
pub async fn cancel_delivery_check(
    check_state: tauri::State<'_, DeliveryCheckState>,
) -> Result<(), String> {
    log::info!("Cancelling delivery check...");
    check_state.request_cancel();
    Ok(())
}
