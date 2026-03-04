use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

/// 注文・商品サマリ統計
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderStats {
    pub total_orders: i64,
    pub total_items: i64,
    /// 正規化名を持つユニーク商品数（商品名解析・商品画像と同一指標）
    pub distinct_items_with_normalized: i64,
    pub total_amount: i64,
}

/// 注文・商品サマリのDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait OrderStatsRepository: Send + Sync {
    /// 注文・商品サマリを取得
    async fn get_order_stats(&self) -> Result<OrderStats, String>;
}

/// 配送状況サマリ（注文ごとの最新配送ステータス別件数）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeliveryStats {
    pub not_shipped: i64,
    pub preparing: i64,
    pub shipped: i64,
    pub in_transit: i64,
    pub out_for_delivery: i64,
    pub delivered: i64,
    pub failed: i64,
    pub returned: i64,
    pub cancelled: i64,
    /// 1年以上未発送の件数（注文日または作成日が1年以上前）
    pub not_shipped_over_1_year: i64,
}

/// 配送状況のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait DeliveryStatsRepository: Send + Sync {
    /// 配送状況サマリを取得（注文ごとの最新ステータスで集計）
    async fn get_delivery_stats(&self) -> Result<DeliveryStats, String>;
}

/// 商品名解析（product_master）進捗
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductMasterStats {
    /// product_master テーブルの件数
    pub product_master_count: i64,
    /// 正規化名を持つユニーク商品数（解析対象）
    pub distinct_items_with_normalized: i64,
    /// 解析済み（product_master に存在する）ユニーク商品数
    pub items_with_parsed: i64,
}

/// 商品名解析進捗のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait ProductMasterStatsRepository: Send + Sync {
    /// 商品名解析進捗を取得
    async fn get_product_master_stats(&self) -> Result<ProductMasterStats, String>;
}

/// 店舗設定・画像サマリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiscStats {
    pub shop_settings_count: i64,
    pub shop_settings_enabled_count: i64,
    pub images_count: i64,
    /// 商品画像の網羅率計算用: 正規化名を持つユニーク商品数
    pub distinct_items_with_normalized: i64,
}

/// 店舗設定・画像のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait MiscStatsRepository: Send + Sync {
    /// 店舗設定・画像サマリを取得
    async fn get_misc_stats(&self) -> Result<MiscStats, String>;
}

/// SQLiteを使用したMiscStatsRepositoryの実装
pub struct SqliteMiscStatsRepository {
    pool: SqlitePool,
}

impl SqliteMiscStatsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MiscStatsRepository for SqliteMiscStatsRepository {
    async fn get_misc_stats(&self) -> Result<MiscStats, String> {
        let stats: (i64, i64, i64, Option<i64>) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM shop_settings) AS shop_settings_count,
                (SELECT COUNT(*) FROM shop_settings WHERE is_enabled = 1) AS shop_settings_enabled_count,
                (SELECT COUNT(*) FROM images) AS images_count,
                (SELECT COUNT(DISTINCT item_name_normalized) FROM items WHERE item_name_normalized IS NOT NULL) AS distinct_items_with_normalized
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch misc stats: {e}"))?;

        Ok(MiscStats {
            shop_settings_count: stats.0,
            shop_settings_enabled_count: stats.1,
            images_count: stats.2,
            distinct_items_with_normalized: stats.3.unwrap_or(0),
        })
    }
}

/// SQLiteを使用したProductMasterStatsRepositoryの実装
pub struct SqliteProductMasterStatsRepository {
    pool: SqlitePool,
}

impl SqliteProductMasterStatsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProductMasterStatsRepository for SqliteProductMasterStatsRepository {
    async fn get_product_master_stats(&self) -> Result<ProductMasterStats, String> {
        let stats: (i64, Option<i64>, Option<i64>) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM product_master) AS product_master_count,
                (SELECT COUNT(DISTINCT item_name_normalized) FROM items WHERE item_name_normalized IS NOT NULL) AS distinct_items,
                (SELECT COUNT(DISTINCT i.item_name_normalized) FROM items i INNER JOIN product_master pm ON i.item_name_normalized = pm.normalized_name) AS items_parsed
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch product master stats: {e}"))?;

        Ok(ProductMasterStats {
            product_master_count: stats.0,
            distinct_items_with_normalized: stats.1.unwrap_or(0),
            items_with_parsed: stats.2.unwrap_or(0),
        })
    }
}

/// SQLiteを使用したDeliveryStatsRepositoryの実装
pub struct SqliteDeliveryStatsRepository {
    pool: SqlitePool,
}

impl SqliteDeliveryStatsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DeliveryStatsRepository for SqliteDeliveryStatsRepository {
    async fn get_delivery_stats(&self) -> Result<DeliveryStats, String> {
        // latest_delivery CTE を一度だけ使い、ステータス別件数と1年以上未発送件数を同時に集計する
        let rows: Vec<(String, i64, i64)> = sqlx::query_as(
            r#"
            WITH latest_delivery AS (
                SELECT order_id, delivery_status
                FROM (
                    SELECT order_id, delivery_status,
                           ROW_NUMBER() OVER (PARTITION BY order_id ORDER BY updated_at DESC) AS rn
                    FROM deliveries
                ) t
                WHERE rn = 1
            )
            SELECT
                COALESCE(ld.delivery_status, 'not_shipped') AS status,
                COUNT(*) AS cnt,
                COUNT(CASE
                    WHEN COALESCE(ld.delivery_status, 'not_shipped') = 'not_shipped'
                     AND COALESCE(o.order_date, o.created_at) < date('now', '-1 year')
                    THEN 1
                END) AS over_1_year_cnt
            FROM orders o
            LEFT JOIN latest_delivery ld ON ld.order_id = o.id
            GROUP BY status
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch delivery stats: {e}"))?;

        let mut stats = DeliveryStats::default();
        for (status, cnt, over_1_year) in rows {
            match status.as_str() {
                "not_shipped" => {
                    stats.not_shipped = cnt;
                    stats.not_shipped_over_1_year = over_1_year;
                }
                "preparing" => stats.preparing = cnt,
                "shipped" => stats.shipped = cnt,
                "in_transit" => stats.in_transit = cnt,
                "out_for_delivery" => stats.out_for_delivery = cnt,
                "delivered" => stats.delivered = cnt,
                "failed" => stats.failed = cnt,
                "returned" => stats.returned = cnt,
                "cancelled" => stats.cancelled = cnt,
                _ => {}
            }
        }
        Ok(stats)
    }
}

/// SQLiteを使用したOrderStatsRepositoryの実装
pub struct SqliteOrderStatsRepository {
    pool: SqlitePool,
}

impl SqliteOrderStatsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrderStatsRepository for SqliteOrderStatsRepository {
    async fn get_order_stats(&self) -> Result<OrderStats, String> {
        let stats: (i64, i64, Option<i64>, Option<i64>) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM orders) AS total_orders,
                (SELECT COUNT(*) FROM items) AS total_items,
                (SELECT COUNT(DISTINCT item_name_normalized) FROM items WHERE item_name_normalized IS NOT NULL) AS distinct_items_with_normalized,
                (SELECT COALESCE(SUM(price * quantity), 0) FROM items) AS total_amount
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch order stats: {e}"))?;

        Ok(OrderStats {
            total_orders: stats.0,
            total_items: stats.1,
            distinct_items_with_normalized: stats.2.unwrap_or(0),
            total_amount: stats.3.unwrap_or(0),
        })
    }
}
