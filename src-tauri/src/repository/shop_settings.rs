use crate::gmail::{CreateShopSettings, ShopSettings, UpdateShopSettings};
use crate::plugins::DefaultShopSetting;
use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use sqlx::sqlite::SqlitePool;

/// ショップ設定関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait ShopSettingsRepository: Send + Sync {
    /// 全ショップ設定を取得
    async fn get_all(&self) -> Result<Vec<ShopSettings>, String>;

    /// 有効なショップ設定のみを取得（ORDER BY shop_name, id で返す。parsers が試行順序に依存）
    async fn get_enabled(&self) -> Result<Vec<ShopSettings>, String>;

    /// ショップ設定を作成
    async fn create(&self, settings: CreateShopSettings) -> Result<ShopSettings, String>;

    /// ショップ設定を更新
    async fn update(&self, id: i64, settings: UpdateShopSettings) -> Result<ShopSettings, String>;

    /// ショップ設定を削除
    async fn delete(&self, id: i64) -> Result<(), String>;

    /// (sender_address, parser_type) が未登録の場合のみ挿入する（冪等）
    ///
    /// `ensure_default_settings()` から呼び出され、アプリ起動時にデフォルト設定を自動登録する。
    async fn insert_if_not_exists(&self, setting: &DefaultShopSetting) -> Result<(), String>;
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
            -- バッチ処理（例: batch_parse_emails）のパーサ試行順序を shop_name, id で一意に決めているため、この並び順は変更しないこと
            ORDER BY shop_name, id
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
            .map(|filters| serde_json::to_string(&filters))
            .transpose()
            .map_err(|e| format!("Failed to serialize subject filters: {e}"))?;

        let result = sqlx::query(
            r#"
            INSERT INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&settings.shop_name)
        .bind(&settings.sender_address)
        .bind(&settings.parser_type)
        .bind(&subject_filters_json)
        .bind(1) // 新規作成時は有効化しておく（DBデフォルトには依存しない）
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create shop setting: {e}"))?;

        // last_insert_rowid()を使用して作成したレコードを取得
        let inserted_id = result.last_insert_rowid();
        let created = sqlx::query_as::<_, ShopSettings>(
            r#"
            SELECT id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at
            FROM shop_settings
            WHERE id = ?
            "#,
        )
        .bind(inserted_id)
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
            .map(|filters| serde_json::to_string(&filters))
            .transpose()
            .map_err(|e| format!("Failed to serialize subject filters: {e}"))?;

        // COALESCEは不要（unwrap_or/orで既にフォールバック済み）
        sqlx::query(
            r#"
            UPDATE shop_settings
            SET shop_name = ?,
                sender_address = ?,
                parser_type = ?,
                is_enabled = ?,
                subject_filters = ?,
                updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(settings.shop_name.as_ref().unwrap_or(&current.shop_name))
        .bind(
            settings
                .sender_address
                .as_ref()
                .unwrap_or(&current.sender_address),
        )
        .bind(
            settings
                .parser_type
                .as_ref()
                .unwrap_or(&current.parser_type),
        )
        .bind(settings.is_enabled.unwrap_or(current.is_enabled))
        .bind(
            subject_filters_json
                .as_ref()
                .or(current.subject_filters.as_ref()),
        )
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

    async fn insert_if_not_exists(&self, setting: &DefaultShopSetting) -> Result<(), String> {
        let subject_filters_json = setting
            .subject_filters
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| format!("Failed to serialize subject filters: {e}"))?;

        sqlx::query(
            r#"
            INSERT OR IGNORE INTO shop_settings
                (shop_name, sender_address, parser_type, subject_filters, is_enabled)
            VALUES (?, ?, ?, ?, 1)
            "#,
        )
        .bind(&setting.shop_name)
        .bind(&setting.sender_address)
        .bind(&setting.parser_type)
        .bind(&subject_filters_json)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to insert shop setting: {e}"))?;

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

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS shop_settings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                shop_name TEXT NOT NULL,
                sender_address TEXT NOT NULL,
                parser_type TEXT NOT NULL,
                is_enabled INTEGER NOT NULL DEFAULT 1 CHECK(is_enabled IN (0, 1)),
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                subject_filters TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create shop_settings table");

        pool
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
