use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

// NOTE: Clippy (type_complexity) 対応
// `sqlx::query_as` で使用する巨大タプル型を type alias にして可読性を保つ。
type ItemOverrideDbRow = (
    i64,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<i64>,
    Option<i64>,
    Option<String>,
    Option<String>,
    String,
    String,
);
type OrderOverrideDbRow = (
    i64,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
);
type ExcludedItemDbRow = (i64, String, String, String, String, Option<String>, String);

/// アイテム上書き保存パラメータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveItemOverride {
    pub shop_domain: String,
    pub order_number: String,
    pub original_item_name: String,
    pub original_brand: String,
    pub item_name: Option<String>,
    pub price: Option<i64>,
    pub quantity: Option<i64>,
    pub brand: Option<String>,
    pub category: Option<String>,
}

/// アイテム上書きレコード
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemOverride {
    pub id: i64,
    pub shop_domain: String,
    pub order_number: String,
    pub original_item_name: String,
    pub original_brand: String,
    pub item_name: Option<String>,
    pub price: Option<i64>,
    pub quantity: Option<i64>,
    pub brand: Option<String>,
    pub category: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 注文上書き保存パラメータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveOrderOverride {
    pub shop_domain: String,
    pub order_number: String,
    pub new_order_number: Option<String>,
    pub order_date: Option<String>,
    pub shop_name: Option<String>,
}

/// 注文上書きレコード
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderOverride {
    pub id: i64,
    pub shop_domain: String,
    pub order_number: String,
    pub new_order_number: Option<String>,
    pub order_date: Option<String>,
    pub shop_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// アイテム除外パラメータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludeItemParams {
    pub shop_domain: String,
    pub order_number: String,
    pub item_name: String,
    pub brand: String,
    pub reason: Option<String>,
}

/// 除外アイテムレコード
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludedItem {
    pub id: i64,
    pub shop_domain: String,
    pub order_number: String,
    pub item_name: String,
    pub brand: String,
    pub reason: Option<String>,
    pub created_at: String,
}

/// 注文除外パラメータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludeOrderParams {
    pub shop_domain: String,
    pub order_number: String,
    pub reason: Option<String>,
}

/// 除外注文レコード
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludedOrder {
    pub id: i64,
    pub shop_domain: String,
    pub order_number: String,
    pub reason: Option<String>,
    pub created_at: String,
}

/// 手動上書き・除外のDB操作
pub struct SqliteOverrideRepository {
    pool: SqlitePool,
}

impl SqliteOverrideRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // ─── アイテム上書き ─────────────────────

    pub async fn save_item_override(&self, params: SaveItemOverride) -> Result<i64, String> {
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO item_overrides (shop_domain, order_number, original_item_name, original_brand,
                                        item_name, price, quantity, brand, category)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (shop_domain, order_number, original_item_name, original_brand)
            DO UPDATE SET
                item_name = excluded.item_name,
                price = excluded.price,
                quantity = excluded.quantity,
                brand = excluded.brand,
                category = excluded.category
            RETURNING id
            "#,
        )
        .bind(&params.shop_domain)
        .bind(&params.order_number)
        .bind(&params.original_item_name)
        .bind(&params.original_brand)
        .bind(&params.item_name)
        .bind(params.price)
        .bind(params.quantity)
        .bind(&params.brand)
        .bind(&params.category)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to save item override: {e}"))?;

        Ok(id)
    }

    pub async fn delete_item_override(&self, id: i64) -> Result<(), String> {
        sqlx::query("DELETE FROM item_overrides WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete item override: {e}"))?;
        Ok(())
    }

    pub async fn delete_item_override_by_key(
        &self,
        shop_domain: &str,
        order_number: &str,
        original_item_name: &str,
        original_brand: &str,
    ) -> Result<(), String> {
        sqlx::query(
            r#"
            DELETE FROM item_overrides
            WHERE shop_domain = ?
              AND order_number = ?
              AND original_item_name = ?
              AND original_brand = ?
            "#,
        )
        .bind(shop_domain)
        .bind(order_number)
        .bind(original_item_name)
        .bind(original_brand)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to delete item override by key: {e}"))?;
        Ok(())
    }

    pub async fn get_all_item_overrides(&self) -> Result<Vec<ItemOverride>, String> {
        let rows: Vec<ItemOverrideDbRow> = sqlx::query_as(
            r#"
                SELECT id, shop_domain, order_number, original_item_name, original_brand,
                       item_name, price, quantity, brand, category,
                       created_at, updated_at
                FROM item_overrides
                ORDER BY updated_at DESC
                "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch item overrides: {e}"))?;

        Ok(rows
            .into_iter()
            .map(|r| ItemOverride {
                id: r.0,
                shop_domain: r.1,
                order_number: r.2,
                original_item_name: r.3,
                original_brand: r.4,
                item_name: r.5,
                price: r.6,
                quantity: r.7,
                brand: r.8,
                category: r.9,
                created_at: r.10,
                updated_at: r.11,
            })
            .collect())
    }

    // ─── 注文上書き ─────────────────────

    pub async fn save_order_override(&self, params: SaveOrderOverride) -> Result<i64, String> {
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO order_overrides (shop_domain, order_number, new_order_number, order_date, shop_name)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (shop_domain, order_number)
            DO UPDATE SET
                new_order_number = excluded.new_order_number,
                order_date = excluded.order_date,
                shop_name = excluded.shop_name
            RETURNING id
            "#,
        )
        .bind(&params.shop_domain)
        .bind(&params.order_number)
        .bind(&params.new_order_number)
        .bind(&params.order_date)
        .bind(&params.shop_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to save order override: {e}"))?;

        Ok(id)
    }

    pub async fn delete_order_override(&self, id: i64) -> Result<(), String> {
        sqlx::query("DELETE FROM order_overrides WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete order override: {e}"))?;
        Ok(())
    }

    pub async fn delete_order_override_by_key(
        &self,
        shop_domain: &str,
        order_number: &str,
    ) -> Result<(), String> {
        sqlx::query(
            r#"
            DELETE FROM order_overrides
            WHERE shop_domain = ?
              AND order_number = ?
            "#,
        )
        .bind(shop_domain)
        .bind(order_number)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to delete order override by key: {e}"))?;
        Ok(())
    }

    pub async fn get_all_order_overrides(&self) -> Result<Vec<OrderOverride>, String> {
        let rows: Vec<OrderOverrideDbRow> = sqlx::query_as(
            r#"
                SELECT id, shop_domain, order_number, new_order_number, order_date, shop_name,
                       created_at, updated_at
                FROM order_overrides
                ORDER BY updated_at DESC
                "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch order overrides: {e}"))?;

        Ok(rows
            .into_iter()
            .map(|r| OrderOverride {
                id: r.0,
                shop_domain: r.1,
                order_number: r.2,
                new_order_number: r.3,
                order_date: r.4,
                shop_name: r.5,
                created_at: r.6,
                updated_at: r.7,
            })
            .collect())
    }

    // ─── アイテム除外 ─────────────────────

    pub async fn exclude_item(&self, params: ExcludeItemParams) -> Result<i64, String> {
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO excluded_items (shop_domain, order_number, item_name, brand, reason)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (shop_domain, order_number, item_name, brand)
            DO UPDATE SET reason = excluded.reason
            RETURNING id
            "#,
        )
        .bind(&params.shop_domain)
        .bind(&params.order_number)
        .bind(&params.item_name)
        .bind(&params.brand)
        .bind(&params.reason)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to exclude item: {e}"))?;

        // 論理削除: items テーブルからは削除しない。表示クエリ側で除外リストを参照して非表示にする。

        Ok(id)
    }

    pub async fn restore_excluded_item(&self, id: i64) -> Result<(), String> {
        sqlx::query("DELETE FROM excluded_items WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to restore excluded item: {e}"))?;
        Ok(())
    }

    pub async fn get_all_excluded_items(&self) -> Result<Vec<ExcludedItem>, String> {
        let rows: Vec<ExcludedItemDbRow> = sqlx::query_as(
            r#"
                SELECT id, shop_domain, order_number, item_name, brand, reason, created_at
                FROM excluded_items
                ORDER BY created_at DESC
                "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch excluded items: {e}"))?;

        Ok(rows
            .into_iter()
            .map(|r| ExcludedItem {
                id: r.0,
                shop_domain: r.1,
                order_number: r.2,
                item_name: r.3,
                brand: r.4,
                reason: r.5,
                created_at: r.6,
            })
            .collect())
    }

    // ─── 注文除外 ─────────────────────

    pub async fn exclude_order(&self, params: ExcludeOrderParams) -> Result<i64, String> {
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO excluded_orders (shop_domain, order_number, reason)
            VALUES (?, ?, ?)
            ON CONFLICT (shop_domain, order_number)
            DO UPDATE SET reason = excluded.reason
            RETURNING id
            "#,
        )
        .bind(&params.shop_domain)
        .bind(&params.order_number)
        .bind(&params.reason)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to exclude order: {e}"))?;

        // 論理削除: orders テーブルからは削除しない。表示クエリ側で除外リストを参照して非表示にする。

        Ok(id)
    }

    pub async fn restore_excluded_order(&self, id: i64) -> Result<(), String> {
        sqlx::query("DELETE FROM excluded_orders WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to restore excluded order: {e}"))?;
        Ok(())
    }

    pub async fn get_all_excluded_orders(&self) -> Result<Vec<ExcludedOrder>, String> {
        let rows: Vec<(i64, String, String, Option<String>, String)> = sqlx::query_as(
            r#"
            SELECT id, shop_domain, order_number, reason, created_at
            FROM excluded_orders
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch excluded orders: {e}"))?;

        Ok(rows
            .into_iter()
            .map(|r| ExcludedOrder {
                id: r.0,
                shop_domain: r.1,
                order_number: r.2,
                reason: r.3,
                created_at: r.4,
            })
            .collect())
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
            CREATE TABLE IF NOT EXISTS item_overrides (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                shop_domain TEXT NOT NULL,
                order_number TEXT NOT NULL COLLATE NOCASE,
                original_item_name TEXT NOT NULL,
                original_brand TEXT NOT NULL DEFAULT '',
                item_name TEXT,
                price INTEGER,
                quantity INTEGER,
                brand TEXT,
                category TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (shop_domain, order_number, original_item_name, original_brand)
            );
            CREATE INDEX IF NOT EXISTS idx_item_overrides_key
            ON item_overrides(shop_domain, order_number, original_item_name, original_brand);
            CREATE TRIGGER IF NOT EXISTS item_overrides_updated_at AFTER UPDATE ON item_overrides BEGIN
                UPDATE item_overrides SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
            END;

            CREATE TABLE IF NOT EXISTS order_overrides (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                shop_domain TEXT NOT NULL,
                order_number TEXT NOT NULL COLLATE NOCASE,
                new_order_number TEXT,
                order_date TEXT,
                shop_name TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (shop_domain, order_number)
            );
            CREATE INDEX IF NOT EXISTS idx_order_overrides_key
            ON order_overrides(shop_domain, order_number);
            CREATE TRIGGER IF NOT EXISTS order_overrides_updated_at AFTER UPDATE ON order_overrides BEGIN
                UPDATE order_overrides SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
            END;

            CREATE TABLE IF NOT EXISTS excluded_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                shop_domain TEXT NOT NULL,
                order_number TEXT NOT NULL COLLATE NOCASE,
                item_name TEXT NOT NULL,
                brand TEXT NOT NULL DEFAULT '',
                reason TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (shop_domain, order_number, item_name, brand)
            );
            CREATE INDEX IF NOT EXISTS idx_excluded_items_key
            ON excluded_items(shop_domain, order_number, item_name, brand);

            CREATE TABLE IF NOT EXISTS excluded_orders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                shop_domain TEXT NOT NULL,
                order_number TEXT NOT NULL COLLATE NOCASE,
                reason TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (shop_domain, order_number)
            );
            CREATE INDEX IF NOT EXISTS idx_excluded_orders_key
            ON excluded_orders(shop_domain, order_number);
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create manual override/exclusion tables");

        pool
    }

    #[tokio::test]
    async fn test_override_repository_save_item_override_upsert() {
        let pool = setup_test_db().await;
        let repo = SqliteOverrideRepository::new(pool.clone());

        let id1 = repo
            .save_item_override(SaveItemOverride {
                shop_domain: "1999.co.jp".to_string(),
                order_number: "ORD-001".to_string(),
                original_item_name: "商品A".to_string(),
                original_brand: "".to_string(),
                item_name: Some("商品A(修正)".to_string()),
                price: Some(2000),
                quantity: Some(2),
                brand: None,
                category: None,
            })
            .await
            .expect("save_item_override (insert)");

        let id2 = repo
            .save_item_override(SaveItemOverride {
                shop_domain: "1999.co.jp".to_string(),
                order_number: "ORD-001".to_string(),
                original_item_name: "商品A".to_string(),
                original_brand: "".to_string(),
                item_name: Some("商品A(再修正)".to_string()),
                price: Some(2100),
                quantity: Some(3),
                brand: Some("BrandX".to_string()),
                category: Some("CatY".to_string()),
            })
            .await
            .expect("save_item_override (update)");

        // 同一キーなら行は増えない & id は維持される
        assert_eq!(id1, id2);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM item_overrides")
            .fetch_one(&pool)
            .await
            .expect("count item_overrides");
        assert_eq!(count, 1);

        let row: (String, i64, i64, String, String) = sqlx::query_as(
            "SELECT item_name, price, quantity, brand, category FROM item_overrides WHERE id = ?",
        )
        .bind(id1)
        .fetch_one(&pool)
        .await
        .expect("fetch item_overrides row");
        assert_eq!(row.0, "商品A(再修正)");
        assert_eq!(row.1, 2100);
        assert_eq!(row.2, 3);
        assert_eq!(row.3, "BrandX");
        assert_eq!(row.4, "CatY");
    }

    #[tokio::test]
    async fn test_override_repository_delete_item_override_by_key() {
        let pool = setup_test_db().await;
        let repo = SqliteOverrideRepository::new(pool.clone());

        let _id = repo
            .save_item_override(SaveItemOverride {
                shop_domain: "1999.co.jp".to_string(),
                order_number: "ORD-DEL".to_string(),
                original_item_name: "商品DEL".to_string(),
                original_brand: "".to_string(),
                item_name: Some("商品DEL(修正)".to_string()),
                price: None,
                quantity: None,
                brand: None,
                category: None,
            })
            .await
            .expect("save_item_override");

        let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM item_overrides")
            .fetch_one(&pool)
            .await
            .expect("count item_overrides (before)");
        assert_eq!(before, 1);

        repo.delete_item_override_by_key("1999.co.jp", "ORD-DEL", "商品DEL", "")
            .await
            .expect("delete_item_override_by_key");

        let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM item_overrides")
            .fetch_one(&pool)
            .await
            .expect("count item_overrides (after)");
        assert_eq!(after, 0);
    }

    #[tokio::test]
    async fn test_override_repository_save_order_override_upsert() {
        let pool = setup_test_db().await;
        let repo = SqliteOverrideRepository::new(pool.clone());

        let id1 = repo
            .save_order_override(SaveOrderOverride {
                shop_domain: "1999.co.jp".to_string(),
                order_number: "ORD-002".to_string(),
                new_order_number: Some("ORD-002A".to_string()),
                order_date: Some("2024-02-01".to_string()),
                shop_name: Some("ショップ名(修正)".to_string()),
            })
            .await
            .expect("save_order_override (insert)");

        let id2 = repo
            .save_order_override(SaveOrderOverride {
                shop_domain: "1999.co.jp".to_string(),
                order_number: "ORD-002".to_string(),
                new_order_number: Some("ORD-002B".to_string()),
                order_date: Some("2024-02-02".to_string()),
                shop_name: Some("ショップ名(再修正)".to_string()),
            })
            .await
            .expect("save_order_override (update)");

        assert_eq!(id1, id2);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM order_overrides")
            .fetch_one(&pool)
            .await
            .expect("count order_overrides");
        assert_eq!(count, 1);

        let row: (String, String, String) = sqlx::query_as(
            "SELECT new_order_number, order_date, shop_name FROM order_overrides WHERE id = ?",
        )
        .bind(id1)
        .fetch_one(&pool)
        .await
        .expect("fetch order_overrides row");
        assert_eq!(row.0, "ORD-002B");
        assert_eq!(row.1, "2024-02-02");
        assert_eq!(row.2, "ショップ名(再修正)");
    }

    #[tokio::test]
    async fn test_override_repository_delete_order_override_by_key() {
        let pool = setup_test_db().await;
        let repo = SqliteOverrideRepository::new(pool.clone());

        let _id = repo
            .save_order_override(SaveOrderOverride {
                shop_domain: "1999.co.jp".to_string(),
                order_number: "ORD-DEL-ORDER".to_string(),
                new_order_number: Some("ORD-DEL-ORDER-2".to_string()),
                order_date: None,
                shop_name: None,
            })
            .await
            .expect("save_order_override");

        let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM order_overrides")
            .fetch_one(&pool)
            .await
            .expect("count order_overrides (before)");
        assert_eq!(before, 1);

        repo.delete_order_override_by_key("1999.co.jp", "ORD-DEL-ORDER")
            .await
            .expect("delete_order_override_by_key");

        let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM order_overrides")
            .fetch_one(&pool)
            .await
            .expect("count order_overrides (after)");
        assert_eq!(after, 0);
    }

    #[tokio::test]
    async fn test_override_repository_exclude_and_restore_item() {
        let pool = setup_test_db().await;
        let repo = SqliteOverrideRepository::new(pool.clone());

        let id1 = repo
            .exclude_item(ExcludeItemParams {
                shop_domain: "1999.co.jp".to_string(),
                order_number: "ORD-003".to_string(),
                item_name: "商品Z".to_string(),
                brand: "".to_string(),
                reason: Some("不要".to_string()),
            })
            .await
            .expect("exclude_item (insert)");

        let id2 = repo
            .exclude_item(ExcludeItemParams {
                shop_domain: "1999.co.jp".to_string(),
                order_number: "ORD-003".to_string(),
                item_name: "商品Z".to_string(),
                brand: "".to_string(),
                reason: Some("やっぱり不要".to_string()),
            })
            .await
            .expect("exclude_item (update)");

        assert_eq!(id1, id2);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM excluded_items")
            .fetch_one(&pool)
            .await
            .expect("count excluded_items");
        assert_eq!(count, 1);

        repo.restore_excluded_item(id1)
            .await
            .expect("restore_excluded_item");

        let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM excluded_items")
            .fetch_one(&pool)
            .await
            .expect("count excluded_items after restore");
        assert_eq!(count_after, 0);
    }

    #[tokio::test]
    async fn test_override_repository_exclude_and_restore_order() {
        let pool = setup_test_db().await;
        let repo = SqliteOverrideRepository::new(pool.clone());

        let id1 = repo
            .exclude_order(ExcludeOrderParams {
                shop_domain: "1999.co.jp".to_string(),
                order_number: "ORD-004".to_string(),
                reason: Some("不要".to_string()),
            })
            .await
            .expect("exclude_order (insert)");

        let id2 = repo
            .exclude_order(ExcludeOrderParams {
                shop_domain: "1999.co.jp".to_string(),
                order_number: "ORD-004".to_string(),
                reason: Some("重複".to_string()),
            })
            .await
            .expect("exclude_order (update)");

        assert_eq!(id1, id2);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM excluded_orders")
            .fetch_one(&pool)
            .await
            .expect("count excluded_orders");
        assert_eq!(count, 1);

        repo.restore_excluded_order(id1)
            .await
            .expect("restore_excluded_order");

        let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM excluded_orders")
            .fetch_one(&pool)
            .await
            .expect("count excluded_orders after restore");
        assert_eq!(count_after, 0);
    }
}
