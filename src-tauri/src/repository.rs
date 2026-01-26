//! リポジトリパターンによるDB操作の抽象化
//!
//! このモジュールはデータベース操作を抽象化し、テスト時にモック可能にします。

use crate::gmail::{GmailMessage, ShopSettings, CreateShopSettings, UpdateShopSettings, SyncMetadata};
use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use sqlx::sqlite::SqlitePool;

/// メール関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait EmailRepository: Send + Sync {
    /// メッセージをDBに保存
    async fn save_messages(&self, messages: &[GmailMessage]) -> Result<(usize, usize), String>;

    /// 既存のメッセージIDを取得
    async fn get_existing_message_ids(&self) -> Result<Vec<String>, String>;

    /// メッセージ数を取得
    async fn get_message_count(&self) -> Result<i64, String>;

    /// 同期メタデータを取得
    async fn get_sync_metadata(&self) -> Result<SyncMetadata, String>;

    /// 同期メタデータを更新
    /// ライフタイム問題を回避するためにOption<String>を使用
    async fn update_sync_metadata(
        &self,
        oldest_date: Option<String>,
        total_synced: i64,
        status: String,
    ) -> Result<(), String>;

    /// 同期開始日時を更新
    async fn update_sync_started_at(&self) -> Result<(), String>;

    /// 同期完了日時を更新
    async fn update_sync_completed_at(&self) -> Result<(), String>;
}

/// ショップ設定関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait ShopSettingsRepository: Send + Sync {
    /// 全ショップ設定を取得
    async fn get_all(&self) -> Result<Vec<ShopSettings>, String>;

    /// 有効なショップ設定のみを取得
    async fn get_enabled(&self) -> Result<Vec<ShopSettings>, String>;

    /// ショップ設定を作成
    async fn create(&self, settings: CreateShopSettings) -> Result<ShopSettings, String>;

    /// ショップ設定を更新
    async fn update(&self, id: i64, settings: UpdateShopSettings) -> Result<ShopSettings, String>;

    /// ショップ設定を削除
    async fn delete(&self, id: i64) -> Result<(), String>;
}

/// SQLiteを使用したEmailRepositoryの実装
pub struct SqliteEmailRepository {
    pool: SqlitePool,
}

impl SqliteEmailRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EmailRepository for SqliteEmailRepository {
    async fn save_messages(&self, messages: &[GmailMessage]) -> Result<(usize, usize), String> {
        let mut saved = 0;
        let mut skipped = 0;

        // トランザクションを使用してバッチ処理
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin transaction: {e}"))?;

        for message in messages {
            // ON CONFLICT で重複をスキップ
            let result = sqlx::query(
                r#"
                INSERT INTO emails (message_id, body_plain, body_html, internal_date, from_address, subject)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(message_id) DO NOTHING
                "#,
            )
            .bind(&message.message_id)
            .bind(&message.body_plain)
            .bind(&message.body_html)
            .bind(message.internal_date)
            .bind(&message.from_address)
            .bind(&message.subject)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert message {}: {}", message.message_id, e))?;

            if result.rows_affected() > 0 {
                saved += 1;
            } else {
                skipped += 1;
            }
        }

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {e}"))?;

        Ok((saved, skipped))
    }

    async fn get_existing_message_ids(&self) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT message_id FROM emails")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("Failed to get existing message IDs: {e}"))?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    async fn get_message_count(&self) -> Result<i64, String> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM emails")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| format!("Failed to get message count: {e}"))?;

        Ok(count.0)
    }

    async fn get_sync_metadata(&self) -> Result<SyncMetadata, String> {
        let row: (String, Option<String>, i64, i64, Option<String>, Option<String>, i64) =
            sqlx::query_as(
                r#"
                SELECT
                    sync_status,
                    oldest_fetched_date,
                    total_synced_count,
                    batch_size,
                    last_sync_started_at,
                    last_sync_completed_at,
                    max_iterations
                FROM sync_metadata
                WHERE id = 1
                "#,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| format!("Failed to get sync metadata: {e}"))?;

        Ok(SyncMetadata {
            sync_status: row.0,
            oldest_fetched_date: row.1,
            total_synced_count: row.2,
            batch_size: row.3,
            last_sync_started_at: row.4,
            last_sync_completed_at: row.5,
            max_iterations: row.6,
        })
    }

    async fn update_sync_metadata(
        &self,
        oldest_date: Option<String>,
        total_synced: i64,
        status: String,
    ) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE sync_metadata
            SET oldest_fetched_date = COALESCE(?, oldest_fetched_date),
                total_synced_count = ?,
                sync_status = ?
            WHERE id = 1
            "#,
        )
        .bind(oldest_date)
        .bind(total_synced)
        .bind(&status)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update sync metadata: {e}"))?;

        Ok(())
    }

    async fn update_sync_started_at(&self) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE sync_metadata
            SET last_sync_started_at = datetime('now')
            WHERE id = 1
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update sync started at: {e}"))?;

        Ok(())
    }

    async fn update_sync_completed_at(&self) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE sync_metadata
            SET last_sync_completed_at = datetime('now')
            WHERE id = 1
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update sync completed at: {e}"))?;

        Ok(())
    }
}

/// SQLiteを使用したShopSettingsRepositoryの実装
pub struct SqliteShopSettingsRepository {
    pool: SqlitePool,
}

impl SqliteShopSettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ShopSettingsRepository for SqliteShopSettingsRepository {
    async fn get_all(&self) -> Result<Vec<ShopSettings>, String> {
        let settings = sqlx::query_as::<_, ShopSettings>(
            r#"
            SELECT id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at
            FROM shop_settings
            ORDER BY shop_name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get shop settings: {e}"))?;

        Ok(settings)
    }

    async fn get_enabled(&self) -> Result<Vec<ShopSettings>, String> {
        let settings = sqlx::query_as::<_, ShopSettings>(
            r#"
            SELECT id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at
            FROM shop_settings
            WHERE is_enabled = 1
            ORDER BY shop_name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get enabled shop settings: {e}"))?;

        Ok(settings)
    }

    async fn create(&self, settings: CreateShopSettings) -> Result<ShopSettings, String> {
        let subject_filters_json = settings
            .subject_filters
            .map(|filters| serde_json::to_string(&filters).unwrap_or_default());

        sqlx::query(
            r#"
            INSERT INTO shop_settings (shop_name, sender_address, parser_type, subject_filters)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&settings.shop_name)
        .bind(&settings.sender_address)
        .bind(&settings.parser_type)
        .bind(&subject_filters_json)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create shop setting: {e}"))?;

        // 作成したレコードを取得
        let created = sqlx::query_as::<_, ShopSettings>(
            r#"
            SELECT id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at
            FROM shop_settings
            WHERE shop_name = ? AND sender_address = ?
            ORDER BY id DESC
            LIMIT 1
            "#,
        )
        .bind(&settings.shop_name)
        .bind(&settings.sender_address)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to get created shop setting: {e}"))?;

        Ok(created)
    }

    async fn update(&self, id: i64, settings: UpdateShopSettings) -> Result<ShopSettings, String> {
        // 現在の設定を取得
        let current = sqlx::query_as::<_, ShopSettings>(
            "SELECT id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at FROM shop_settings WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get shop setting: {e}"))?
        .ok_or_else(|| format!("Shop setting with id {id} not found"))?;

        let subject_filters_json = settings
            .subject_filters
            .map(|filters| serde_json::to_string(&filters).unwrap_or_default());

        sqlx::query(
            r#"
            UPDATE shop_settings
            SET shop_name = COALESCE(?, shop_name),
                sender_address = COALESCE(?, sender_address),
                parser_type = COALESCE(?, parser_type),
                is_enabled = COALESCE(?, is_enabled),
                subject_filters = COALESCE(?, subject_filters),
                updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(settings.shop_name.as_ref().unwrap_or(&current.shop_name))
        .bind(settings.sender_address.as_ref().unwrap_or(&current.sender_address))
        .bind(settings.parser_type.as_ref().unwrap_or(&current.parser_type))
        .bind(settings.is_enabled.unwrap_or(current.is_enabled))
        .bind(subject_filters_json.as_ref().or(current.subject_filters.as_ref()))
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update shop setting: {e}"))?;

        // 更新後のレコードを取得
        let updated = sqlx::query_as::<_, ShopSettings>(
            "SELECT id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at FROM shop_settings WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to get updated shop setting: {e}"))?;

        Ok(updated)
    }

    async fn delete(&self, id: i64) -> Result<(), String> {
        sqlx::query("DELETE FROM shop_settings WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete shop setting: {e}"))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // テーブル作成
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS emails (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id TEXT UNIQUE NOT NULL,
                subject TEXT,
                body_plain TEXT,
                body_html TEXT,
                internal_date INTEGER NOT NULL,
                from_address TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create emails table");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sync_metadata (
                id INTEGER PRIMARY KEY,
                sync_status TEXT DEFAULT 'idle',
                oldest_fetched_date TEXT,
                total_synced_count INTEGER DEFAULT 0,
                batch_size INTEGER DEFAULT 50,
                last_sync_started_at TEXT,
                last_sync_completed_at TEXT,
                max_iterations INTEGER DEFAULT 10
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create sync_metadata table");

        sqlx::query("INSERT OR IGNORE INTO sync_metadata (id) VALUES (1)")
            .execute(&pool)
            .await
            .expect("Failed to insert default metadata");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS shop_settings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                shop_name TEXT NOT NULL,
                sender_address TEXT NOT NULL,
                parser_type TEXT NOT NULL,
                is_enabled INTEGER DEFAULT 1,
                subject_filters TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create shop_settings table");

        pool
    }

    #[tokio::test]
    async fn test_email_repository_save_and_get() {
        let pool = setup_test_db().await;
        let repo = SqliteEmailRepository::new(pool);

        let messages = vec![
            GmailMessage {
                message_id: "test123".to_string(),
                snippet: "Test snippet".to_string(),
                subject: Some("Test subject".to_string()),
                body_plain: Some("Plain body".to_string()),
                body_html: Some("<p>HTML body</p>".to_string()),
                internal_date: 1704067200000,
                from_address: Some("test@example.com".to_string()),
            },
            GmailMessage {
                message_id: "test456".to_string(),
                snippet: "Another snippet".to_string(),
                subject: Some("Another subject".to_string()),
                body_plain: None,
                body_html: None,
                internal_date: 1704153600000,
                from_address: None,
            },
        ];

        // 保存
        let (saved, skipped) = repo.save_messages(&messages).await.unwrap();
        assert_eq!(saved, 2);
        assert_eq!(skipped, 0);

        // 重複保存
        let (saved, skipped) = repo.save_messages(&messages).await.unwrap();
        assert_eq!(saved, 0);
        assert_eq!(skipped, 2);

        // カウント確認
        let count = repo.get_message_count().await.unwrap();
        assert_eq!(count, 2);

        // 既存ID取得
        let ids = repo.get_existing_message_ids().await.unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"test123".to_string()));
        assert!(ids.contains(&"test456".to_string()));
    }

    #[tokio::test]
    async fn test_sync_metadata_operations() {
        let pool = setup_test_db().await;
        let repo = SqliteEmailRepository::new(pool);

        // 初期値確認
        let metadata = repo.get_sync_metadata().await.unwrap();
        assert_eq!(metadata.sync_status, "idle");
        assert_eq!(metadata.total_synced_count, 0);

        // 更新
        repo.update_sync_metadata(Some("2024-01-01".to_string()), 100, "syncing".to_string())
            .await
            .unwrap();

        let metadata = repo.get_sync_metadata().await.unwrap();
        assert_eq!(metadata.sync_status, "syncing");
        assert_eq!(metadata.total_synced_count, 100);
        assert_eq!(metadata.oldest_fetched_date, Some("2024-01-01".to_string()));
    }

    #[tokio::test]
    async fn test_shop_settings_repository_crud() {
        let pool = setup_test_db().await;
        let repo = SqliteShopSettingsRepository::new(pool);

        // 作成
        let settings = CreateShopSettings {
            shop_name: "Test Shop".to_string(),
            sender_address: "shop@example.com".to_string(),
            parser_type: "hobbysearch_confirm".to_string(),
            subject_filters: Some(vec!["注文確認".to_string()]),
        };

        let created = repo.create(settings).await.unwrap();
        assert_eq!(created.shop_name, "Test Shop");
        assert!(created.is_enabled);

        // 全件取得
        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 1);

        // 更新
        let update = UpdateShopSettings {
            shop_name: Some("Updated Shop".to_string()),
            sender_address: None,
            parser_type: None,
            is_enabled: Some(false),
            subject_filters: None,
        };

        let updated = repo.update(created.id, update).await.unwrap();
        assert_eq!(updated.shop_name, "Updated Shop");
        assert!(!updated.is_enabled);

        // 有効なもののみ取得
        let enabled = repo.get_enabled().await.unwrap();
        assert_eq!(enabled.len(), 0);

        // 削除
        repo.delete(created.id).await.unwrap();
        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 0);
    }
}
