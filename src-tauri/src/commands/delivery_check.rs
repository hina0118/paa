use sqlx::sqlite::SqlitePool;

use crate::orchestration;

/// 配送状況確認バッチの多重実行ガード・キャンセル制御用状態（`BatchRunState` の薄いラッパー）
#[derive(Clone, Default)]
pub struct DeliveryCheckState(crate::BatchRunState);

impl DeliveryCheckState {
    pub fn new() -> Self {
        Self::default()
    }

    /// バッチを開始する（既に実行中なら Err）
    pub fn try_start(&self) -> Result<(), String> {
        self.0
            .try_start()
            .map_err(|_| "配送状況確認は既に実行中です。完了するまでお待ちください。".to_string())
    }

    /// バッチ完了時に呼ぶ
    pub fn finish(&self) {
        self.0.finish();
    }

    /// キャンセルを要求する
    pub fn request_cancel(&self) {
        self.0.request_cancel();
    }

    /// `BatchRunner` の `should_cancel` クロージャ用
    pub fn should_cancel(&self) -> bool {
        self.0.should_cancel()
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
