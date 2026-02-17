use sqlx::sqlite::SqlitePool;
use tauri::Manager;

use crate::batch_commands;
use crate::config;
use crate::gmail;

/// 最大繰り返し回数のバリデーション（1以上である必要がある）
pub fn validate_max_iterations(max_iterations: i64) -> Result<(), String> {
    if max_iterations <= 0 {
        return Err("最大繰り返し回数は1以上である必要があります".to_string());
    }
    Ok(())
}

/// 1ページあたり取得件数のバリデーション（1〜500）
pub fn validate_max_results_per_page(max_results_per_page: i64) -> Result<(), String> {
    if !(1..=500).contains(&max_results_per_page) {
        return Err("1ページあたり取得件数は1〜500の範囲である必要があります".to_string());
    }
    Ok(())
}

/// 同期タイムアウト（分）のバリデーション（1〜120）
pub fn validate_timeout_minutes(timeout_minutes: i64) -> Result<(), String> {
    if !(1..=120).contains(&timeout_minutes) {
        return Err("同期タイムアウトは1〜120分の範囲である必要があります".to_string());
    }
    Ok(())
}

/// Gmail同期処理を開始
/// BatchRunner<GmailSyncTask> を使用
#[tauri::command]
pub async fn start_sync(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    sync_state: tauri::State<'_, gmail::SyncState>,
) -> Result<(), String> {
    let pool_clone = pool.inner().clone();
    let sync_state_clone = sync_state.inner().clone();
    tauri::async_runtime::spawn(batch_commands::run_sync_task(
        app_handle,
        pool_clone,
        sync_state_clone,
    ));
    Ok(())
}

#[tauri::command]
pub async fn cancel_sync(sync_state: tauri::State<'_, gmail::SyncState>) -> Result<(), String> {
    log::info!("Cancelling sync...");
    sync_state.request_cancel();
    Ok(())
}

#[tauri::command]
pub async fn get_sync_status(
    app_handle: tauri::AppHandle,
    sync_state: tauri::State<'_, gmail::SyncState>,
) -> Result<gmail::SyncMetadata, String> {
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let config = config::load(&app_config_dir)?;

    let sync_status = if sync_state.inner().is_running() {
        "syncing"
    } else if sync_state
        .inner()
        .last_error
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false)
    {
        "error"
    } else {
        "idle"
    };

    let last_error_message = sync_state
        .inner()
        .last_error
        .lock()
        .ok()
        .and_then(|g| g.clone());

    Ok(gmail::SyncMetadata {
        sync_status: sync_status.to_string(),
        oldest_fetched_date: None,
        total_synced_count: 0,
        batch_size: config.sync.batch_size,
        last_sync_started_at: None,
        last_sync_completed_at: None,
        max_iterations: config.sync.max_iterations,
        max_results_per_page: config.sync.max_results_per_page,
        timeout_minutes: config.sync.timeout_minutes,
        last_error_message,
    })
}

#[tauri::command]
pub async fn reset_sync_status(
    sync_state: tauri::State<'_, gmail::SyncState>,
) -> Result<(), String> {
    log::info!("Resetting sync status to 'idle'");
    sync_state.inner().force_idle();
    Ok(())
}

#[tauri::command]
pub async fn reset_sync_date() -> Result<(), String> {
    log::info!("reset_sync_date: no-op (oldest_fetched_date は未使用)");
    Ok(())
}

#[tauri::command]
pub async fn update_batch_size(
    app_handle: tauri::AppHandle,
    batch_size: i64,
) -> Result<(), String> {
    log::info!("Updating sync batch size to: {batch_size}");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.sync.batch_size = batch_size;
    config::save(&app_config_dir, &config)
}

#[tauri::command]
pub async fn update_max_iterations(
    app_handle: tauri::AppHandle,
    max_iterations: i64,
) -> Result<(), String> {
    validate_max_iterations(max_iterations)?;

    log::info!("Updating max iterations to: {max_iterations}");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.sync.max_iterations = max_iterations;
    config::save(&app_config_dir, &config)
}

#[tauri::command]
pub async fn update_max_results_per_page(
    app_handle: tauri::AppHandle,
    max_results_per_page: i64,
) -> Result<(), String> {
    validate_max_results_per_page(max_results_per_page)?;
    log::info!("Updating max results per page to: {max_results_per_page}");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.sync.max_results_per_page = max_results_per_page;
    config::save(&app_config_dir, &config)
}

#[tauri::command]
pub async fn update_timeout_minutes(
    app_handle: tauri::AppHandle,
    timeout_minutes: i64,
) -> Result<(), String> {
    validate_timeout_minutes(timeout_minutes)?;
    log::info!("Updating sync timeout to: {timeout_minutes} minutes");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.sync.timeout_minutes = timeout_minutes;
    config::save(&app_config_dir, &config)
}

/// Gmail メール取得（BatchRunner 経由で start_sync と同等の処理を実行）
#[tauri::command]
pub async fn fetch_gmail_emails(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    sync_state: tauri::State<'_, gmail::SyncState>,
) -> Result<gmail::FetchResult, String> {
    log::info!("Starting Gmail email fetch (via start_sync / BatchRunner)...");
    log::info!("If a browser window doesn't open automatically, please check the console for the authentication URL.");

    // BatchRunner を使用する start_sync に委譲
    start_sync(app_handle, pool, sync_state).await?;

    // 進捗は batch-progress イベントで通知される
    Ok(gmail::FetchResult {
        fetched_count: 0,
        saved_count: 0,
        skipped_count: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_max_results_per_page_boundaries() {
        assert!(validate_max_results_per_page(1).is_ok());
        assert!(validate_max_results_per_page(500).is_ok());
        assert!(validate_max_results_per_page(0).is_err());
        assert!(validate_max_results_per_page(501).is_err());
    }

    #[test]
    fn test_validate_timeout_minutes_boundaries() {
        assert!(validate_timeout_minutes(1).is_ok());
        assert!(validate_timeout_minutes(120).is_ok());
        assert!(validate_timeout_minutes(0).is_err());
        assert!(validate_timeout_minutes(121).is_err());
    }

    #[test]
    fn test_validate_max_iterations_valid() {
        assert!(validate_max_iterations(1).is_ok());
        assert!(validate_max_iterations(100).is_ok());
    }

    #[test]
    fn test_validate_max_iterations_zero() {
        let result = validate_max_iterations(0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("1以上"));
    }

    #[test]
    fn test_validate_max_iterations_negative() {
        let result = validate_max_iterations(-1);
        assert!(result.is_err());
    }
}
