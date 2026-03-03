use tauri::Manager;

use crate::config;
use crate::scheduler::{SCHEDULER_INTERVAL_MAX_MINUTES, SCHEDULER_INTERVAL_MIN_MINUTES};

/// Gemini バッチサイズのバリデーション（1〜50）
pub fn validate_gemini_batch_size(batch_size: i64) -> Result<(), String> {
    if !(1..=50).contains(&batch_size) {
        return Err("商品名パースのバッチサイズは1〜50の範囲である必要があります".to_string());
    }
    Ok(())
}

/// Gemini リクエスト間待機秒数のバリデーション（0〜60）
pub fn validate_gemini_delay_seconds(delay_seconds: i64) -> Result<(), String> {
    if !(0..=60).contains(&delay_seconds) {
        return Err("リクエスト間の待機秒数は0〜60の範囲である必要があります".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn get_gemini_config(
    app_handle: tauri::AppHandle,
) -> Result<config::GeminiConfig, String> {
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let config = config::load(&app_config_dir)?;
    Ok(config.gemini)
}

#[tauri::command]
pub async fn update_gemini_batch_size(
    app_handle: tauri::AppHandle,
    batch_size: i64,
) -> Result<(), String> {
    validate_gemini_batch_size(batch_size)?;
    log::info!("Updating Gemini batch size to: {batch_size}");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.gemini.batch_size = batch_size;
    config::save(&app_config_dir, &config)
}

#[tauri::command]
pub async fn update_gemini_delay_seconds(
    app_handle: tauri::AppHandle,
    delay_seconds: i64,
) -> Result<(), String> {
    validate_gemini_delay_seconds(delay_seconds)?;
    log::info!("Updating Gemini delay to: {delay_seconds} seconds");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.gemini.delay_seconds = delay_seconds;
    config::save(&app_config_dir, &config)
}

// ---------------------------------------------------------------------------
// スケジューラ設定
// ---------------------------------------------------------------------------

/// スケジューラ実行間隔のバリデーション（`SCHEDULER_INTERVAL_MIN_MINUTES`〜`SCHEDULER_INTERVAL_MAX_MINUTES`分）
pub fn validate_scheduler_interval(interval_minutes: i64) -> Result<(), String> {
    if !(SCHEDULER_INTERVAL_MIN_MINUTES..=SCHEDULER_INTERVAL_MAX_MINUTES)
        .contains(&interval_minutes)
    {
        return Err(format!(
            "スケジューラの実行間隔は{}〜{}分の範囲である必要があります",
            SCHEDULER_INTERVAL_MIN_MINUTES, SCHEDULER_INTERVAL_MAX_MINUTES,
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn get_scheduler_config(
    app_handle: tauri::AppHandle,
) -> Result<config::SchedulerConfig, String> {
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let config = config::load(&app_config_dir)?;
    Ok(config.scheduler)
}

#[tauri::command]
pub async fn update_scheduler_interval(
    app_handle: tauri::AppHandle,
    interval_minutes: i64,
) -> Result<(), String> {
    validate_scheduler_interval(interval_minutes)?;
    log::info!("Updating scheduler interval to: {interval_minutes} minutes");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.scheduler.interval_minutes = interval_minutes;
    config::save(&app_config_dir, &config)?;

    if let Some(sched_state) = app_handle.try_state::<crate::scheduler::SchedulerState>() {
        sched_state.set_interval_minutes(interval_minutes);
    }

    Ok(())
}

#[tauri::command]
pub async fn update_scheduler_enabled(
    app_handle: tauri::AppHandle,
    enabled: bool,
) -> Result<(), String> {
    log::info!("Updating scheduler enabled to: {enabled}");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.scheduler.enabled = enabled;
    config::save(&app_config_dir, &config)?;

    // SchedulerState にも即座に反映
    if let Some(sched_state) = app_handle.try_state::<crate::scheduler::SchedulerState>() {
        sched_state.set_enabled(enabled);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_gemini_batch_size_boundaries() {
        assert!(validate_gemini_batch_size(1).is_ok());
        assert!(validate_gemini_batch_size(50).is_ok());
        assert!(validate_gemini_batch_size(0).is_err());
        assert!(validate_gemini_batch_size(51).is_err());
    }

    #[test]
    fn test_validate_gemini_delay_seconds_boundaries() {
        assert!(validate_gemini_delay_seconds(0).is_ok());
        assert!(validate_gemini_delay_seconds(60).is_ok());
        assert!(validate_gemini_delay_seconds(-1).is_err());
        assert!(validate_gemini_delay_seconds(61).is_err());
    }

    #[test]
    fn test_validate_scheduler_interval_boundaries() {
        assert!(validate_scheduler_interval(1).is_ok());
        assert!(validate_scheduler_interval(1440).is_ok());
        assert!(validate_scheduler_interval(10080).is_ok());
        assert!(validate_scheduler_interval(0).is_err());
        assert!(validate_scheduler_interval(-1).is_err());
        assert!(validate_scheduler_interval(10081).is_err());
    }
}
