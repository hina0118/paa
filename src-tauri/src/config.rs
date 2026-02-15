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
    #[serde(default)]
    pub gemini: GeminiConfig,
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
    /// Gmail API の1ページあたり取得件数（最大500）
    #[serde(default = "default_max_results_per_page")]
    pub max_results_per_page: i64,
    /// 同期処理のタイムアウト（分）
    #[serde(default = "default_sync_timeout_minutes")]
    pub timeout_minutes: i64,
}

fn default_max_results_per_page() -> i64 {
    100
}

fn default_sync_timeout_minutes() -> i64 {
    30
}

/// Gemini API（商品名パース）設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiConfig {
    /// 1リクエストあたりの商品数
    #[serde(default = "default_gemini_batch_size")]
    pub batch_size: i64,
    /// リクエスト間の待機秒数（レート制限対策）
    #[serde(default = "default_gemini_delay_seconds")]
    pub delay_seconds: i64,
}

fn default_gemini_batch_size() -> i64 {
    10
}

fn default_gemini_delay_seconds() -> i64 {
    10
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            delay_seconds: 10,
        }
    }
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
                max_results_per_page: 100,
                timeout_minutes: 30,
            },
            parse: ParseConfig { batch_size: 100 },
            window: WindowConfig::default(),
            gemini: GeminiConfig::default(),
        }
    }
}

/// 設定を読み込む。ファイルが存在しない場合はデフォルトを返し、保存する。
pub fn load(config_dir: &Path) -> Result<AppConfig, String> {
    let path = config_dir.join(CONFIG_FILENAME);

    if path.exists() {
        let contents =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read config file: {e}"))?;
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
        assert_eq!(config.sync.max_results_per_page, 100);
        assert_eq!(config.sync.timeout_minutes, 30);
        assert_eq!(config.parse.batch_size, 100);
        assert_eq!(config.window.width, 800);
        assert_eq!(config.window.height, 600);
        assert_eq!(config.gemini.batch_size, 10);
        assert_eq!(config.gemini.delay_seconds, 10);

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
                max_results_per_page: 200,
                timeout_minutes: 60,
            },
            parse: ParseConfig { batch_size: 200 },
            window: WindowConfig {
                width: 1024,
                height: 768,
                x: Some(100),
                y: Some(200),
                maximized: true,
            },
            gemini: GeminiConfig {
                batch_size: 20,
                delay_seconds: 5,
            },
        };

        save(dir.path(), &config).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded.sync.batch_size, 100);
        assert_eq!(loaded.sync.max_iterations, 500);
        assert_eq!(loaded.sync.max_results_per_page, 200);
        assert_eq!(loaded.sync.timeout_minutes, 60);
        assert_eq!(loaded.parse.batch_size, 200);
        assert_eq!(loaded.window.width, 1024);
        assert!(loaded.window.maximized);
        assert_eq!(loaded.gemini.batch_size, 20);
        assert_eq!(loaded.gemini.delay_seconds, 5);
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

    #[test]
    fn test_load_applies_field_defaults_when_missing_in_json() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(CONFIG_FILENAME);

        // sync.max_results_per_page / sync.timeout_minutes を省略 → #[serde(default = ...)] が呼ばれる
        // gemini を空オブジェクトで渡し、gemini.* の default 関数も呼ばれる
        let json = r#"
        {
          "sync": { "batch_size": 12, "max_iterations": 34 },
          "parse": { "batch_size": 56 },
          "gemini": {}
        }
        "#;
        fs::write(&path, json).unwrap();

        let loaded = load(dir.path()).unwrap();

        // JSON で指定した値を検証
        assert_eq!(loaded.sync.batch_size, 12);
        assert_eq!(loaded.sync.max_iterations, 34);
        assert_eq!(loaded.parse.batch_size, 56);

        // デフォルト値から取得した値と比較（serde の #[serde(default)] 適用元と揃える）
        assert_eq!(loaded.sync.max_results_per_page, default_max_results_per_page());
        assert_eq!(loaded.sync.timeout_minutes, default_sync_timeout_minutes());
        let default_gemini = GeminiConfig::default();
        assert_eq!(loaded.gemini.batch_size, default_gemini.batch_size);
        assert_eq!(loaded.gemini.delay_seconds, default_gemini.delay_seconds);

        // window は JSON から省略 → AppConfig の #[serde(default)] で WindowConfig::default
        let default_window = WindowConfig::default();
        assert_eq!(loaded.window.width, default_window.width);
        assert_eq!(loaded.window.height, default_window.height);
        assert_eq!(loaded.window.maximized, default_window.maximized);
    }

    #[test]
    fn test_load_applies_serde_defaults_when_window_and_gemini_missing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(CONFIG_FILENAME);

        // window / gemini を省略しても読み込める（AppConfig の #[serde(default)]）
        let json = r#"
        {
          "sync": { "batch_size": 1, "max_iterations": 2, "max_results_per_page": 3, "timeout_minutes": 4 },
          "parse": { "batch_size": 5 }
        }
        "#;
        fs::write(&path, json).unwrap();

        let loaded = load(dir.path()).unwrap();

        // JSON で指定した値を検証
        assert_eq!(loaded.sync.max_results_per_page, 3);
        assert_eq!(loaded.sync.timeout_minutes, 4);

        // デフォルト値から取得した値と比較（serde の #[serde(default)] 適用元と揃える）
        let default_window = WindowConfig::default();
        let default_gemini = GeminiConfig::default();
        assert_eq!(loaded.window.width, default_window.width);
        assert_eq!(loaded.gemini.batch_size, default_gemini.batch_size);
    }
}
