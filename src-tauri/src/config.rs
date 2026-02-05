//! アプリケーション設定ファイルの管理
//!
//! sync/parse の batch_size, max_iterations を paa_config.json で管理する。
//! 状態・進捗は DB テーブル、設定はこのファイルに分離する。

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const CONFIG_FILENAME: &str = "paa_config.json";

/// アプリケーション設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub sync: SyncConfig,
    pub parse: ParseConfig,
    #[serde(default)]
    pub window: WindowConfig,
}

/// ウィンドウ設定（サイズ・位置・最大化状態）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub width: i64,
    pub height: i64,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub maximized: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            x: None,
            y: None,
            maximized: false,
        }
    }
}

/// 同期（Gmail）設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub batch_size: i64,
    pub max_iterations: i64,
}

/// パース設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseConfig {
    pub batch_size: i64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            sync: SyncConfig {
                batch_size: 50,
                max_iterations: 1000,
            },
            parse: ParseConfig {
                batch_size: 100,
            },
            window: WindowConfig::default(),
        }
    }
}

/// 設定を読み込む。ファイルが存在しない場合はデフォルトを返し、保存する。
pub fn load(config_dir: &Path) -> Result<AppConfig, String> {
    let path = config_dir.join(CONFIG_FILENAME);

    if path.exists() {
        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read config file: {e}"))?;
        serde_json::from_str(&contents).map_err(|e| format!("Invalid config JSON: {e}"))
    } else {
        let config = AppConfig::default();
        save(config_dir, &config)?;
        Ok(config)
    }
}

/// 設定を保存する。
pub fn save(config_dir: &Path, config: &AppConfig) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| format!("Failed to create config dir: {e}"))?;

    let path = config_dir.join(CONFIG_FILENAME);
    let contents = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;

    fs::write(&path, contents).map_err(|e| format!("Failed to write config file: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_creates_default_when_missing() {
        let dir = TempDir::new().unwrap();
        let config = load(dir.path()).unwrap();
        assert_eq!(config.sync.batch_size, 50);
        assert_eq!(config.sync.max_iterations, 1000);
        assert_eq!(config.parse.batch_size, 100);
        assert_eq!(config.window.width, 800);
        assert_eq!(config.window.height, 600);

        // ファイルが作成されている
        assert!(dir.path().join(CONFIG_FILENAME).exists());
    }

    #[test]
    fn test_save_and_load() {
        let dir = TempDir::new().unwrap();
        let config = AppConfig {
            sync: SyncConfig {
                batch_size: 100,
                max_iterations: 500,
            },
            parse: ParseConfig { batch_size: 200 },
            window: WindowConfig {
                width: 1024,
                height: 768,
                x: Some(100),
                y: Some(200),
                maximized: true,
            },
        };

        save(dir.path(), &config).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded.sync.batch_size, 100);
        assert_eq!(loaded.sync.max_iterations, 500);
        assert_eq!(loaded.parse.batch_size, 200);
        assert_eq!(loaded.window.width, 1024);
        assert_eq!(loaded.window.maximized, true);
    }

    #[test]
    fn test_load_invalid_json_returns_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(CONFIG_FILENAME);
        fs::write(&path, "invalid json").unwrap();

        let result = load(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid config"));
    }
}
