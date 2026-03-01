//! バッチ処理の本体ロジック。コマンド・トレイメニュー両方から呼び出される。
//!
//! メインウィンドウを開かずにトレイから直接バッチを実行するため、
//! ロジックを共通関数として抽出している。
//!
//! ## モジュール構成
//!
//! - `error_handler`  – 共通エラーハンドリング (`ErrorReporter`)
//! - `sync_orchestrator` – Gmail同期オーケストレーション
//! - `parse_orchestrator` – メール解析オーケストレーション
//! - `product_parse_orchestrator` – 商品名解析オーケストレーション

mod delivery_check_orchestrator;
pub(crate) mod error_handler;
mod parse_orchestrator;
mod product_parse_orchestrator;
mod sync_orchestrator;

// — re-exports —
pub use delivery_check_orchestrator::run_delivery_check_task;
pub use parse_orchestrator::run_batch_parse_task;
pub use product_parse_orchestrator::run_product_name_parse_task;
pub use sync_orchestrator::run_sync_task;

use crate::batch_runner::BatchEventEmitter;
use crate::e2e_mocks::{is_e2e_mock_mode, E2EMockGmailClient, GmailClientForE2E};
use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

// ---------------------------------------------------------------------------
// BatchCommandsApp トレイト + Tauri 実装
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
pub(crate) trait BatchCommandsApp: BatchEventEmitter {
    fn notify(&self, title: &str, body: &str);
    fn app_config_dir(&self) -> Result<std::path::PathBuf, String>;
    fn app_data_dir(&self) -> Result<std::path::PathBuf, String>;
    async fn create_gmail_client(&self) -> Result<GmailClientForE2E, String>;
}

pub(crate) struct TauriBatchCommandsApp {
    pub app: tauri::AppHandle,
}

impl BatchEventEmitter for TauriBatchCommandsApp {
    fn emit_event<S: serde::Serialize + Clone>(&self, event: &str, payload: S) {
        let _ = self.app.emit(event, payload);
    }
}

#[async_trait::async_trait]
impl BatchCommandsApp for TauriBatchCommandsApp {
    fn notify(&self, title: &str, body: &str) {
        let _ = self
            .app
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show();
    }

    fn app_config_dir(&self) -> Result<std::path::PathBuf, String> {
        self.app
            .path()
            .app_config_dir()
            .map_err(|e| format!("Failed to get app config dir: {e}"))
    }

    fn app_data_dir(&self) -> Result<std::path::PathBuf, String> {
        self.app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {e}"))
    }

    async fn create_gmail_client(&self) -> Result<GmailClientForE2E, String> {
        if is_e2e_mock_mode() {
            log::info!("Using E2E mock Gmail client");
            return Ok(GmailClientForE2E::Mock(E2EMockGmailClient));
        }
        crate::gmail::GmailClient::new(&self.app)
            .await
            .map(|c| GmailClientForE2E::Real(Box::new(c)))
    }
}

// ---------------------------------------------------------------------------
// ユーティリティ
// ---------------------------------------------------------------------------

/// config.parse.batch_size (i64) を usize へ安全に変換。
/// 0 以下は default にフォールバック。変換失敗時（32-bit で i64 が大きい等）も default。
/// 上限はクランプしない（大きい i64 は usize::try_from で失敗→default）。
///
/// Note: バッチ処理モジュール内でのみ利用するユーティリティのため `pub(crate)` とし、
/// 上部の `pub use` で再エクスポートしないのは意図的な設計です。
pub(crate) fn clamp_batch_size(v: i64, default: usize) -> usize {
    if v <= 0 {
        default
    } else {
        usize::try_from(v).unwrap_or(default)
    }
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_batch_size() {
        // clamp_batch_size は本番コード（tray_parse, start_batch_parse）で使用
        assert_eq!(clamp_batch_size(0, 100), 100);
        assert_eq!(clamp_batch_size(-1, 100), 100);
        assert_eq!(clamp_batch_size(50, 100), 50);
        assert_eq!(clamp_batch_size(200, 100), 200);
        assert_eq!(clamp_batch_size(i64::MIN, 100), 100);
    }
}

/// テスト用ヘルパー。各オーケストレーターのテストモジュールから共有される。
#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use crate::e2e_mocks::E2EMockGmailClient;
    use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex as StdMutex;

    pub struct FakeApp {
        pub config_dir: std::path::PathBuf,
        pub data_dir: Option<std::path::PathBuf>,
        pub emitted_events: StdMutex<Vec<String>>,
        pub notify_count: AtomicUsize,
        pub fail_create_gmail_client: bool,
    }

    impl BatchEventEmitter for FakeApp {
        fn emit_event<S: serde::Serialize + Clone>(&self, event: &str, _payload: S) {
            self.emitted_events.lock().unwrap().push(event.to_string());
        }
    }

    #[async_trait::async_trait]
    impl BatchCommandsApp for FakeApp {
        fn notify(&self, _title: &str, _body: &str) {
            self.notify_count.fetch_add(1, Ordering::SeqCst);
        }

        fn app_config_dir(&self) -> Result<std::path::PathBuf, String> {
            Ok(self.config_dir.clone())
        }

        fn app_data_dir(&self) -> Result<std::path::PathBuf, String> {
            self.data_dir
                .clone()
                .ok_or_else(|| "Data dir not set".to_string())
        }

        async fn create_gmail_client(&self) -> Result<GmailClientForE2E, String> {
            if self.fail_create_gmail_client {
                return Err("boom".to_string());
            }
            // 常に E2E モック相当を返す（ネットワークや認証に依存しない）
            Ok(GmailClientForE2E::Mock(E2EMockGmailClient))
        }
    }

    pub async fn create_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap()
    }

    pub async fn create_shop_settings_table(pool: &SqlitePool) {
        sqlx::query(
            r#"
            CREATE TABLE shop_settings (
              id INTEGER PRIMARY KEY,
              shop_name TEXT NOT NULL,
              sender_address TEXT NOT NULL,
              parser_type TEXT NOT NULL,
              is_enabled INTEGER NOT NULL,
              subject_filters TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await
        .unwrap();
    }

    pub async fn insert_enabled_shop(pool: &SqlitePool) {
        sqlx::query(
            r#"
            INSERT INTO shop_settings (id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at)
            VALUES (1, 'TestShop', 'shop@example.com', 'hobbysearch_confirm', 1, NULL, '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')
            "#,
        )
        .execute(pool)
        .await
        .unwrap();
    }
}
