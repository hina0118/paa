use crate::gemini::ParsedProduct;
use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

/// ProductMaster エンティティ
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProductMaster {
    pub id: i64,
    pub raw_name: String,
    pub normalized_name: String,
    pub maker: Option<String>,
    pub series: Option<String>,
    pub product_name: Option<String>,
    pub scale: Option<String>,
    pub is_reissue: bool,
    pub platform_hint: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ProductMaster> for ParsedProduct {
    fn from(pm: ProductMaster) -> Self {
        ParsedProduct {
            maker: pm.maker,
            series: pm.series,
            name: pm.product_name.unwrap_or_else(|| pm.raw_name.clone()),
            scale: pm.scale,
            is_reissue: pm.is_reissue,
        }
    }
}

/// ProductMaster リポジトリトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait ProductMasterRepository: Send + Sync {
    /// raw_name でキャッシュ検索
    async fn find_by_raw_name(&self, raw_name: &str) -> Result<Option<ProductMaster>, String>;

    /// normalized_name でキャッシュ検索（類似検索用）
    async fn find_by_normalized_name(
        &self,
        normalized_name: &str,
    ) -> Result<Option<ProductMaster>, String>;

    /// 複数の raw_name で一括キャッシュ検索（N+1クエリ回避用）
    async fn find_by_raw_names(
        &self,
        raw_names: &[String],
    ) -> Result<std::collections::HashMap<String, ProductMaster>, String>;

    /// 複数の normalized_name で一括キャッシュ検索（N+1クエリ回避用）
    async fn find_by_normalized_names(
        &self,
        normalized_names: &[String],
    ) -> Result<std::collections::HashMap<String, ProductMaster>, String>;

    /// 新規保存
    /// Note: platform_hintはOption<String>を使用（mockallとの互換性のため）
    async fn save(
        &self,
        raw_name: &str,
        normalized_name: &str,
        parsed: &ParsedProduct,
        platform_hint: Option<String>,
    ) -> Result<i64, String>;

    /// 更新
    async fn update(&self, id: i64, parsed: &ParsedProduct) -> Result<(), String>;
}

/// SQLiteを使用したProductMasterRepositoryの実装
pub struct SqliteProductMasterRepository {
    pool: SqlitePool,
}

impl SqliteProductMasterRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProductMasterRepository for SqliteProductMasterRepository {
    async fn find_by_raw_name(&self, raw_name: &str) -> Result<Option<ProductMaster>, String> {
        sqlx::query_as::<_, ProductMaster>(
            r#"
            SELECT
                id,
                raw_name,
                normalized_name,
                maker,
                series,
                product_name,
                scale,
                is_reissue,
                platform_hint,
                created_at,
                updated_at
            FROM product_master
            WHERE raw_name = ?
            LIMIT 1
            "#,
        )
        .bind(raw_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to find product master by raw_name: {e}"))
    }

    async fn find_by_normalized_name(
        &self,
        normalized_name: &str,
    ) -> Result<Option<ProductMaster>, String> {
        sqlx::query_as::<_, ProductMaster>(
            r#"
            SELECT
                id,
                raw_name,
                normalized_name,
                maker,
                series,
                product_name,
                scale,
                is_reissue,
                platform_hint,
                created_at,
                updated_at
            FROM product_master
            WHERE normalized_name = ?
            LIMIT 1
            "#,
        )
        .bind(normalized_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to find product master by normalized_name: {e}"))
    }

    async fn find_by_raw_names(
        &self,
        raw_names: &[String],
    ) -> Result<std::collections::HashMap<String, ProductMaster>, String> {
        if raw_names.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        // SQLite にはバインド変数の上限（デフォルト999）があるため、チャンクで分割
        const MAX_PARAMS_PER_QUERY: usize = 900;
        let mut all_rows: Vec<ProductMaster> = Vec::new();

        for chunk in raw_names.chunks(MAX_PARAMS_PER_QUERY) {
            let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let sql = format!(
                r#"
                SELECT
                    id,
                    raw_name,
                    normalized_name,
                    maker,
                    series,
                    product_name,
                    scale,
                    is_reissue,
                    platform_hint,
                    created_at,
                    updated_at
                FROM product_master
                WHERE raw_name IN ({})
                "#,
                placeholders
            );
            let mut query = sqlx::query_as::<_, ProductMaster>(&sql);
            for name in chunk {
                query = query.bind(name);
            }
            let rows = query
                .fetch_all(&self.pool)
                .await
                .map_err(|e| format!("Failed to find product masters by raw_names: {e}"))?;
            all_rows.extend(rows);
        }
        Ok(all_rows
            .into_iter()
            .map(|r| (r.raw_name.clone(), r))
            .collect())
    }

    async fn find_by_normalized_names(
        &self,
        normalized_names: &[String],
    ) -> Result<std::collections::HashMap<String, ProductMaster>, String> {
        if normalized_names.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        // SQLite にはバインド変数の上限（デフォルト999）があるため、チャンクで分割
        const MAX_PARAMS_PER_QUERY: usize = 900;
        let mut all_rows: Vec<ProductMaster> = Vec::new();

        for chunk in normalized_names.chunks(MAX_PARAMS_PER_QUERY) {
            let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let sql = format!(
                r#"
                SELECT
                    id,
                    raw_name,
                    normalized_name,
                    maker,
                    series,
                    product_name,
                    scale,
                    is_reissue,
                    platform_hint,
                    created_at,
                    updated_at
                FROM product_master
                WHERE normalized_name IN ({})
                "#,
                placeholders
            );
            let mut query = sqlx::query_as::<_, ProductMaster>(&sql);
            for name in chunk {
                query = query.bind(name);
            }
            let rows = query
                .fetch_all(&self.pool)
                .await
                .map_err(|e| format!("Failed to find product masters by normalized_names: {e}"))?;
            all_rows.extend(rows);
        }
        Ok(all_rows
            .into_iter()
            .map(|r| (r.normalized_name.clone(), r))
            .collect())
    }

    async fn save(
        &self,
        raw_name: &str,
        normalized_name: &str,
        parsed: &ParsedProduct,
        platform_hint: Option<String>,
    ) -> Result<i64, String> {
        // Avoid logging user/product data (raw_name, maker, series, name); keep logs metrics-only.
        log::debug!("Saving product_master entry");

        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO product_master (
                raw_name,
                normalized_name,
                maker,
                series,
                product_name,
                scale,
                is_reissue,
                platform_hint
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(raw_name) DO UPDATE SET
                normalized_name = excluded.normalized_name,
                maker = excluded.maker,
                series = excluded.series,
                product_name = excluded.product_name,
                scale = excluded.scale,
                is_reissue = excluded.is_reissue,
                platform_hint = COALESCE(product_master.platform_hint, excluded.platform_hint)
            RETURNING id
            "#,
        )
        .bind(raw_name)
        .bind(normalized_name)
        .bind(&parsed.maker)
        .bind(&parsed.series)
        .bind(&parsed.name)
        .bind(&parsed.scale)
        .bind(parsed.is_reissue)
        .bind(&platform_hint)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            log::error!("Failed to save product master: {}", e);
            format!("Failed to save product master: {e}")
        })?;

        log::debug!("Successfully saved to product_master");
        Ok(id)
    }

    async fn update(&self, id: i64, parsed: &ParsedProduct) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE product_master
            SET
                maker = ?,
                series = ?,
                product_name = ?,
                scale = ?,
                is_reissue = ?
            WHERE id = ?
            "#,
        )
        .bind(&parsed.maker)
        .bind(&parsed.series)
        .bind(&parsed.name)
        .bind(&parsed.scale)
        .bind(parsed.is_reissue)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update product master: {e}"))?;

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
            CREATE TABLE IF NOT EXISTS product_master (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                raw_name TEXT UNIQUE NOT NULL,
                normalized_name TEXT NOT NULL,
                maker TEXT,
                series TEXT,
                product_name TEXT,
                scale TEXT,
                is_reissue INTEGER NOT NULL DEFAULT 0 CHECK(is_reissue IN (0, 1)),
                platform_hint TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create product_master table");

        pool
    }

    fn make_parsed_product(
        maker: Option<&str>,
        series: Option<&str>,
        name: &str,
        scale: Option<&str>,
        is_reissue: bool,
    ) -> ParsedProduct {
        ParsedProduct {
            maker: maker.map(String::from),
            series: series.map(String::from),
            name: name.to_string(),
            scale: scale.map(String::from),
            is_reissue,
        }
    }

    #[tokio::test]
    async fn test_product_master_repository_save_and_find_by_raw_name() {
        let pool = setup_test_db().await;
        let repo = SqliteProductMasterRepository::new(pool.clone());

        let parsed = make_parsed_product(
            Some("バンダイ"),
            Some("ガンダム"),
            "RG 1/144 ガンダム",
            Some("1/144"),
            false,
        );

        // 新規保存
        let id = repo
            .save(
                "RG 1/144 ガンダム",
                "rg1144gundam",
                &parsed,
                Some("hobbysearch".to_string()),
            )
            .await
            .unwrap();

        assert!(id > 0);

        // find_by_raw_name で取得
        let found = repo.find_by_raw_name("RG 1/144 ガンダム").await.unwrap();
        let pm = found.expect("should find by raw_name");
        assert_eq!(pm.raw_name, "RG 1/144 ガンダム");
        assert_eq!(pm.normalized_name, "rg1144gundam");
        assert_eq!(pm.maker, Some("バンダイ".to_string()));
        assert_eq!(pm.series, Some("ガンダム".to_string()));
        assert_eq!(pm.product_name, Some("RG 1/144 ガンダム".to_string()));
        assert_eq!(pm.scale, Some("1/144".to_string()));
        assert!(!pm.is_reissue);
        assert_eq!(pm.platform_hint, Some("hobbysearch".to_string()));
    }

    #[tokio::test]
    async fn test_product_master_repository_save_on_conflict_updates_normalized_name() {
        let pool = setup_test_db().await;
        let repo = SqliteProductMasterRepository::new(pool.clone());

        let parsed1 = make_parsed_product(
            Some("バンダイ"),
            Some("ガンダム"),
            "RG 1/144 ガンダム",
            Some("1/144"),
            false,
        );

        // 初回保存
        let id1 = repo
            .save("RG 1/144 ガンダム", "rg1144gundam", &parsed1, None)
            .await
            .unwrap();

        // 同じ raw_name で別の normalized_name を指定して保存（ON CONFLICT で更新）
        let parsed2 = make_parsed_product(
            Some("バンダイ"),
            Some("ガンダムユニット"),
            "RG 1/144 ガンダム（改）",
            Some("1/144"),
            false,
        );
        let id2 = repo
            .save("RG 1/144 ガンダム", "rg1144gundam2", &parsed2, None)
            .await
            .unwrap();

        // 同じIDが返る（UPDATEなので新規INSERTではない）
        assert_eq!(id1, id2);

        // 更新後の内容を確認
        let found = repo.find_by_raw_name("RG 1/144 ガンダム").await.unwrap();
        let pm = found.expect("should find");
        assert_eq!(pm.normalized_name, "rg1144gundam2");
        assert_eq!(pm.product_name, Some("RG 1/144 ガンダム（改）".to_string()));
    }

    #[tokio::test]
    async fn test_product_master_repository_find_by_normalized_name() {
        let pool = setup_test_db().await;
        let repo = SqliteProductMasterRepository::new(pool.clone());

        let parsed = make_parsed_product(
            Some("メガハウス"),
            Some("ポケモン"),
            "ピカチュウ フィギュア",
            None,
            false,
        );

        repo.save("ピカチュウ フィギュア", "pikachufigure", &parsed, None)
            .await
            .unwrap();

        let found = repo.find_by_normalized_name("pikachufigure").await.unwrap();
        let pm = found.expect("should find by normalized_name");
        assert_eq!(pm.raw_name, "ピカチュウ フィギュア");
        assert_eq!(pm.normalized_name, "pikachufigure");
    }

    #[tokio::test]
    async fn test_product_master_repository_find_by_raw_names() {
        let pool = setup_test_db().await;
        let repo = SqliteProductMasterRepository::new(pool.clone());

        let items = vec![
            (
                "商品A",
                "shohina",
                make_parsed_product(None, None, "商品A", None, false),
            ),
            (
                "商品B",
                "shohinb",
                make_parsed_product(None, None, "商品B", None, false),
            ),
        ];

        for (raw, norm, parsed) in &items {
            repo.save(raw, norm, parsed, None).await.unwrap();
        }

        let raw_names: Vec<String> = vec!["商品A".to_string(), "商品B".to_string()];
        let map = repo.find_by_raw_names(&raw_names).await.unwrap();

        assert_eq!(map.len(), 2);
        assert!(map.contains_key("商品A"));
        assert!(map.contains_key("商品B"));
        assert_eq!(map.get("商品A").unwrap().normalized_name, "shohina");
        assert_eq!(map.get("商品B").unwrap().normalized_name, "shohinb");
    }

    #[tokio::test]
    async fn test_product_master_repository_find_by_normalized_names() {
        let pool = setup_test_db().await;
        let repo = SqliteProductMasterRepository::new(pool.clone());

        let items = vec![
            (
                "商品A",
                "shohina",
                make_parsed_product(None, None, "商品A", None, false),
            ),
            (
                "商品B",
                "shohinb",
                make_parsed_product(None, None, "商品B", None, false),
            ),
        ];

        for (raw, norm, parsed) in &items {
            repo.save(raw, norm, parsed, None).await.unwrap();
        }

        let norm_names: Vec<String> = vec!["shohina".to_string(), "shohinb".to_string()];
        let map = repo.find_by_normalized_names(&norm_names).await.unwrap();

        assert_eq!(map.len(), 2);
        assert!(map.contains_key("shohina"));
        assert!(map.contains_key("shohinb"));
        assert_eq!(map.get("shohina").unwrap().raw_name, "商品A");
        assert_eq!(map.get("shohinb").unwrap().raw_name, "商品B");
    }

    #[tokio::test]
    async fn test_product_master_repository_update() {
        let pool = setup_test_db().await;
        let repo = SqliteProductMasterRepository::new(pool.clone());

        let parsed = make_parsed_product(
            Some("メーカーA"),
            Some("シリーズA"),
            "商品名A",
            Some("1/100"),
            false,
        );

        let id = repo
            .save("商品名A", "shohinmeia", &parsed, None)
            .await
            .unwrap();

        let updated = make_parsed_product(
            Some("メーカーB"),
            Some("シリーズB"),
            "商品名B",
            Some("1/144"),
            true,
        );

        repo.update(id, &updated).await.unwrap();

        let found = repo.find_by_raw_name("商品名A").await.unwrap();
        let pm = found.expect("should find");
        assert_eq!(pm.maker, Some("メーカーB".to_string()));
        assert_eq!(pm.series, Some("シリーズB".to_string()));
        assert_eq!(pm.product_name, Some("商品名B".to_string()));
        assert_eq!(pm.scale, Some("1/144".to_string()));
        assert!(pm.is_reissue);
    }
}
