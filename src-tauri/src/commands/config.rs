use tauri::Manager;

use crate::config;

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
}
