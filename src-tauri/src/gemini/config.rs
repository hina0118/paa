//! Gemini APIキー管理（Googleと同じJSONファイルベース方式）
//!
//! # セキュリティガイドライン
//! - APIキーは絶対にログに出力しないこと
//! - ファイルは app_data_dir に配置（client_secret.json と同じ場所）

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// APIキー設定ファイルの構造
#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiApiKeyConfig {
    pub api_key: String,
}

/// APIキー設定ファイルのパスを取得
pub fn get_config_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join("gemini_api_key.json")
}

/// APIキーが設定されているかチェック
pub fn has_api_key(app_data_dir: &PathBuf) -> bool {
    get_config_path(app_data_dir).exists()
}

/// APIキーをファイルから読み込み
///
/// # セキュリティ
/// APIキーはログに出力されません
pub fn load_api_key(app_data_dir: &PathBuf) -> Result<String, String> {
    let config_path = get_config_path(app_data_dir);

    if !config_path.exists() {
        return Err(format!(
            "Gemini API key file not found. Please place gemini_api_key.json at: {}\n\n\
            File format: {{\"api_key\": \"YOUR_API_KEY\"}}\n\n\
            This is the same directory where client_secret.json is stored.",
            config_path.display()
        ));
    }

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read Gemini API key file: {e}"))?;

    let config: GeminiApiKeyConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse Gemini API key file: {e}"))?;

    if config.api_key.is_empty() {
        return Err("Gemini API key is empty".to_string());
    }

    // セキュリティ: APIキーをログに出力しない
    log::info!("Gemini API key loaded successfully");

    Ok(config.api_key)
}

/// APIキーをファイルに保存
///
/// # セキュリティ
/// APIキーはログに出力されません
pub fn save_api_key(app_data_dir: &PathBuf, api_key: &str) -> Result<(), String> {
    let config_path = get_config_path(app_data_dir);

    let config = GeminiApiKeyConfig {
        api_key: api_key.to_string(),
    };

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize API key config: {e}"))?;

    std::fs::write(&config_path, content)
        .map_err(|e| format!("Failed to write Gemini API key file: {e}"))?;

    // セキュリティ: APIキーをログに出力しない
    log::info!("Gemini API key saved successfully");

    Ok(())
}

/// APIキーファイルを削除
pub fn delete_api_key(app_data_dir: &PathBuf) -> Result<(), String> {
    let config_path = get_config_path(app_data_dir);

    if config_path.exists() {
        std::fs::remove_file(&config_path)
            .map_err(|e| format!("Failed to delete Gemini API key file: {e}"))?;
        log::info!("Gemini API key deleted successfully");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_has_api_key_returns_false_when_file_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path().to_path_buf();

        assert!(!has_api_key(&app_data_dir));
    }

    #[test]
    fn test_has_api_key_returns_true_when_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path().to_path_buf();

        // ファイルを作成
        let config_path = get_config_path(&app_data_dir);
        fs::write(&config_path, r#"{"api_key": "test_key"}"#).unwrap();

        assert!(has_api_key(&app_data_dir));
    }

    #[test]
    fn test_load_api_key_success() {
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path().to_path_buf();

        // ファイルを作成
        let config_path = get_config_path(&app_data_dir);
        fs::write(&config_path, r#"{"api_key": "test_api_key_12345"}"#).unwrap();

        let result = load_api_key(&app_data_dir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_api_key_12345");
    }

    #[test]
    fn test_load_api_key_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path().to_path_buf();

        let result = load_api_key(&app_data_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_load_api_key_empty_key() {
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path().to_path_buf();

        // 空のAPIキーでファイルを作成
        let config_path = get_config_path(&app_data_dir);
        fs::write(&config_path, r#"{"api_key": ""}"#).unwrap();

        let result = load_api_key(&app_data_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_save_and_load_api_key() {
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path().to_path_buf();

        // 保存
        let api_key = "my_secret_api_key";
        let save_result = save_api_key(&app_data_dir, api_key);
        assert!(save_result.is_ok());

        // 読み込み
        let load_result = load_api_key(&app_data_dir);
        assert!(load_result.is_ok());
        assert_eq!(load_result.unwrap(), api_key);
    }

    #[test]
    fn test_delete_api_key() {
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path().to_path_buf();

        // 保存
        save_api_key(&app_data_dir, "test_key").unwrap();
        assert!(has_api_key(&app_data_dir));

        // 削除
        let delete_result = delete_api_key(&app_data_dir);
        assert!(delete_result.is_ok());
        assert!(!has_api_key(&app_data_dir));
    }
}
