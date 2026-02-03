//! SerpApi APIキー管理
//!
//! # セキュリティガイドライン
//! - APIキーは絶対にログに出力しないこと
//! - 永続化には OS のセキュアストレージ（keyring）を使用すること

use keyring::Entry;
use std::path::Path;

/// keyring 用のエントリを取得（SerpApi API Key）
fn serpapi_api_key_entry() -> Result<Entry, String> {
    Entry::new("paa-serpapi", "serpapi-api-key")
        .map_err(|e| format!("Failed to access secure storage for API key: {e}"))
}

/// APIキーが設定されているかチェック
pub fn has_api_key(_app_data_dir: &Path) -> bool {
    if let Ok(entry) = serpapi_api_key_entry() {
        if let Ok(secret) = entry.get_password() {
            return !secret.is_empty();
        }
    }
    false
}

/// 設定が完了しているかチェック（SerpApiはAPIキーのみ）
pub fn is_configured(app_data_dir: &Path) -> bool {
    has_api_key(app_data_dir)
}

/// APIキーを読み込み
///
/// # セキュリティ
/// APIキーはログに出力されません
pub fn load_api_key(_app_data_dir: &Path) -> Result<String, String> {
    let entry = serpapi_api_key_entry()?;
    let secret = entry
        .get_password()
        .map_err(|e| format!("Failed to load SerpApi API key from secure storage: {e}"))?;

    if secret.is_empty() {
        return Err("SerpApi API key is empty".to_string());
    }

    log::info!("SerpApi API key loaded successfully from secure storage");
    Ok(secret)
}

/// APIキーを保存
///
/// # セキュリティ
/// APIキーはログに出力されません
pub fn save_api_key(_app_data_dir: &Path, api_key: &str) -> Result<(), String> {
    if api_key.is_empty() {
        return Err("SerpApi API key is empty".to_string());
    }

    let entry = serpapi_api_key_entry()?;
    entry
        .set_password(api_key)
        .map_err(|e| format!("Failed to save SerpApi API key to secure storage: {e}"))?;

    log::info!("SerpApi API key saved successfully to secure storage");
    Ok(())
}

/// APIキーを削除
pub fn delete_api_key(_app_data_dir: &Path) -> Result<(), String> {
    let entry = serpapi_api_key_entry()?;
    entry
        .delete_credential()
        .map_err(|e| format!("Failed to delete SerpApi API key from secure storage: {e}"))?;

    log::info!("SerpApi API key deleted successfully from secure storage");
    Ok(())
}

#[cfg(test)]
#[cfg(not(ci))]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    /// テスト用: keyring のエントリをクリーンアップ
    fn cleanup_test_keyring() {
        if let Ok(entry) = serpapi_api_key_entry() {
            let _ = entry.delete_credential();
        }
    }

    #[test]
    #[serial]
    fn test_has_api_key_returns_false_when_empty() {
        cleanup_test_keyring();
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path();

        assert!(!has_api_key(app_data_dir));
    }

    #[test]
    #[serial]
    fn test_has_api_key_returns_true_when_set() {
        cleanup_test_keyring();
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path();

        save_api_key(app_data_dir, "test_key").unwrap();
        assert!(has_api_key(app_data_dir));
        cleanup_test_keyring();
    }

    #[test]
    #[serial]
    fn test_is_configured() {
        cleanup_test_keyring();
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path();

        // APIキーなし
        assert!(!is_configured(app_data_dir));

        // APIキーあり
        save_api_key(app_data_dir, "test_key").unwrap();
        assert!(is_configured(app_data_dir));

        cleanup_test_keyring();
    }

    #[test]
    #[serial]
    fn test_save_and_load_api_key() {
        cleanup_test_keyring();
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path();

        let api_key = "my_secret_api_key";
        let save_result = save_api_key(app_data_dir, api_key);
        assert!(save_result.is_ok());

        let load_result = load_api_key(app_data_dir);
        assert!(load_result.is_ok());
        assert_eq!(load_result.unwrap(), api_key);

        cleanup_test_keyring();
    }

    #[test]
    #[serial]
    fn test_delete_api_key() {
        cleanup_test_keyring();
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path();

        save_api_key(app_data_dir, "test_key").unwrap();
        assert!(has_api_key(app_data_dir));

        let delete_result = delete_api_key(app_data_dir);
        assert!(delete_result.is_ok());
        assert!(!has_api_key(app_data_dir));
    }
}
