//! リポジトリパターンによるDB操作の抽象化
//!
//! このモジュールはデータベース操作を抽象化し、テスト時にモック可能にします。

use crate::gemini::normalize_product_name;
use crate::gmail::{CreateShopSettings, GmailMessage, ShopSettings, UpdateShopSettings};
use crate::parsers::hobbysearch_cancel::CancelInfo;
use crate::parsers::{EmailRow, OrderInfo};
use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{Sqlite, SqlitePool};
use std::collections::{HashMap, HashSet};

/// メール統計情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailStats {
    pub total_emails: i64,
    pub with_body_plain: i64,
    pub with_body_html: i64,
    pub without_body: i64,
    pub avg_plain_length: f64,
    pub avg_html_length: f64,
}

/// メール関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait EmailRepository: Send + Sync {
    /// メッセージをDBに保存
    ///
    /// # Returns
    /// `(saved, skipped)` - saved: rows_affected>0（INSERT または ON CONFLICT DO UPDATE）、
    /// skipped: rows_affected=0。
    ///
    /// # セマンティクス（ON CONFLICT DO UPDATE 使用時）
    /// 重複（既存 message_id）の場合も UPDATE が実行されるため、saved にカウントされる。
    /// skipped は rows_affected=0 のときのみ（SQLite の changes() は UPDATE でマッチ行数を返すため、通常は 0）。
    /// ログ／メトリクスで saved を「新規のみ」と解釈しないこと。
    async fn save_messages(&self, messages: &[GmailMessage]) -> Result<(usize, usize), String>;

    /// 既存のメッセージIDを取得
    async fn get_existing_message_ids(&self) -> Result<Vec<String>, String>;

    /// 指定されたメッセージIDのうち、emails テーブルに存在しないもののみを返す。
    /// メモリ効率のため、SQL の NOT IN を使用して DB 側でフィルタリングする。
    async fn filter_new_message_ids(&self, message_ids: &[String]) -> Result<Vec<String>, String>;

    /// メッセージ数を取得
    async fn get_message_count(&self) -> Result<i64, String>;
}

/// メール統計関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait EmailStatsRepository: Send + Sync {
    /// メール統計情報を取得
    async fn get_email_stats(&self) -> Result<EmailStats, String>;
}

/// 注文・商品サマリ統計
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderStats {
    pub total_orders: i64,
    pub total_items: i64,
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
        let stats: (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM shop_settings) AS shop_settings_count,
                (SELECT COUNT(*) FROM shop_settings WHERE is_enabled = 1) AS shop_settings_enabled_count,
                (SELECT COUNT(*) FROM images) AS images_count
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch misc stats: {e}"))?;

        Ok(MiscStats {
            shop_settings_count: stats.0,
            shop_settings_enabled_count: stats.1,
            images_count: stats.2,
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
        let rows: Vec<(String, i64)> = sqlx::query_as(
            r#"
            WITH latest_delivery AS (
                SELECT order_id, delivery_status
                FROM (
                    SELECT order_id, delivery_status,
                           ROW_NUMBER() OVER (PARTITION BY order_id ORDER BY updated_at DESC) AS rn
                    FROM deliveries
                ) t
                WHERE rn = 1
            ),
            order_status AS (
                SELECT COALESCE(ld.delivery_status, 'not_shipped') AS status
                FROM orders o
                LEFT JOIN latest_delivery ld ON ld.order_id = o.id
            )
            SELECT status, COUNT(*) AS cnt
            FROM order_status
            GROUP BY status
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch delivery stats: {e}"))?;

        let mut stats = DeliveryStats::default();
        for (status, cnt) in rows {
            match status.as_str() {
                "not_shipped" => stats.not_shipped = cnt,
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
        let stats: (i64, i64, Option<i64>) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM orders) AS total_orders,
                (SELECT COUNT(*) FROM items) AS total_items,
                (SELECT COALESCE(SUM(price * quantity), 0) FROM items) AS total_amount
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch order stats: {e}"))?;

        Ok(OrderStats {
            total_orders: stats.0,
            total_items: stats.1,
            total_amount: stats.2.unwrap_or(0),
        })
    }
}

/// 注文関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait OrderRepository: Send + Sync {
    /// 注文情報を保存（orders, items, deliveries, order_emailsテーブル）
    async fn save_order(
        &self,
        order_info: &OrderInfo,
        email_id: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
    ) -> Result<i64, String>;

    /// キャンセルメールの内容を適用（該当商品の数量減算または削除）
    async fn apply_cancel(
        &self,
        cancel_info: &CancelInfo,
        email_id: i64,
        shop_domain: Option<String>,
        shop_name: Option<String>,
    ) -> Result<i64, String>;

    /// 組み換えメールの商品を元注文から削除する。
    /// 新注文の各商品について、同じショップの過去注文（発送済みでない）から商品名でマッチする item を検索し削除する。
    /// 残り商品が 0 になった order は削除する。
    async fn apply_change_items(
        &self,
        order_info: &OrderInfo,
        shop_domain: Option<String>,
        change_email_internal_date: Option<i64>,
    ) -> Result<(), String>;

    /// 組み換えメール用: apply_change_items と save_order を同一トランザクションで実行する。
    /// データ欠損（元注文だけ減って新注文が保存されない）を防ぐ。
    async fn apply_change_items_and_save_order(
        &self,
        order_info: &OrderInfo,
        email_id: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
        change_email_internal_date: Option<i64>,
    ) -> Result<i64, String>;
}

/// パース関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait ParseRepository: Send + Sync {
    /// 未パースのメールを取得（order_emails に存在しないメール）
    async fn get_unparsed_emails(&self, batch_size: usize) -> Result<Vec<EmailRow>, String>;

    /// 注文関連テーブルをクリア（order_emails, deliveries, items, orders）
    async fn clear_order_tables(&self) -> Result<(), String>;

    /// パース対象の全メール数を取得
    async fn get_total_email_count(&self) -> Result<i64, String>;
}

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

/// SQLiteを使用したEmailStatsRepositoryの実装
pub struct SqliteEmailStatsRepository {
    pool: SqlitePool,
}

impl SqliteEmailStatsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EmailStatsRepository for SqliteEmailStatsRepository {
    async fn get_email_stats(&self) -> Result<EmailStats, String> {
        let stats: (i64, i64, i64, i64, Option<f64>, Option<f64>) = sqlx::query_as(
            r#"
            WITH email_lengths AS (
                SELECT
                    body_plain,
                    body_html,
                    CASE WHEN body_plain IS NOT NULL THEN LENGTH(body_plain) ELSE 0 END AS plain_length,
                    CASE WHEN body_html IS NOT NULL THEN LENGTH(body_html) ELSE 0 END AS html_length
                FROM emails
            )
            SELECT
                COUNT(*) AS total,
                COUNT(CASE WHEN body_plain IS NOT NULL AND plain_length > 0 THEN 1 END) AS with_plain,
                COUNT(CASE WHEN body_html IS NOT NULL AND html_length > 0 THEN 1 END) AS with_html,
                COUNT(CASE WHEN (body_plain IS NULL OR plain_length = 0) AND (body_html IS NULL OR html_length = 0) THEN 1 END) AS without_body,
                AVG(CASE WHEN body_plain IS NOT NULL AND plain_length > 0 THEN plain_length END) AS avg_plain,
                AVG(CASE WHEN body_html IS NOT NULL AND html_length > 0 THEN html_length END) AS avg_html
            FROM email_lengths
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch email stats: {e}"))?;

        Ok(EmailStats {
            total_emails: stats.0,
            with_body_plain: stats.1,
            with_body_html: stats.2,
            without_body: stats.3,
            avg_plain_length: stats.4.unwrap_or(0.0),
            avg_html_length: stats.5.unwrap_or(0.0),
        })
    }
}

/// 商品名比較用に【】[]（）() で囲まれた部分を除去する
fn strip_bracketed_content(s: &str) -> String {
    static RE: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
        // 【】[]（）() とその囲まれた内容を除去
        Regex::new(r"【[^】]*】|\[[^\]]*\]|（[^）]*）|\([^)]*\)")
            .expect("strip_bracketed_content regex")
    });
    RE.replace_all(s, "").trim().to_string()
}

/// 商品名がマッチするか判定（apply_cancel / apply_change_items で共通利用）
fn item_names_match(
    product_name: &str,
    item_name: &str,
    item_name_normalized: Option<&str>,
) -> bool {
    let product_name_core = product_name
        .trim_end_matches(" (プラモデル)")
        .trim_end_matches(" (ディスプレイ)")
        .trim();
    let product_name_stripped = strip_bracketed_content(product_name);
    let product_normalized = normalize_product_name(product_name);

    let item_trimmed = item_name.trim();
    let item_stripped = strip_bracketed_content(item_trimmed);

    if item_trimmed == product_name || item_trimmed == product_name_core {
        return true;
    }
    if item_trimmed.contains(product_name)
        || product_name.contains(item_trimmed)
        || item_trimmed.contains(product_name_core)
        || product_name_core.contains(item_trimmed)
        || (!product_name_stripped.is_empty()
            && (item_trimmed.contains(&product_name_stripped)
                || product_name_stripped.contains(item_trimmed)))
        || {
            let item_stripped_nonempty = !item_stripped.is_empty();
            !product_name_stripped.is_empty()
                && item_stripped_nonempty
                && (item_stripped.contains(&product_name_stripped)
                    || product_name_stripped.contains(&item_stripped))
        }
    {
        return true;
    }
    let db_normalized = item_name_normalized
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| normalize_product_name(item_name));
    product_normalized == db_normalized
        || product_normalized.contains(db_normalized.as_str())
        || db_normalized.contains(product_normalized.as_str())
}

/// apply_change_items で order_id ごとの items を保持する型
/// (item_id, item_name, item_name_normalized, quantity)
type ItemsByOrderMap = HashMap<i64, Vec<(i64, String, Option<String>, i64)>>;

/// SQLiteを使用したOrderRepositoryの実装
pub struct SqliteOrderRepository {
    pool: SqlitePool,
}

impl SqliteOrderRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// apply_change_items のトランザクション内ロジック（tx は呼び出し元で commit）
    async fn apply_change_items_in_tx(
        tx: &mut sqlx::Transaction<'_, Sqlite>,
        order_info: &OrderInfo,
        shop_domain: Option<String>,
        change_email_internal_date: Option<i64>,
    ) -> Result<(), String> {
        let new_order_number = &order_info.order_number;
        // i64::MAX は SQLite datetime() でオーバーフローするため、None 時は 2100年 UTC を使用
        let cutoff_ts = change_email_internal_date.unwrap_or(4_102_444_800_000i64); // 2100-01-01 00:00:00 UTC

        let order_ids: Vec<i64> = if let Some(ref d) = shop_domain {
            if !d.is_empty() {
                sqlx::query_scalar(
                    r#"
                    SELECT o.id FROM orders o
                    WHERE o.order_number != ?
                    AND o.shop_domain = ?
                    AND o.id NOT IN (
                        SELECT d.order_id FROM deliveries d
                        WHERE d.delivery_status IN ('shipped', 'in_transit', 'out_for_delivery', 'delivered')
                    )
                    AND COALESCE(o.order_date, o.created_at) < datetime(? / 1000, 'unixepoch')
                    ORDER BY o.order_date IS NULL, o.order_date DESC, o.id DESC
                    "#,
                )
                .bind(new_order_number)
                .bind(d)
                .bind(cutoff_ts)
                .fetch_all(tx.as_mut())
                .await
                .map_err(|e| format!("Failed to fetch change-target orders: {e}"))?
            } else {
                sqlx::query_scalar(
                    r#"
                    SELECT o.id FROM orders o
                    WHERE o.order_number != ?
                    AND (o.shop_domain IS NULL OR o.shop_domain = '')
                    AND o.id NOT IN (
                        SELECT d.order_id FROM deliveries d
                        WHERE d.delivery_status IN ('shipped', 'in_transit', 'out_for_delivery', 'delivered')
                    )
                    AND COALESCE(o.order_date, o.created_at) < datetime(? / 1000, 'unixepoch')
                    ORDER BY o.order_date IS NULL, o.order_date DESC, o.id DESC
                    "#,
                )
                .bind(new_order_number)
                .bind(cutoff_ts)
                .fetch_all(tx.as_mut())
                .await
                .map_err(|e| format!("Failed to fetch change-target orders: {e}"))?
            }
        } else {
            sqlx::query_scalar(
                r#"
                SELECT o.id FROM orders o
                WHERE o.order_number != ?
                AND (o.shop_domain IS NULL OR o.shop_domain = '')
                AND o.id NOT IN (
                    SELECT d.order_id FROM deliveries d
                    WHERE d.delivery_status IN ('shipped', 'in_transit', 'out_for_delivery', 'delivered')
                )
                AND COALESCE(o.order_date, o.created_at) < datetime(? / 1000, 'unixepoch')
                ORDER BY o.order_date IS NULL, o.order_date DESC, o.id DESC
                "#,
            )
            .bind(new_order_number)
            .bind(cutoff_ts)
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to fetch change-target orders: {e}"))?
        };

        let mut items_by_order: ItemsByOrderMap = if order_ids
            .is_empty()
        {
            HashMap::new()
        } else {
            let placeholders: Vec<String> = (0..order_ids.len()).map(|_| "?".to_string()).collect();
            let placeholders_str = placeholders.join(", ");
            let query_str = format!(
                r#"SELECT order_id, id, item_name, item_name_normalized, quantity FROM items WHERE order_id IN ({}) ORDER BY order_id, id"#,
                placeholders_str
            );
            let mut q = sqlx::query_as::<_, (i64, i64, String, Option<String>, i64)>(&query_str);
            for id in &order_ids {
                q = q.bind(id);
            }
            let rows: Vec<(i64, i64, String, Option<String>, i64)> = q
                .fetch_all(tx.as_mut())
                .await
                .map_err(|e| format!("Failed to fetch items: {e}"))?;
            let mut map: ItemsByOrderMap = HashMap::new();
            for (order_id, id, item_name, item_name_normalized, quantity) in rows {
                map.entry(order_id).or_default().push((
                    id,
                    item_name,
                    item_name_normalized,
                    quantity,
                ));
            }
            map
        };

        let mut orders_to_delete: HashSet<i64> = HashSet::new();

        for item in &order_info.items {
            let product_name = item.name.trim();
            let cancel_qty = item.quantity.max(0);

            if cancel_qty <= 0 {
                continue;
            }

            let mut remaining_qty = cancel_qty;
            let mut matched_any = false;

            for &order_id in &order_ids {
                if remaining_qty <= 0 {
                    break;
                }
                // 同一 order_id 内で remaining_qty > 0 の間は複数行を順次消費する
                loop {
                    if remaining_qty <= 0 {
                        break;
                    }
                    let items = items_by_order
                        .get(&order_id)
                        .map(|v| v.as_slice())
                        .unwrap_or(&[]);

                    let found = items.iter().find(|(_, item_name, item_name_normalized, _)| {
                        item_names_match(
                            product_name,
                            item_name,
                            item_name_normalized.as_deref(),
                        )
                    });

                    if let Some((item_id, _, _, current_qty)) = found {
                        matched_any = true;
                        let item_id = *item_id;
                        let current_qty = *current_qty;
                        let take_qty = remaining_qty.min(current_qty);
                        let new_qty = current_qty - take_qty;
                        remaining_qty -= take_qty;

                        if new_qty <= 0 {
                            sqlx::query("DELETE FROM items WHERE id = ?")
                                .bind(item_id)
                                .execute(tx.as_mut())
                                .await
                                .map_err(|e| format!("Failed to delete item: {e}"))?;
                            log::debug!(
                                "apply_change_items: removed item id={} from order {}",
                                item_id,
                                order_id
                            );
                            if let Some(vec) = items_by_order.get_mut(&order_id) {
                                vec.retain(|(id, _, _, _)| *id != item_id);
                            }
                            orders_to_delete.insert(order_id);
                        } else {
                            sqlx::query("UPDATE items SET quantity = ? WHERE id = ?")
                                .bind(new_qty)
                                .bind(item_id)
                                .execute(tx.as_mut())
                                .await
                                .map_err(|e| format!("Failed to update item quantity: {e}"))?;
                            log::debug!(
                                "apply_change_items: item id={} quantity {} -> {}",
                                item_id,
                                current_qty,
                                new_qty
                            );
                            if let Some(vec) = items_by_order.get_mut(&order_id) {
                                if let Some(entry) =
                                    vec.iter_mut().find(|(id, _, _, _)| *id == item_id)
                                {
                                    entry.3 = new_qty;
                                }
                            }
                        }
                    } else {
                        // この order_id ではこれ以上マッチする items がない
                        break;
                    }
                }
            }

            if !matched_any || remaining_qty > 0 {
                log::warn!(
                    "apply_change_items: no matching order for item {:?} shop_domain={:?} order_number={} (remaining_qty={})",
                    product_name,
                    shop_domain,
                    order_info.order_number,
                    remaining_qty
                );
            }
        }

        for order_id in orders_to_delete {
            let remaining: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
                .bind(order_id)
                .fetch_one(tx.as_mut())
                .await
                .map_err(|e| format!("Failed to count items: {e}"))?;
            if remaining.0 == 0 {
                sqlx::query("DELETE FROM order_emails WHERE order_id = ?")
                    .bind(order_id)
                    .execute(tx.as_mut())
                    .await
                    .map_err(|e| format!("Failed to delete order_emails: {e}"))?;
                sqlx::query("DELETE FROM deliveries WHERE order_id = ?")
                    .bind(order_id)
                    .execute(tx.as_mut())
                    .await
                    .map_err(|e| format!("Failed to delete deliveries: {e}"))?;
                sqlx::query("DELETE FROM orders WHERE id = ?")
                    .bind(order_id)
                    .execute(tx.as_mut())
                    .await
                    .map_err(|e| format!("Failed to delete order: {e}"))?;
                log::info!("apply_change_items: removed empty order {}", order_id);
            }
        }

        Ok(())
    }

    /// save_order のトランザクション内ロジック（tx は呼び出し元で commit）
    async fn save_order_in_tx(
        tx: &mut sqlx::Transaction<'_, Sqlite>,
        order_info: &OrderInfo,
        email_id: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
    ) -> Result<i64, String> {
        let existing_order: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT id FROM orders
            WHERE order_number = ? AND shop_domain = ?
            LIMIT 1
            "#,
        )
        .bind(&order_info.order_number)
        .bind(shop_domain.as_deref())
        .fetch_optional(tx.as_mut())
        .await
        .map_err(|e| format!("Failed to check existing order: {e}"))?;

        let order_id = if let Some((existing_id,)) = existing_order {
            log::debug!("Found existing order with id: {}", existing_id);
            existing_id
        } else {
            let new_order_id = sqlx::query(
                r#"
                INSERT INTO orders (order_number, order_date, shop_domain, shop_name)
                VALUES (?, ?, ?, ?)
                "#,
            )
            .bind(&order_info.order_number)
            .bind(&order_info.order_date)
            .bind(shop_domain.as_deref())
            .bind(shop_name.as_deref())
            .execute(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to insert order: {e}"))?
            .last_insert_rowid();

            log::debug!("Created new order with id: {}", new_order_id);
            new_order_id
        };

        if existing_order.is_some() && order_info.order_date.is_some() {
            sqlx::query(
                r#"
                UPDATE orders
                SET order_date = COALESCE(?, order_date)
                WHERE id = ?
                "#,
            )
            .bind(&order_info.order_date)
            .bind(order_id)
            .execute(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to update order date: {e}"))?;

            log::debug!("Updated order {} with new date info", order_id);
        }

        for item in &order_info.items {
            let existing_item: Option<(i64,)> = sqlx::query_as(
                r#"
                SELECT id FROM items
                WHERE order_id = ? AND item_name = ? AND COALESCE(brand, '') = COALESCE(?, '')
                LIMIT 1
                "#,
            )
            .bind(order_id)
            .bind(&item.name)
            .bind(&item.manufacturer)
            .fetch_optional(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to check existing item: {e}"))?;

            if existing_item.is_none() {
                let item_name_normalized = {
                    let n = normalize_product_name(&item.name);
                    if n.is_empty() {
                        None
                    } else {
                        Some(n)
                    }
                };
                sqlx::query(
                    r#"
                    INSERT INTO items (order_id, item_name, item_name_normalized, brand, price, quantity)
                    VALUES (?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(order_id)
                .bind(&item.name)
                .bind(item_name_normalized.as_deref())
                .bind(&item.manufacturer)
                .bind(item.unit_price)
                .bind(item.quantity)
                .execute(tx.as_mut())
                .await
                .map_err(|e| format!("Failed to insert item: {e}"))?;

                log::debug!("Added new item '{}' to order {}", item.name, order_id);
            } else {
                log::debug!("Item '{}' already exists for order {}", item.name, order_id);
            }
        }

        if let Some(delivery_info) = &order_info.delivery_info {
            let existing_delivery: Option<(i64,)> = sqlx::query_as(
                r#"
                SELECT id FROM deliveries
                WHERE order_id = ? AND tracking_number = ?
                LIMIT 1
                "#,
            )
            .bind(order_id)
            .bind(&delivery_info.tracking_number)
            .fetch_optional(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to check existing delivery: {e}"))?;

            if existing_delivery.is_none() {
                sqlx::query(
                    r#"
                    INSERT INTO deliveries (order_id, tracking_number, carrier, delivery_status)
                    VALUES (?, ?, ?, 'shipped')
                    "#,
                )
                .bind(order_id)
                .bind(&delivery_info.tracking_number)
                .bind(&delivery_info.carrier)
                .execute(tx.as_mut())
                .await
                .map_err(|e| format!("Failed to insert delivery: {e}"))?;

                log::debug!("Added new delivery info for order {}", order_id);
            } else {
                sqlx::query(
                    r#"
                    UPDATE deliveries
                    SET carrier = COALESCE(?, carrier),
                        delivery_status = 'shipped'
                    WHERE order_id = ? AND tracking_number = ?
                    "#,
                )
                .bind(&delivery_info.carrier)
                .bind(order_id)
                .bind(&delivery_info.tracking_number)
                .execute(tx.as_mut())
                .await
                .map_err(|e| format!("Failed to update delivery: {e}"))?;

                log::debug!("Updated delivery info for order {}", order_id);
            }
        }

        if let Some(email_id_val) = email_id {
            let existing_link: Option<(i64,)> = sqlx::query_as(
                r#"
                SELECT order_id FROM order_emails
                WHERE order_id = ? AND email_id = ?
                LIMIT 1
                "#,
            )
            .bind(order_id)
            .bind(email_id_val)
            .fetch_optional(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to check existing order_email link: {e}"))?;

            if existing_link.is_none() {
                sqlx::query(
                    r#"
                    INSERT INTO order_emails (order_id, email_id)
                    VALUES (?, ?)
                    "#,
                )
                .bind(order_id)
                .bind(email_id_val)
                .execute(tx.as_mut())
                .await
                .map_err(|e| format!("Failed to link order to email: {e}"))?;

                log::debug!("Linked order {} to email {}", order_id, email_id_val);
            } else {
                log::debug!(
                    "Order {} is already linked to email {}",
                    order_id,
                    email_id_val
                );
            }
        }

        Ok(order_id)
    }
}

#[async_trait]
impl OrderRepository for SqliteOrderRepository {
    async fn save_order(
        &self,
        order_info: &OrderInfo,
        email_id: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
    ) -> Result<i64, String> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to start transaction: {e}"))?;

        let order_id =
            Self::save_order_in_tx(&mut tx, order_info, email_id, shop_domain, shop_name).await?;

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {e}"))?;

        Ok(order_id)
    }

    async fn apply_cancel(
        &self,
        cancel_info: &CancelInfo,
        email_id: i64,
        shop_domain: Option<String>,
        _shop_name: Option<String>,
    ) -> Result<i64, String> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to start transaction: {e}"))?;

        // 1. 既存の注文を検索（order_number + shop_domain）
        // インデックス (order_number, shop_domain) を活かすため、shop_domain に関数をかけずクエリを分岐
        let order_row: Option<(i64,)> = if let Some(ref domain) = shop_domain {
            if !domain.is_empty() {
                sqlx::query_as(
                    r#"
                    SELECT id FROM orders
                    WHERE order_number = ? AND shop_domain = ?
                    LIMIT 1
                    "#,
                )
                .bind(&cancel_info.order_number)
                .bind(domain)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| format!("Failed to find order: {e}"))?
            } else {
                sqlx::query_as(
                    r#"
                    SELECT id FROM orders
                    WHERE order_number = ? AND (shop_domain IS NULL OR shop_domain = '')
                    LIMIT 1
                    "#,
                )
                .bind(&cancel_info.order_number)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| format!("Failed to find order: {e}"))?
            }
        } else {
            sqlx::query_as(
                r#"
                SELECT id FROM orders
                WHERE order_number = ? AND (shop_domain IS NULL OR shop_domain = '')
                LIMIT 1
                "#,
            )
            .bind(&cancel_info.order_number)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("Failed to find order: {e}"))?
        };

        let order_id = match order_row {
            Some((id,)) => id,
            None => {
                log::warn!(
                    "Cancel mail: order {} not found (shop_domain={:?})",
                    cancel_info.order_number,
                    shop_domain
                );
                tx.rollback()
                    .await
                    .map_err(|e| format!("Failed to rollback: {e}"))?;
                return Err(format!(
                    "Order {} not found for cancel",
                    cancel_info.order_number
                ));
            }
        };

        // 2. 該当注文の商品を検索（完全一致 → 包含 → item_name_normalized 部分一致の順でマッチ）
        let items: Vec<(i64, String, Option<String>, i64)> = sqlx::query_as(
            r#"
            SELECT id, item_name, item_name_normalized, quantity FROM items
            WHERE order_id = ?
            ORDER BY id
            "#,
        )
        .bind(order_id)
        .fetch_all(tx.as_mut())
        .await
        .map_err(|e| format!("Failed to fetch items: {e}"))?;

        let product_name = cancel_info.product_name.trim();
        let matched = items
            .iter()
            .find(|(_, item_name, item_name_normalized, _)| {
                item_names_match(
                    product_name,
                    item_name,
                    item_name_normalized.as_deref(),
                )
            });

        match matched {
            Some((item_id, _, _, current_qty)) => {
                let item_id = *item_id;
                let current_qty = *current_qty;

                if cancel_info.cancel_quantity <= 0 {
                    log::warn!(
                        "Invalid cancel quantity {} for product '{}' in order {}",
                        cancel_info.cancel_quantity,
                        product_name,
                        order_id
                    );
                    tx.rollback()
                        .await
                        .map_err(|e| format!("Failed to rollback: {e}"))?;
                    return Err(format!(
                        "Invalid cancel quantity {} for product '{}'",
                        cancel_info.cancel_quantity, product_name
                    ));
                }

                let new_qty = current_qty - cancel_info.cancel_quantity;

                if new_qty <= 0 {
                    sqlx::query("DELETE FROM items WHERE id = ?")
                        .bind(item_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| format!("Failed to delete item: {e}"))?;
                    log::info!(
                        "Cancel applied: removed item id={} from order {}",
                        item_id,
                        order_id
                    );
                } else {
                    sqlx::query("UPDATE items SET quantity = ? WHERE id = ?")
                        .bind(new_qty)
                        .bind(item_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| format!("Failed to update item quantity: {e}"))?;
                    log::info!(
                        "Cancel applied: item id={} quantity {} -> {}",
                        item_id,
                        current_qty,
                        new_qty
                    );
                }
            }
            None => {
                log::warn!(
                    "Cancel mail: product '{}' not found in order {}",
                    product_name,
                    order_id
                );
                tx.rollback()
                    .await
                    .map_err(|e| format!("Failed to rollback: {e}"))?;
                return Err(format!("Product '{}' not found in order", product_name));
            }
        }

        // 3. order_emails にメールとの関連を保存
        let existing_link: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT order_id FROM order_emails
            WHERE order_id = ? AND email_id = ?
            LIMIT 1
            "#,
        )
        .bind(order_id)
        .bind(email_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Failed to check order_email link: {e}"))?;

        if existing_link.is_none() {
            sqlx::query(
                r#"
                INSERT INTO order_emails (order_id, email_id)
                VALUES (?, ?)
                "#,
            )
            .bind(order_id)
            .bind(email_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to link order to email: {e}"))?;
        }

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {e}"))?;

        Ok(order_id)
    }

    async fn apply_change_items(
        &self,
        order_info: &OrderInfo,
        shop_domain: Option<String>,
        change_email_internal_date: Option<i64>,
    ) -> Result<(), String> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to start transaction: {e}"))?;

        Self::apply_change_items_in_tx(
            &mut tx,
            order_info,
            shop_domain,
            change_email_internal_date,
        )
        .await?;

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {e}"))?;

        Ok(())
    }

    async fn apply_change_items_and_save_order(
        &self,
        order_info: &OrderInfo,
        email_id: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
        change_email_internal_date: Option<i64>,
    ) -> Result<i64, String> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to start transaction: {e}"))?;

        Self::apply_change_items_in_tx(
            &mut tx,
            order_info,
            shop_domain.clone(),
            change_email_internal_date,
        )
        .await?;

        let order_id =
            Self::save_order_in_tx(&mut tx, order_info, email_id, shop_domain, shop_name).await?;

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {e}"))?;

        Ok(order_id)
    }
}

/// SQLiteを使用したParseRepositoryの実装
pub struct SqliteParseRepository {
    pool: SqlitePool,
}

impl SqliteParseRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ParseRepository for SqliteParseRepository {
    async fn get_unparsed_emails(&self, batch_size: usize) -> Result<Vec<EmailRow>, String> {
        let emails: Vec<EmailRow> = sqlx::query_as(
            r#"
            SELECT e.id, e.message_id, e.body_plain, e.from_address, e.subject, e.internal_date
            FROM emails e
            LEFT JOIN order_emails oe ON e.id = oe.email_id
            WHERE e.body_plain IS NOT NULL
            AND e.from_address IS NOT NULL
            AND oe.email_id IS NULL
            ORDER BY e.internal_date ASC
            LIMIT ?
            "#,
        )
        .bind(batch_size as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch unparsed emails: {e}"))?;

        Ok(emails)
    }

    async fn clear_order_tables(&self) -> Result<(), String> {
        // トランザクション内で全てのDELETE操作を実行してアトミック性を確保
        // 外部キー制約により、order_emails -> deliveries -> items -> orders の順でクリア
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin transaction: {e}"))?;

        sqlx::query("DELETE FROM order_emails")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to clear order_emails table: {e}"))?;

        sqlx::query("DELETE FROM deliveries")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to clear deliveries table: {e}"))?;

        sqlx::query("DELETE FROM items")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to clear items table: {e}"))?;

        sqlx::query("DELETE FROM orders")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to clear orders table: {e}"))?;

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {e}"))?;

        Ok(())
    }

    async fn get_total_email_count(&self) -> Result<i64, String> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM emails
            WHERE body_plain IS NOT NULL
            AND from_address IS NOT NULL
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to count emails: {e}"))?;

        Ok(count)
    }
}

#[async_trait]
impl EmailRepository for SqliteEmailRepository {
    async fn save_messages(&self, messages: &[GmailMessage]) -> Result<(usize, usize), String> {
        let mut saved = 0;
        let mut skipped = 0;

        // rows_affected の解釈: SQLite の changes() は「UPDATE でマッチした行数」ではなく
        // 「直近のステートメントによって実際に変更された行数」を返す。
        // このクエリでは ON CONFLICT DO UPDATE が発生した場合、既存行に対して UPDATE が実行され、
        // COALESCE により既存値がそのまま再代入されるケースでも、その行は更新されたものとしてカウントされるため
        // rows_affected は 1 となる想定である。
        // saved は「新規のみ」ではなく「INSERT/UPDATE が行われた件数」。FetchResult/SyncProgressEvent の
        // saved_count/newly_saved に伝播する。
        // 参考: https://www.sqlite.org/c3ref/changes.html

        // トランザクションを使用してバッチ処理
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin transaction: {e}"))?;

        for message in messages {
            // ON CONFLICT で既存の場合は body を補完（初回同期時に body_html 等が取れなかった場合の再取得で更新）
            let result = sqlx::query(
                r#"
                INSERT INTO emails (message_id, body_plain, body_html, internal_date, from_address, subject)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(message_id) DO UPDATE SET
                    body_plain = COALESCE(excluded.body_plain, body_plain),
                    body_html = COALESCE(excluded.body_html, body_html),
                    internal_date = COALESCE(excluded.internal_date, internal_date),
                    from_address = COALESCE(excluded.from_address, from_address),
                    subject = COALESCE(excluded.subject, subject)
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

    async fn filter_new_message_ids(&self, message_ids: &[String]) -> Result<Vec<String>, String> {
        if message_ids.is_empty() {
            return Ok(Vec::new());
        }

        // トランザクション内で実行（TEMP テーブルは接続ごとのため、プールでは同一接続を保証する必要がある）
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin transaction: {e}"))?;

        // 一時テーブルを作成
        sqlx::query(
            "CREATE TEMP TABLE IF NOT EXISTS temp_filter_ids (message_id TEXT PRIMARY KEY)",
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to create temp table: {e}"))?;

        // SQLite の SQLITE_MAX_VARIABLE_NUMBER (999) を考慮してチャンク処理
        const CHUNK_SIZE: usize = 900;
        let mut new_ids = Vec::new();

        for chunk in message_ids.chunks(CHUNK_SIZE) {
            sqlx::query("DELETE FROM temp_filter_ids")
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Failed to clear temp table: {e}"))?;

            // チャンクを一時テーブルに INSERT
            let mut q = sqlx::QueryBuilder::new("INSERT INTO temp_filter_ids (message_id) ");
            q.push_values(chunk, |mut b, id| {
                b.push_bind(id);
            });
            q.build()
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Failed to insert into temp table: {e}"))?;

            // NOT IN でフィルタリング
            let rows: Vec<(String,)> = sqlx::query_as(
                "SELECT message_id FROM temp_filter_ids WHERE message_id NOT IN (SELECT message_id FROM emails)",
            )
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to filter new IDs: {e}"))?;

            new_ids.extend(rows.into_iter().map(|(id,)| id));
        }

        sqlx::query("DROP TABLE IF EXISTS temp_filter_ids")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to drop temp table: {e}"))?;

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {e}"))?;

        Ok(new_ids)
    }

    async fn get_message_count(&self) -> Result<i64, String> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM emails")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| format!("Failed to get message count: {e}"))?;

        Ok(count.0)
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
}

// =============================================================================
// ProductMasterRepository - Gemini AI による商品名解析結果のキャッシュ
// =============================================================================

use crate::gemini::ParsedProduct;

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
    use crate::gemini::ParsedProduct;
    use crate::parsers::hobbysearch_cancel::CancelInfo;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // テーブル作成（migrationsと同等の定義）

        // emails テーブル (002, 011, 017, 019 に対応)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS emails (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id TEXT UNIQUE NOT NULL,
                body_plain TEXT,
                body_html TEXT,
                analysis_status TEXT NOT NULL DEFAULT 'pending' CHECK(analysis_status IN ('pending', 'completed')),
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                internal_date INTEGER,
                from_address TEXT,
                subject TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create emails table");

        // orders テーブル (003 に対応、shop_name 含む)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS orders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                shop_domain TEXT,
                shop_name TEXT,
                order_number TEXT,
                order_date DATETIME,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create orders table");

        // items テーブル (004 に対応)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL,
                item_name TEXT NOT NULL,
                item_name_normalized TEXT,
                price INTEGER NOT NULL DEFAULT 0,
                quantity INTEGER NOT NULL DEFAULT 1,
                category TEXT,
                brand TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create items table");

        // deliveries テーブル (006 に対応)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS deliveries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL,
                tracking_number TEXT,
                carrier TEXT,
                delivery_status TEXT NOT NULL DEFAULT 'not_shipped' CHECK(delivery_status IN ('not_shipped', 'preparing', 'shipped', 'in_transit', 'out_for_delivery', 'delivered', 'failed', 'returned', 'cancelled')),
                estimated_delivery DATETIME,
                actual_delivery DATETIME,
                last_checked_at DATETIME,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create deliveries table");

        // order_emails テーブル (008 に対応)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS order_emails (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL,
                email_id INTEGER NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
                FOREIGN KEY (email_id) REFERENCES emails(id) ON DELETE CASCADE,
                UNIQUE (order_id, email_id)
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create order_emails table");

        // shop_settings テーブル (014, 015 に対応)
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

        // product_master テーブル (002 に対応)
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

        // 外部キー制約を有効化（ロールバックテストで使用）
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("Failed to enable foreign keys");

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

        // 重複保存: ON CONFLICT DO UPDATE により UPDATE が実行され、saved としてカウントされる
        let (saved, skipped) = repo.save_messages(&messages).await.unwrap();
        assert_eq!(saved, 2);
        assert_eq!(skipped, 0);

        // カウント確認
        let count = repo.get_message_count().await.unwrap();
        assert_eq!(count, 2);

        // 既存ID取得
        let ids = repo.get_existing_message_ids().await.unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"test123".to_string()));
        assert!(ids.contains(&"test456".to_string()));

        // filter_new_message_ids: 既存IDは除外され、新規IDのみ返る
        let input_ids = vec![
            "test123".to_string(),
            "test456".to_string(),
            "new_id_1".to_string(),
            "new_id_2".to_string(),
        ];
        let new_ids = repo.filter_new_message_ids(&input_ids).await.unwrap();
        assert_eq!(new_ids.len(), 2);
        assert!(new_ids.contains(&"new_id_1".to_string()));
        assert!(new_ids.contains(&"new_id_2".to_string()));
        assert!(!new_ids.contains(&"test123".to_string()));
        assert!(!new_ids.contains(&"test456".to_string()));

        // 空配列の場合は空を返す
        let empty = repo.filter_new_message_ids(&[]).await.unwrap();
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn test_filter_new_message_ids_chunk_boundaries() {
        // CHUNK_SIZE (900) を超える件数、境界値、全既存、全新規のエッジケース
        let pool = setup_test_db().await;
        let repo = SqliteEmailRepository::new(pool);

        // 既存メッセージを3件保存
        let existing = vec![
            GmailMessage {
                message_id: "existing_1".to_string(),
                snippet: "".to_string(),
                subject: None,
                body_plain: None,
                body_html: None,
                internal_date: 0,
                from_address: None,
            },
            GmailMessage {
                message_id: "existing_2".to_string(),
                snippet: "".to_string(),
                subject: None,
                body_plain: None,
                body_html: None,
                internal_date: 0,
                from_address: None,
            },
            GmailMessage {
                message_id: "existing_3".to_string(),
                snippet: "".to_string(),
                subject: None,
                body_plain: None,
                body_html: None,
                internal_date: 0,
                from_address: None,
            },
        ];
        repo.save_messages(&existing).await.unwrap();

        // 全既存ID → 空を返す
        let all_existing = vec![
            "existing_1".to_string(),
            "existing_2".to_string(),
            "existing_3".to_string(),
        ];
        let result = repo.filter_new_message_ids(&all_existing).await.unwrap();
        assert!(result.is_empty(), "all existing should return empty");

        // 全新規ID → 全て返す
        let all_new: Vec<String> = (0..5).map(|i| format!("new_only_{}", i)).collect();
        let result = repo.filter_new_message_ids(&all_new).await.unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(result, all_new);

        // CHUNK_SIZE ちょうど (900件): 既存3 + 新規897
        let mut ids_900: Vec<String> = vec![
            "existing_1".into(),
            "existing_2".into(),
            "existing_3".into(),
        ];
        ids_900.extend((0..897).map(|i| format!("chunk_900_{}", i)));
        let result = repo.filter_new_message_ids(&ids_900).await.unwrap();
        assert_eq!(result.len(), 897);

        // CHUNK_SIZE 超え (1000件): 既存3 + 新規997
        let mut ids_1000: Vec<String> = vec![
            "existing_1".into(),
            "existing_2".into(),
            "existing_3".into(),
        ];
        ids_1000.extend((0..997).map(|i| format!("chunk_1000_{}", i)));
        let result = repo.filter_new_message_ids(&ids_1000).await.unwrap();
        assert_eq!(result.len(), 997);

        // 2000件超: 既存3 + 新規2000
        let mut ids_2000: Vec<String> = vec![
            "existing_1".into(),
            "existing_2".into(),
            "existing_3".into(),
        ];
        ids_2000.extend((0..2000).map(|i| format!("chunk_2000_{}", i)));
        let result = repo.filter_new_message_ids(&ids_2000).await.unwrap();
        assert_eq!(result.len(), 2000);
    }

    #[tokio::test]
    async fn test_email_stats_repository_get_stats() {
        let pool = setup_test_db().await;
        let repo = SqliteEmailStatsRepository::new(pool.clone());

        // 空の状態での統計
        let stats = repo.get_email_stats().await.unwrap();
        assert_eq!(stats.total_emails, 0);
        assert_eq!(stats.with_body_plain, 0);
        assert_eq!(stats.with_body_html, 0);
        assert_eq!(stats.without_body, 0);
        assert_eq!(stats.avg_plain_length, 0.0);
        assert_eq!(stats.avg_html_length, 0.0);

        // テストデータを追加
        sqlx::query(
            r#"
            INSERT INTO emails (message_id, body_plain, body_html, from_address, subject)
            VALUES
                ('msg1', 'Plain text body', NULL, 'test1@example.com', 'Subject 1'),
                ('msg2', NULL, '<p>HTML body</p>', 'test2@example.com', 'Subject 2'),
                ('msg3', 'Another plain', '<p>Another HTML</p>', 'test3@example.com', 'Subject 3'),
                ('msg4', NULL, NULL, 'test4@example.com', 'Subject 4')
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to insert test emails");

        // 統計を再取得
        let stats = repo.get_email_stats().await.unwrap();
        assert_eq!(stats.total_emails, 4);
        assert_eq!(stats.with_body_plain, 2); // msg1, msg3
        assert_eq!(stats.with_body_html, 2); // msg2, msg3
        assert_eq!(stats.without_body, 1); // msg4
        assert!(stats.avg_plain_length > 0.0);
        assert!(stats.avg_html_length > 0.0);
    }

    #[tokio::test]
    async fn test_order_repository_save_new_order() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // テスト用のメールを追加
        sqlx::query("INSERT INTO emails (message_id, body_plain, from_address, subject) VALUES ('test-email-1', 'body', 'test@example.com', 'Subject')")
            .execute(&pool)
            .await
            .expect("Failed to insert test email");

        let email_id: (i64,) =
            sqlx::query_as("SELECT id FROM emails WHERE message_id = 'test-email-1'")
                .fetch_one(&pool)
                .await
                .expect("Failed to get email id");

        // 新しい注文情報を作成
        use crate::parsers::{DeliveryInfo, OrderInfo, OrderItem};
        let order_info = OrderInfo {
            order_number: "ORD-001".to_string(),
            order_date: Some("2024-01-01".to_string()),
            delivery_address: None,
            delivery_info: Some(DeliveryInfo {
                carrier: "ヤマト運輸".to_string(),
                tracking_number: "1234567890".to_string(),
                delivery_date: None,
                delivery_time: None,
                carrier_url: None,
            }),
            items: vec![
                OrderItem {
                    name: "商品A".to_string(),
                    manufacturer: Some("メーカーA".to_string()),
                    model_number: None,
                    unit_price: 1000,
                    quantity: 2,
                    subtotal: 2000,
                },
                OrderItem {
                    name: "商品B".to_string(),
                    manufacturer: None,
                    model_number: None,
                    unit_price: 500,
                    quantity: 1,
                    subtotal: 500,
                },
            ],
            subtotal: Some(2500),
            shipping_fee: Some(500),
            total_amount: Some(3000),
        };

        // 注文を保存
        let order_id = repo
            .save_order(
                &order_info,
                Some(email_id.0),
                Some("example.com".to_string()),
                Some("Test Shop".to_string()),
            )
            .await
            .unwrap();

        // 検証: ordersテーブル
        let order: (String, Option<String>, Option<String>, Option<String>) = sqlx::query_as(
            "SELECT order_number, order_date, shop_domain, shop_name FROM orders WHERE id = ?",
        )
        .bind(order_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch order");
        assert_eq!(order.0, "ORD-001");
        assert_eq!(order.1, Some("2024-01-01".to_string()));
        assert_eq!(order.2, Some("example.com".to_string()));
        assert_eq!(order.3, Some("Test Shop".to_string()));

        // 検証: itemsテーブル
        let items: Vec<(String, Option<String>, i64, i64)> = sqlx::query_as(
            "SELECT item_name, brand, price, quantity FROM items WHERE order_id = ? ORDER BY item_name",
        )
        .bind(order_id)
        .fetch_all(&pool)
        .await
        .expect("Failed to fetch items");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].0, "商品A");
        assert_eq!(items[0].1, Some("メーカーA".to_string()));
        assert_eq!(items[0].2, 1000);
        assert_eq!(items[0].3, 2);

        // 検証: deliveriesテーブル
        let delivery: (String, String, String) = sqlx::query_as(
            "SELECT tracking_number, carrier, delivery_status FROM deliveries WHERE order_id = ?",
        )
        .bind(order_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch delivery");
        assert_eq!(delivery.0, "1234567890");
        assert_eq!(delivery.1, "ヤマト運輸");
        assert_eq!(delivery.2, "shipped");

        // 検証: order_emailsテーブル
        let link: (i64, i64) = sqlx::query_as(
            "SELECT order_id, email_id FROM order_emails WHERE order_id = ? AND email_id = ?",
        )
        .bind(order_id)
        .bind(email_id.0)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch order_email link");
        assert_eq!(link.0, order_id);
        assert_eq!(link.1, email_id.0);
    }

    #[tokio::test]
    async fn test_parse_repository_get_unparsed_emails_and_clear() {
        let pool = setup_test_db().await;
        let repo = SqliteParseRepository::new(pool.clone());

        // テスト用のメールを追加
        sqlx::query(
            r#"
            INSERT INTO emails (message_id, body_plain, from_address, subject, internal_date)
            VALUES
                ('email1', 'body1', 'test1@example.com', 'Subject 1', 1000),
                ('email2', 'body2', 'test2@example.com', 'Subject 2', 2000),
                ('email3', 'body3', 'test3@example.com', 'Subject 3', 3000)
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to insert test emails");

        // 未パースのメールを取得
        let emails = repo.get_unparsed_emails(10).await.unwrap();
        assert_eq!(emails.len(), 3);

        // 注文を作成してemail1をパース済みにする
        sqlx::query(
            "INSERT INTO orders (order_number, shop_domain) VALUES ('ORD-001', 'example.com')",
        )
        .execute(&pool)
        .await
        .expect("Failed to insert order");

        let order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = 'ORD-001'")
                .fetch_one(&pool)
                .await
                .expect("Failed to get order id");

        let email_id: (i64,) = sqlx::query_as("SELECT id FROM emails WHERE message_id = 'email1'")
            .fetch_one(&pool)
            .await
            .expect("Failed to get email id");

        sqlx::query("INSERT INTO order_emails (order_id, email_id) VALUES (?, ?)")
            .bind(order_id.0)
            .bind(email_id.0)
            .execute(&pool)
            .await
            .expect("Failed to link order to email");

        // 未パースのメールを再取得（email1は除外される）
        let emails = repo.get_unparsed_emails(10).await.unwrap();
        assert_eq!(emails.len(), 2);

        // 全メール数を取得
        let total = repo.get_total_email_count().await.unwrap();
        assert_eq!(total, 3);

        // テーブルをクリア
        repo.clear_order_tables().await.unwrap();

        // クリア後、未パースのメールは再び3件になる
        let emails = repo.get_unparsed_emails(10).await.unwrap();
        assert_eq!(emails.len(), 3);
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

    // =========================================================================
    // ProductMasterRepository テスト
    // =========================================================================

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

    // --- apply_cancel 統合テスト ---

    #[tokio::test]
    async fn test_apply_cancel_quantity_decrease() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // 注文と商品を直接挿入
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('99-1111-1111', '1999.co.jp', 'ホビーサーチ')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-1111-1111'")
                .fetch_one(&pool)
                .await
                .expect("get order id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品A', 2)"#)
            .bind(order_id.0)
            .execute(&pool)
            .await
            .expect("insert item");
        sqlx::query("INSERT INTO emails (message_id, body_plain) VALUES ('cancel-email-1', '')")
            .execute(&pool)
            .await
            .expect("insert email");
        let email_id: (i64,) =
            sqlx::query_as("SELECT id FROM emails WHERE message_id = 'cancel-email-1'")
                .fetch_one(&pool)
                .await
                .expect("get email id");

        let cancel_info = CancelInfo {
            order_number: "99-1111-1111".to_string(),
            product_name: "商品A".to_string(),
            cancel_quantity: 1,
        };
        let result = repo
            .apply_cancel(
                &cancel_info,
                email_id.0,
                Some("1999.co.jp".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), order_id.0);

        let qty: (i64,) =
            sqlx::query_as("SELECT quantity FROM items WHERE order_id = ? AND item_name = '商品A'")
                .bind(order_id.0)
                .fetch_one(&pool)
                .await
                .expect("get item");
        assert_eq!(qty.0, 1);

        // order_emails に (order_id, email_id) が1件挿入されること
        let link_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM order_emails WHERE order_id = ? AND email_id = ?")
                .bind(order_id.0)
                .bind(email_id.0)
                .fetch_one(&pool)
                .await
                .expect("count order_emails");
        assert_eq!(link_count.0, 1, "order_emails should have 1 link");
    }

    #[tokio::test]
    async fn test_apply_cancel_order_emails_no_duplicate() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('99-4444-4444', '1999.co.jp', 'ホビーサーチ')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-4444-4444'")
                .fetch_one(&pool)
                .await
                .expect("get order id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品D', 2)"#)
            .bind(order_id.0)
            .execute(&pool)
            .await
            .expect("insert item");
        sqlx::query("INSERT INTO emails (message_id, body_plain) VALUES ('cancel-email-5', '')")
            .execute(&pool)
            .await
            .expect("insert email");
        let email_id: (i64,) =
            sqlx::query_as("SELECT id FROM emails WHERE message_id = 'cancel-email-5'")
                .fetch_one(&pool)
                .await
                .expect("get email id");

        let cancel_info = CancelInfo {
            order_number: "99-4444-4444".to_string(),
            product_name: "商品D".to_string(),
            cancel_quantity: 1,
        };

        // 1回目: 数量 2 -> 1
        repo.apply_cancel(
            &cancel_info,
            email_id.0,
            Some("1999.co.jp".to_string()),
            None,
        )
        .await
        .expect("first apply");
        // 2回目: 同一 email_id で再度適用 → 数量 1 -> 0、order_emails は重複しない
        repo.apply_cancel(
            &cancel_info,
            email_id.0,
            Some("1999.co.jp".to_string()),
            None,
        )
        .await
        .expect("second apply");

        let link_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM order_emails WHERE order_id = ? AND email_id = ?")
                .bind(order_id.0)
                .bind(email_id.0)
                .fetch_one(&pool)
                .await
                .expect("count order_emails");
        assert_eq!(link_count.0, 1, "order_emails should not have duplicate");
    }

    #[tokio::test]
    async fn test_apply_cancel_item_removed_when_quantity_zero() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('99-2222-2222', '1999.co.jp', 'ホビーサーチ')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-2222-2222'")
                .fetch_one(&pool)
                .await
                .expect("get order id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品B', 1)"#)
            .bind(order_id.0)
            .execute(&pool)
            .await
            .expect("insert item");
        sqlx::query("INSERT INTO emails (message_id, body_plain) VALUES ('cancel-email-2', '')")
            .execute(&pool)
            .await
            .expect("insert email");
        let email_id: (i64,) =
            sqlx::query_as("SELECT id FROM emails WHERE message_id = 'cancel-email-2'")
                .fetch_one(&pool)
                .await
                .expect("get email id");

        let cancel_info = CancelInfo {
            order_number: "99-2222-2222".to_string(),
            product_name: "商品B".to_string(),
            cancel_quantity: 1,
        };
        let result = repo
            .apply_cancel(
                &cancel_info,
                email_id.0,
                Some("1999.co.jp".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(order_id.0)
            .fetch_one(&pool)
            .await
            .expect("count items");
        assert_eq!(count.0, 0, "item should be deleted when quantity becomes 0");
    }

    #[tokio::test]
    async fn test_apply_cancel_order_not_found() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        sqlx::query("INSERT INTO emails (message_id, body_plain) VALUES ('cancel-email-3', '')")
            .execute(&pool)
            .await
            .expect("insert email");
        let email_id: (i64,) =
            sqlx::query_as("SELECT id FROM emails WHERE message_id = 'cancel-email-3'")
                .fetch_one(&pool)
                .await
                .expect("get email id");

        let cancel_info = CancelInfo {
            order_number: "99-9999-9999".to_string(),
            product_name: "商品X".to_string(),
            cancel_quantity: 1,
        };
        let result = repo
            .apply_cancel(
                &cancel_info,
                email_id.0,
                Some("1999.co.jp".to_string()),
                None,
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_apply_cancel_product_not_found() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('99-3333-3333', '1999.co.jp', 'ホビーサーチ')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-3333-3333'")
                .fetch_one(&pool)
                .await
                .expect("get order id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品C', 1)"#)
            .bind(order_id.0)
            .execute(&pool)
            .await
            .expect("insert item");
        sqlx::query("INSERT INTO emails (message_id, body_plain) VALUES ('cancel-email-4', '')")
            .execute(&pool)
            .await
            .expect("insert email");
        let email_id: (i64,) =
            sqlx::query_as("SELECT id FROM emails WHERE message_id = 'cancel-email-4'")
                .fetch_one(&pool)
                .await
                .expect("get email id");

        let cancel_info = CancelInfo {
            order_number: "99-3333-3333".to_string(),
            product_name: "存在しない商品名".to_string(),
            cancel_quantity: 1,
        };
        let result = repo
            .apply_cancel(
                &cancel_info,
                email_id.0,
                Some("1999.co.jp".to_string()),
                None,
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_apply_cancel_invalid_quantity() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('99-5555-5555', '1999.co.jp', 'ホビーサーチ')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        sqlx::query(
            r#"INSERT INTO items (order_id, item_name, quantity) SELECT id, '商品E', 1 FROM orders WHERE order_number = '99-5555-5555'"#,
        )
        .execute(&pool)
        .await
        .expect("insert item");
        sqlx::query("INSERT INTO emails (message_id, body_plain) VALUES ('cancel-email-6', '')")
            .execute(&pool)
            .await
            .expect("insert email");
        let email_id: (i64,) =
            sqlx::query_as("SELECT id FROM emails WHERE message_id = 'cancel-email-6'")
                .fetch_one(&pool)
                .await
                .expect("get email id");

        let cancel_info = CancelInfo {
            order_number: "99-5555-5555".to_string(),
            product_name: "商品E".to_string(),
            cancel_quantity: 0,
        };
        let result = repo
            .apply_cancel(
                &cancel_info,
                email_id.0,
                Some("1999.co.jp".to_string()),
                None,
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid cancel quantity"));
    }

    // --- apply_change_items 統合テスト ---

    #[tokio::test]
    async fn test_apply_change_items_removes_item_from_old_order() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // 元注文 (order_number 99-1000-0001) に商品A を追加
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('99-1000-0001', '1999.co.jp', 'ホビーサーチ')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let old_order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-1000-0001'")
                .fetch_one(&pool)
                .await
                .expect("get order id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品A', 1)"#)
            .bind(old_order_id.0)
            .execute(&pool)
            .await
            .expect("insert item");

        // 組み換え後の新注文情報（商品A が新注文に含まれる）
        let order_info = crate::parsers::OrderInfo {
            order_number: "25-0918-1710".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![crate::parsers::OrderItem {
                name: "商品A".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 1000,
                quantity: 1,
                subtotal: 1000,
            }],
            subtotal: Some(1000),
            shipping_fee: None,
            total_amount: Some(1000),
        };

        let result = repo
            .apply_change_items(&order_info, Some("1999.co.jp".to_string()), None)
            .await;
        assert!(result.is_ok());

        // 元注文から商品A が削除されていること
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(old_order_id.0)
            .fetch_one(&pool)
            .await
            .expect("count items");
        assert_eq!(count.0, 0, "item should be removed from old order");

        // 残り商品 0 で元注文が削除されていること
        let order_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE id = ?")
            .bind(old_order_id.0)
            .fetch_one(&pool)
            .await
            .expect("check order");
        assert_eq!(order_exists.0, 0, "empty order should be deleted");
    }

    #[tokio::test]
    async fn test_apply_change_items_ignores_shipped_orders() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // 発送済みの注文（deliveries に shipped あり）
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('99-2000-0001', '1999.co.jp', 'ホビーサーチ')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let shipped_order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-2000-0001'")
                .fetch_one(&pool)
                .await
                .expect("get order id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品X', 1)"#)
            .bind(shipped_order_id.0)
            .execute(&pool)
            .await
            .expect("insert item");
        sqlx::query(
            r#"INSERT INTO deliveries (order_id, tracking_number, carrier, delivery_status) VALUES (?, '123456', '佐川', 'shipped')"#,
        )
        .bind(shipped_order_id.0)
        .execute(&pool)
        .await
        .expect("insert delivery");

        let order_info = crate::parsers::OrderInfo {
            order_number: "25-0918-1710".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![crate::parsers::OrderItem {
                name: "商品X".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 500,
                quantity: 1,
                subtotal: 500,
            }],
            subtotal: Some(500),
            shipping_fee: None,
            total_amount: Some(500),
        };

        let result = repo
            .apply_change_items(&order_info, Some("1999.co.jp".to_string()), None)
            .await;
        assert!(result.is_ok());

        // 発送済み注文の商品は削除されないこと
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(shipped_order_id.0)
            .fetch_one(&pool)
            .await
            .expect("count items");
        assert_eq!(count.0, 1, "shipped order items should not be removed");
    }

    #[tokio::test]
    async fn test_apply_change_items_no_match_still_succeeds() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // 元注文に商品A がない
        let order_info = crate::parsers::OrderInfo {
            order_number: "25-0918-1710".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![crate::parsers::OrderItem {
                name: "存在しない商品".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 100,
                quantity: 1,
                subtotal: 100,
            }],
            subtotal: Some(100),
            shipping_fee: None,
            total_amount: Some(100),
        };

        // マッチする注文がなくても Err は返さない（フォールバック設計）
        let result = repo
            .apply_change_items(&order_info, Some("1999.co.jp".to_string()), None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_apply_change_items_reduces_quantity() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // 元注文に商品A が2個
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('99-3000-0001', '1999.co.jp', 'ホビーサーチ')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let old_order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-3000-0001'")
                .fetch_one(&pool)
                .await
                .expect("get order id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品A', 2)"#)
            .bind(old_order_id.0)
            .execute(&pool)
            .await
            .expect("insert item");

        // 組み換え後の新注文（商品A が1個のみ → 元注文の quantity が 2 -> 1 に減算）
        let order_info = crate::parsers::OrderInfo {
            order_number: "25-0918-1710".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![crate::parsers::OrderItem {
                name: "商品A".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 1000,
                quantity: 1,
                subtotal: 1000,
            }],
            subtotal: Some(1000),
            shipping_fee: None,
            total_amount: Some(1000),
        };

        let result = repo
            .apply_change_items(&order_info, Some("1999.co.jp".to_string()), None)
            .await;
        assert!(result.is_ok());

        // 元注文の商品A が quantity 1 に減算されていること
        let (qty,): (i64,) =
            sqlx::query_as("SELECT quantity FROM items WHERE order_id = ? AND item_name = '商品A'")
                .bind(old_order_id.0)
                .fetch_one(&pool)
                .await
                .expect("get quantity");
        assert_eq!(qty, 1, "quantity should be reduced from 2 to 1");

        // 元注文は残っていること（商品がまだ1個あるため削除されない）
        let order_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE id = ?")
            .bind(old_order_id.0)
            .fetch_one(&pool)
            .await
            .expect("check order");
        assert_eq!(
            order_exists.0, 1,
            "order should remain with remaining items"
        );
    }

    #[tokio::test]
    async fn test_apply_change_items_spans_multiple_orders() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // 元注文1: 商品A が1個
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('99-4000-0001', '1999.co.jp', 'ホビーサーチ')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order 1");
        let order1_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-4000-0001'")
                .fetch_one(&pool)
                .await
                .expect("get order 1 id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品A', 1)"#)
            .bind(order1_id.0)
            .execute(&pool)
            .await
            .expect("insert item");

        // 元注文2: 商品A が1個
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('99-4000-0002', '1999.co.jp', 'ホビーサーチ')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order 2");
        let order2_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-4000-0002'")
                .fetch_one(&pool)
                .await
                .expect("get order 2 id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品A', 1)"#)
            .bind(order2_id.0)
            .execute(&pool)
            .await
            .expect("insert item");

        // 組み換え後の新注文（商品A が2個 → 2つの元注文から各1個ずつ削除）
        let order_info = crate::parsers::OrderInfo {
            order_number: "25-0918-1710".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![crate::parsers::OrderItem {
                name: "商品A".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 1000,
                quantity: 2,
                subtotal: 2000,
            }],
            subtotal: Some(2000),
            shipping_fee: None,
            total_amount: Some(2000),
        };

        let result = repo
            .apply_change_items(&order_info, Some("1999.co.jp".to_string()), None)
            .await;
        assert!(result.is_ok());

        // 両方の元注文から商品A が削除されていること
        let count1: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(order1_id.0)
            .fetch_one(&pool)
            .await
            .expect("count order 1 items");
        let count2: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(order2_id.0)
            .fetch_one(&pool)
            .await
            .expect("count order 2 items");
        assert_eq!(count1.0, 0, "order 1 items should be removed");
        assert_eq!(count2.0, 0, "order 2 items should be removed");

        // 両方の元注文が削除されていること（残り商品0）
        let order1_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE id = ?")
            .bind(order1_id.0)
            .fetch_one(&pool)
            .await
            .expect("check order 1");
        let order2_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE id = ?")
            .bind(order2_id.0)
            .fetch_one(&pool)
            .await
            .expect("check order 2");
        assert_eq!(order1_exists.0, 0, "empty order 1 should be deleted");
        assert_eq!(order2_exists.0, 0, "empty order 2 should be deleted");
    }

    #[tokio::test]
    async fn test_apply_change_items_consumes_multiple_rows_in_same_order() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // 同一注文内に同名商品が複数行（商品A×1 が2行）
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('99-4500-0001', '1999.co.jp', 'ホビーサーチ')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-4500-0001'")
                .fetch_one(&pool)
                .await
                .expect("get order id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品A', 1)"#)
            .bind(order_id.0)
            .execute(&pool)
            .await
            .expect("insert item 1");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品A', 1)"#)
            .bind(order_id.0)
            .execute(&pool)
            .await
            .expect("insert item 2");

        // 組み換え後は商品A が2個 → 同一注文内の2行から各1個ずつ消費
        let order_info = crate::parsers::OrderInfo {
            order_number: "25-0918-1710".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![crate::parsers::OrderItem {
                name: "商品A".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 1000,
                quantity: 2,
                subtotal: 2000,
            }],
            subtotal: Some(2000),
            shipping_fee: None,
            total_amount: Some(2000),
        };

        let result = repo
            .apply_change_items(&order_info, Some("1999.co.jp".to_string()), None)
            .await;
        assert!(result.is_ok());

        // 同一注文内の2行とも削除され、注文が削除されていること
        let item_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(order_id.0)
            .fetch_one(&pool)
            .await
            .expect("count items");
        assert_eq!(item_count.0, 0, "both rows should be consumed");
        let order_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE id = ?")
            .bind(order_id.0)
            .fetch_one(&pool)
            .await
            .expect("check order");
        assert_eq!(order_exists.0, 0, "empty order should be deleted");
    }

    // --- apply_change_items_and_save_order 統合テスト ---

    #[tokio::test]
    async fn test_apply_change_items_and_save_order_atomic_success() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // 元注文とメールをセットアップ
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('99-5000-0001', '1999.co.jp', 'ホビーサーチ')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let old_order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-5000-0001'")
                .fetch_one(&pool)
                .await
                .expect("get order id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品B', 1)"#)
            .bind(old_order_id.0)
            .execute(&pool)
            .await
            .expect("insert item");
        sqlx::query("INSERT INTO emails (message_id, body_plain) VALUES ('change-email-1', '')")
            .execute(&pool)
            .await
            .expect("insert email");
        let email_id: (i64,) =
            sqlx::query_as("SELECT id FROM emails WHERE message_id = 'change-email-1'")
                .fetch_one(&pool)
                .await
                .expect("get email id");

        let order_info = crate::parsers::OrderInfo {
            order_number: "25-0918-1710".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![crate::parsers::OrderItem {
                name: "商品B".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 500,
                quantity: 1,
                subtotal: 500,
            }],
            subtotal: Some(500),
            shipping_fee: None,
            total_amount: Some(500),
        };

        let result = repo
            .apply_change_items_and_save_order(
                &order_info,
                Some(email_id.0),
                Some("1999.co.jp".to_string()),
                Some("ホビーサーチ".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());
        let new_order_id = result.unwrap();

        // 元注文から商品が削除され、空注文が削除されていること
        let old_order_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE id = ?")
            .bind(old_order_id.0)
            .fetch_one(&pool)
            .await
            .expect("check old order");
        assert_eq!(old_order_exists.0, 0, "old order should be deleted");

        // 新注文が保存され、商品が含まれていること
        let new_order_items: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
                .bind(new_order_id)
                .fetch_one(&pool)
                .await
                .expect("count new order items");
        assert_eq!(new_order_items.0, 1, "new order should have 1 item");

        // order_emails に紐づいていること
        let link_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM order_emails WHERE order_id = ? AND email_id = ?")
                .bind(new_order_id)
                .bind(email_id.0)
                .fetch_one(&pool)
                .await
                .expect("count order_emails");
        assert_eq!(link_count.0, 1, "order_emails should link new order to email");
    }

    #[tokio::test]
    async fn test_apply_change_items_and_save_order_rollback_on_save_failure() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // 元注文をセットアップ（email は作成しない → 存在しない email_id を渡す）
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('99-6000-0001', '1999.co.jp', 'ホビーサーチ')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let old_order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-6000-0001'")
                .fetch_one(&pool)
                .await
                .expect("get order id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品C', 1)"#)
            .bind(old_order_id.0)
            .execute(&pool)
            .await
            .expect("insert item");

        // 存在しない email_id を渡す（order_emails INSERT で FK 違反 → トランザクションロールバック）
        let order_info = crate::parsers::OrderInfo {
            order_number: "25-0918-1710".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![crate::parsers::OrderItem {
                name: "商品C".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 300,
                quantity: 1,
                subtotal: 300,
            }],
            subtotal: Some(300),
            shipping_fee: None,
            total_amount: Some(300),
        };

        let result = repo
            .apply_change_items_and_save_order(
                &order_info,
                Some(99999), // 存在しない email_id
                Some("1999.co.jp".to_string()),
                Some("ホビーサーチ".to_string()),
                None,
            )
            .await;
        assert!(result.is_err());

        // ロールバックにより元注文の商品が残っていること
        let item_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(old_order_id.0)
            .fetch_one(&pool)
            .await
            .expect("count items");
        assert_eq!(item_count.0, 1, "old order items should remain after rollback");
    }
}
