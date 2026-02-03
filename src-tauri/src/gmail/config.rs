//! Gmail OAuth認証情報管理
//!
//! # セキュリティガイドライン
//! - client_id/client_secretは絶対にログに出力しないこと
//! - 永続化には OS のセキュアストレージ（keyring）を使用すること

use keyring::Entry;
use serde::Deserialize;

/// keyring のサービス名
const KEYRING_SERVICE: &str = "paa-gmail-oauth";

/// keyring 用のエントリを取得（client_id用）
fn client_id_entry() -> Result<Entry, String> {
    Entry::new(KEYRING_SERVICE, "gmail-client-id")
        .map_err(|e| format!("Failed to access secure storage for client_id: {e}"))
}

/// keyring 用のエントリを取得（client_secret用）
fn client_secret_entry() -> Result<Entry, String> {
    Entry::new(KEYRING_SERVICE, "gmail-client-secret")
        .map_err(|e| format!("Failed to access secure storage for client_secret: {e}"))
}

/// OAuth認証情報が設定されているかチェック
pub fn has_oauth_credentials() -> bool {
    let has_client_id = client_id_entry()
        .ok()
        .and_then(|e| e.get_password().ok())
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    let has_client_secret = client_secret_entry()
        .ok()
        .and_then(|e| e.get_password().ok())
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    has_client_id && has_client_secret
}

/// OAuth認証情報を読み込み
///
/// # セキュリティ
/// client_id/client_secretはログに出力されません
pub fn load_oauth_credentials() -> Result<(String, String), String> {
    let client_id = client_id_entry()?
        .get_password()
        .map_err(|e| format!("Failed to load client_id from secure storage: {e}"))?;

    let client_secret = client_secret_entry()?
        .get_password()
        .map_err(|e| format!("Failed to load client_secret from secure storage: {e}"))?;

    if client_id.is_empty() {
        return Err("Gmail client_id is empty".to_string());
    }
    if client_secret.is_empty() {
        return Err("Gmail client_secret is empty".to_string());
    }

    log::info!("Gmail OAuth credentials loaded successfully from secure storage");
    Ok((client_id, client_secret))
}

/// OAuth認証情報を保存
///
/// # セキュリティ
/// client_id/client_secretはログに出力されません
pub fn save_oauth_credentials(client_id: &str, client_secret: &str) -> Result<(), String> {
    if client_id.is_empty() {
        return Err("Gmail client_id is empty".to_string());
    }
    if client_secret.is_empty() {
        return Err("Gmail client_secret is empty".to_string());
    }

    client_id_entry()?
        .set_password(client_id)
        .map_err(|e| format!("Failed to save client_id to secure storage: {e}"))?;

    client_secret_entry()?
        .set_password(client_secret)
        .map_err(|e| format!("Failed to save client_secret to secure storage: {e}"))?;

    log::info!("Gmail OAuth credentials saved successfully to secure storage");
    Ok(())
}

/// OAuth認証情報を削除
pub fn delete_oauth_credentials() -> Result<(), String> {
    // client_idの削除
    client_id_entry()?
        .delete_credential()
        .map_err(|e| format!("Failed to delete Gmail client_id from secure storage: {e}"))?;

    // client_secretの削除
    client_secret_entry()?
        .delete_credential()
        .map_err(|e| format!("Failed to delete Gmail client_secret from secure storage: {e}"))?;

    log::info!("Gmail OAuth credentials deleted successfully from secure storage");
    Ok(())
}

/// Google Cloud ConsoleからダウンロードしたJSONの構造
#[derive(Debug, Deserialize)]
struct ClientSecretJson {
    installed: Option<InstalledCredentials>,
    web: Option<WebCredentials>,
}

#[derive(Debug, Deserialize)]
struct InstalledCredentials {
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Deserialize)]
struct WebCredentials {
    client_id: String,
    client_secret: String,
}

/// client_secret.jsonの内容からOAuth認証情報を抽出して保存
///
/// Google Cloud Consoleからダウンロードした形式に対応:
/// - "installed" キー（デスクトップアプリ用）
/// - "web" キー（Webアプリ用）
pub fn save_oauth_credentials_from_json(json_content: &str) -> Result<(), String> {
    let parsed: ClientSecretJson =
        serde_json::from_str(json_content).map_err(|e| format!("Invalid JSON format: {e}"))?;

    // "installed" または "web" キーから認証情報を取得
    let (client_id, client_secret) = if let Some(installed) = parsed.installed {
        (installed.client_id, installed.client_secret)
    } else if let Some(web) = parsed.web {
        (web.client_id, web.client_secret)
    } else {
        return Err(
            "Invalid client_secret.json format: neither 'installed' nor 'web' key found"
                .to_string(),
        );
    };

    save_oauth_credentials(&client_id, &client_secret)
}

#[cfg(test)]
#[cfg(not(ci))]
mod tests {
    use super::*;
    use serial_test::serial;

    /// テスト用: keyring のエントリをクリーンアップ
    fn cleanup_test_keyring() {
        if let Ok(entry) = client_id_entry() {
            let _ = entry.delete_credential();
        }
        if let Ok(entry) = client_secret_entry() {
            let _ = entry.delete_credential();
        }
    }

    #[test]
    #[serial]
    fn test_has_oauth_credentials_returns_false_when_empty() {
        cleanup_test_keyring();
        assert!(!has_oauth_credentials());
    }

    #[test]
    #[serial]
    fn test_has_oauth_credentials_returns_true_when_set() {
        cleanup_test_keyring();
        save_oauth_credentials("test_client_id", "test_client_secret").unwrap();
        assert!(has_oauth_credentials());
        cleanup_test_keyring();
    }

    #[test]
    #[serial]
    fn test_save_and_load_oauth_credentials() {
        cleanup_test_keyring();

        let client_id = "my_test_client_id";
        let client_secret = "my_test_client_secret";

        let save_result = save_oauth_credentials(client_id, client_secret);
        assert!(save_result.is_ok());

        let load_result = load_oauth_credentials();
        assert!(load_result.is_ok());
        let (loaded_id, loaded_secret) = load_result.unwrap();
        assert_eq!(loaded_id, client_id);
        assert_eq!(loaded_secret, client_secret);

        cleanup_test_keyring();
    }

    #[test]
    #[serial]
    fn test_load_oauth_credentials_not_found() {
        cleanup_test_keyring();
        let result = load_oauth_credentials();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_lowercase().contains("failed"));
    }

    #[test]
    #[serial]
    fn test_delete_oauth_credentials() {
        cleanup_test_keyring();

        save_oauth_credentials("test_id", "test_secret").unwrap();
        assert!(has_oauth_credentials());

        let delete_result = delete_oauth_credentials();
        assert!(delete_result.is_ok());
        assert!(!has_oauth_credentials());
    }

    #[test]
    #[serial]
    fn test_save_oauth_credentials_from_json_installed() {
        cleanup_test_keyring();

        let json = r#"{
            "installed": {
                "client_id": "123456.apps.googleusercontent.com",
                "client_secret": "GOCSPX-secret123",
                "project_id": "test-project",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token"
            }
        }"#;

        let result = save_oauth_credentials_from_json(json);
        assert!(result.is_ok());

        let (id, secret) = load_oauth_credentials().unwrap();
        assert_eq!(id, "123456.apps.googleusercontent.com");
        assert_eq!(secret, "GOCSPX-secret123");

        cleanup_test_keyring();
    }

    #[test]
    #[serial]
    fn test_save_oauth_credentials_from_json_web() {
        cleanup_test_keyring();

        let json = r#"{
            "web": {
                "client_id": "web-client-id.apps.googleusercontent.com",
                "client_secret": "web-secret-456"
            }
        }"#;

        let result = save_oauth_credentials_from_json(json);
        assert!(result.is_ok());

        let (id, secret) = load_oauth_credentials().unwrap();
        assert_eq!(id, "web-client-id.apps.googleusercontent.com");
        assert_eq!(secret, "web-secret-456");

        cleanup_test_keyring();
    }

    #[test]
    #[serial]
    fn test_save_oauth_credentials_from_invalid_json() {
        cleanup_test_keyring();

        // 無効なJSON
        let result1 = save_oauth_credentials_from_json("not json");
        assert!(result1.is_err());
        assert!(result1.unwrap_err().contains("Invalid JSON"));

        // installedもwebもない
        let result2 = save_oauth_credentials_from_json(r#"{"other": {}}"#);
        assert!(result2.is_err());
        assert!(result2.unwrap_err().contains("neither"));

        cleanup_test_keyring();
    }

    #[test]
    #[serial]
    fn test_save_oauth_credentials_empty_values() {
        cleanup_test_keyring();

        let result1 = save_oauth_credentials("", "secret");
        assert!(result1.is_err());
        assert!(result1.unwrap_err().contains("client_id is empty"));

        let result2 = save_oauth_credentials("id", "");
        assert!(result2.is_err());
        assert!(result2.unwrap_err().contains("client_secret is empty"));

        cleanup_test_keyring();
    }
}
