use tauri::Manager;

use crate::gemini;
use crate::gmail;
use crate::google_search;

// =============================================================================
// Gemini API Commands
// =============================================================================

/// Gemini APIキーが設定されているかチェック
#[tauri::command]
pub async fn has_gemini_api_key(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    Ok(gemini::has_api_key(&app_data_dir))
}

/// Gemini APIキーを保存
#[tauri::command]
pub async fn save_gemini_api_key(
    app_handle: tauri::AppHandle,
    api_key: String,
) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    gemini::config::save_api_key(&app_data_dir, &api_key)?;

    log::info!("Gemini API key saved successfully");
    Ok(())
}

/// Gemini APIキーを削除
#[tauri::command]
pub async fn delete_gemini_api_key(app_handle: tauri::AppHandle) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    gemini::config::delete_api_key(&app_data_dir)?;

    log::info!("Gemini API key deleted successfully");
    Ok(())
}

// =============================================================================
// Gmail OAuth Commands
// =============================================================================

/// Gmail OAuth認証情報が設定されているかチェック
#[tauri::command]
pub async fn has_gmail_oauth_credentials(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    Ok(gmail::has_oauth_credentials(&app_data_dir))
}

/// Gmail OAuth認証情報を保存（JSONから）
#[tauri::command]
pub async fn save_gmail_oauth_credentials(
    app_handle: tauri::AppHandle,
    json_content: String,
) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    gmail::save_oauth_credentials_from_json(&app_data_dir, &json_content)?;
    log::info!("Gmail OAuth credentials saved successfully");
    Ok(())
}

/// Gmail OAuth認証情報を削除
#[tauri::command]
pub async fn delete_gmail_oauth_credentials(app_handle: tauri::AppHandle) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    gmail::delete_oauth_credentials(&app_data_dir)?;
    log::info!("Gmail OAuth credentials deleted successfully");
    Ok(())
}

// =============================================================================
// SerpApi Image Search Config Commands
// =============================================================================

/// SerpApi が設定済みかチェック（API Key のみ）
#[tauri::command]
pub async fn is_google_search_configured(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    Ok(google_search::is_configured(&app_data_dir))
}

/// SerpApi API キーを保存
#[tauri::command]
pub async fn save_google_search_api_key(
    app_handle: tauri::AppHandle,
    api_key: String,
) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    google_search::save_api_key(&app_data_dir, &api_key)?;

    log::info!("SerpApi API key saved successfully");
    Ok(())
}

/// SerpApi API 設定を削除
#[tauri::command]
pub async fn delete_google_search_config(app_handle: tauri::AppHandle) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    google_search::delete_api_key(&app_data_dir)?;

    log::info!("SerpApi config deleted successfully");
    Ok(())
}
