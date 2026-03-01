use crate::gemini::normalize_product_name;
use crate::parsers::cancel_info::CancelInfo;
use crate::parsers::consolidation_info::ConsolidationInfo;
use crate::parsers::order_number_change_info::OrderNumberChangeInfo;
use crate::parsers::OrderInfo;
use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use once_cell::sync::Lazy;
use regex::Regex;
use sqlx::sqlite::{Sqlite, SqlitePool};
use std::collections::{HashMap, HashSet};

type ItemRow = (i64, i64, String, Option<String>, Option<String>, i64);

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
    /// * `alternate_domains`: 検索失敗時に試す追加ドメイン（店舗固有、DMM の mail/mono 等）
    async fn apply_cancel(
        &self,
        cancel_info: &CancelInfo,
        email_id: i64,
        shop_domain: Option<String>,
        shop_name: Option<String>,
        alternate_domains: Option<Vec<String>>,
    ) -> Result<i64, String>;

    /// 注文番号変更メールの内容を適用（旧番号の注文を新番号に更新）
    /// * `alternate_domains`: 検索失敗時に試す追加ドメイン（店舗固有、DMM の mail/mono 等）
    async fn apply_order_number_change(
        &self,
        change_info: &OrderNumberChangeInfo,
        email_id: i64,
        change_email_internal_date: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
        alternate_domains: Option<Vec<String>>,
    ) -> Result<i64, String>;

    /// 組み換えメールの商品を元注文から削除する。
    /// 新注文の各商品について、同じショップの過去注文（発送済みでない）から商品名でマッチする item を検索し削除する。
    /// 残り商品が 0 になった order は deliveries のみクリーンアップし、orders/order_emails は再パース防止のため保持する。
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

    /// 分割完了メール用: 先頭の注文を「元注文」として扱い、既存注文があれば商品を差し替え、なければ新規登録する。
    /// * `alternate_domains`: 検索失敗時に試す追加ドメイン（DMM の mail.dmm.com / mono.dmm.com 等）
    async fn apply_split_first_order(
        &self,
        order_info: &OrderInfo,
        email_id: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
        alternate_domains: Option<Vec<String>>,
    ) -> Result<i64, String>;

    /// まとめ完了メール用: 複数注文を1注文に統合。先頭の注文を新番号に更新し、残りは商品を削除（注文は保持）。
    /// * `alternate_domains`: 検索失敗時に試す追加ドメイン（DMM の mail.dmm.com / mono.dmm.com 等）
    async fn apply_consolidation(
        &self,
        consolidation_info: &ConsolidationInfo,
        email_id: i64,
        shop_domain: Option<String>,
        shop_name: Option<String>,
        alternate_domains: Option<Vec<String>>,
    ) -> Result<i64, String>;

    /// 発送完了メール用: 発送メールに記載された商品・金額を最終状態として扱い、
    /// 既存注文の items を置き換え、delivery 情報を更新する。
    /// * `alternate_domains`: 検索失敗時に試す追加ドメイン（DMM の mail.dmm.com / mono.dmm.com 等）
    async fn apply_send_and_replace_items(
        &self,
        order_info: &OrderInfo,
        email_id: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
        alternate_domains: Option<Vec<String>>,
    ) -> Result<i64, String>;
}

/// 商品名比較用に【】[]（）() で囲まれた部分を除去する
fn strip_bracketed_content(s: &str) -> String {
    static RE: Lazy<Regex> = Lazy::new(|| {
        // 【】[]（）() とその囲まれた内容を除去
        Regex::new(r"【[^】]*】|\[[^\]]*\]|（[^）]*）|\([^)]*\)")
            .expect("strip_bracketed_content regex")
    });
    RE.replace_all(s, "").trim().to_string()
}

/// 商品名がマッチするか判定（apply_cancel / apply_change_items で共通利用）
///
/// # 引数
/// - `product_name`: 受信メール由来のアイテム名
/// - `product_master_name`: `product_master.product_name`（受信アイテム側。未登録の場合は None）
/// - `item_name`: DB に保存済みのアイテム名
/// - `item_name_normalized`: DB に保存済みの正規化名
/// - `item_product_master_name`: `product_master.product_name`（DB アイテム側。未登録の場合は None）
fn item_names_match(
    product_name: &str,
    product_master_name: Option<&str>,
    item_name: &str,
    item_name_normalized: Option<&str>,
    item_product_master_name: Option<&str>,
) -> bool {
    let product_name_core = product_name
        .trim_end_matches(" (プラモデル)")
        .trim_end_matches(" (ディスプレイ)")
        .trim();
    let product_name_stripped = strip_bracketed_content(product_name);
    let product_normalized = normalize_product_name(product_name);

    let item_trimmed = item_name.trim();
    let item_stripped = strip_bracketed_content(item_trimmed);

    // パターン1: 完全一致
    if item_trimmed == product_name || item_trimmed == product_name_core {
        return true;
    }
    // パターン2: 包含関係・括弧除去後の部分一致
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
    // パターン3: 正規化名の部分一致（空同士は誤マッチ防止のため除外）
    let db_normalized = item_name_normalized
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| normalize_product_name(item_name));
    if !product_normalized.is_empty()
        && !db_normalized.is_empty()
        && (product_normalized == db_normalized
            || product_normalized.contains(db_normalized.as_str())
            || db_normalized.contains(product_normalized.as_str()))
    {
        return true;
    }
    // パターン4: product_master による突合せ（商品コード差異等を吸収）
    // 両方の product_master.product_name が非空で一致すれば同一商品とみなす
    if let (Some(pm_in), Some(pm_db)) = (product_master_name, item_product_master_name) {
        if !pm_in.is_empty() && !pm_db.is_empty() && pm_in == pm_db {
            return true;
        }
    }
    false
}

/// apply_change_items で order_id ごとの items を保持する型
/// (item_id, item_name, item_name_normalized, product_master_name, quantity)
type ItemsByOrderMap = HashMap<i64, Vec<(i64, String, Option<String>, Option<String>, i64)>>;

/// SQLiteを使用したOrderRepositoryの実装
pub struct SqliteOrderRepository {
    pool: SqlitePool,
}

impl SqliteOrderRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 注文番号＋ドメインで注文IDを検索。alternate_domains が渡された場合、検索失敗時に追加ドメインで再試行。
    pub(crate) async fn find_order_by_number_and_domain(
        tx: &mut sqlx::Transaction<'_, Sqlite>,
        order_number: &str,
        shop_domain: &Option<String>,
        alternate_domains: Option<&[String]>,
    ) -> Result<Option<i64>, sqlx::Error> {
        let mut domains_to_try: Vec<Option<String>> = match shop_domain {
            Some(d) if !d.is_empty() => vec![Some(d.clone())],
            _ => vec![None],
        };
        if let Some(alts) = alternate_domains {
            for alt in alts.iter() {
                if !alt.is_empty() {
                    domains_to_try.push(Some(alt.clone()));
                }
            }
        }
        for domain_opt in domains_to_try {
            let row: Option<(i64,)> = match &domain_opt {
                Some(domain) => {
                    sqlx::query_as(
                        r#"
                        SELECT id FROM orders
                        WHERE order_number COLLATE NOCASE = ? AND shop_domain = ?
                        LIMIT 1
                        "#,
                    )
                    .bind(order_number)
                    .bind(domain)
                    .fetch_optional(tx.as_mut())
                    .await?
                }
                None => {
                    sqlx::query_as(
                        r#"
                        SELECT id FROM orders
                        WHERE order_number COLLATE NOCASE = ? AND (shop_domain IS NULL OR shop_domain = '')
                        LIMIT 1
                        "#,
                    )
                    .bind(order_number)
                    .fetch_optional(tx.as_mut())
                    .await?
                }
            };
            if let Some((id,)) = row {
                return Ok(Some(id));
            }
        }
        Ok(None)
    }

    /// apply_change_items のトランザクション内ロジック（tx は呼び出し元で commit）
    pub(crate) async fn apply_change_items_in_tx(
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
                    WHERE o.order_number COLLATE NOCASE != ?
                    AND o.shop_domain = ?
                    AND o.id NOT IN (
                        SELECT d.order_id FROM deliveries d
                        WHERE d.delivery_status IN ('shipped', 'in_transit', 'out_for_delivery', 'delivered')
                    )
                    AND (
                        (o.order_date IS NOT NULL AND o.order_date < datetime(? / 1000, 'unixepoch', '+9 hours'))
                        OR (o.order_date IS NULL AND o.created_at < datetime(? / 1000, 'unixepoch'))
                    )
                    ORDER BY o.order_date IS NULL, o.order_date DESC, o.id DESC
                    "#,
                )
                .bind(new_order_number)
                .bind(d)
                .bind(cutoff_ts)
                .bind(cutoff_ts)
                .fetch_all(tx.as_mut())
                .await
                .map_err(|e| format!("Failed to fetch change-target orders: {e}"))?
            } else {
                sqlx::query_scalar(
                    r#"
                    SELECT o.id FROM orders o
                    WHERE o.order_number COLLATE NOCASE != ?
                    AND (o.shop_domain IS NULL OR o.shop_domain = '')
                    AND o.id NOT IN (
                        SELECT d.order_id FROM deliveries d
                        WHERE d.delivery_status IN ('shipped', 'in_transit', 'out_for_delivery', 'delivered')
                    )
                    AND (
                        (o.order_date IS NOT NULL AND o.order_date < datetime(? / 1000, 'unixepoch', '+9 hours'))
                        OR (o.order_date IS NULL AND o.created_at < datetime(? / 1000, 'unixepoch'))
                    )
                    ORDER BY o.order_date IS NULL, o.order_date DESC, o.id DESC
                    "#,
                )
                .bind(new_order_number)
                .bind(cutoff_ts)
                .bind(cutoff_ts)
                .fetch_all(tx.as_mut())
                .await
                .map_err(|e| format!("Failed to fetch change-target orders: {e}"))?
            }
        } else {
            sqlx::query_scalar(
                r#"
                SELECT o.id FROM orders o
                WHERE o.order_number COLLATE NOCASE != ?
                AND (o.shop_domain IS NULL OR o.shop_domain = '')
                AND o.id NOT IN (
                    SELECT d.order_id FROM deliveries d
                    WHERE d.delivery_status IN ('shipped', 'in_transit', 'out_for_delivery', 'delivered')
                )
                AND (
                    (o.order_date IS NOT NULL AND o.order_date < datetime(? / 1000, 'unixepoch', '+9 hours'))
                    OR (o.order_date IS NULL AND o.created_at < datetime(? / 1000, 'unixepoch'))
                )
                ORDER BY o.order_date IS NULL, o.order_date DESC, o.id DESC
                "#,
            )
            .bind(new_order_number)
            .bind(cutoff_ts)
            .bind(cutoff_ts)
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to fetch change-target orders: {e}"))?
        };

        // SQLite のバインド変数上限（999）を超えないようチャンク単位で取得
        const BIND_LIMIT: usize = 500;
        let mut items_by_order: ItemsByOrderMap = HashMap::new();
        for chunk in order_ids.chunks(BIND_LIMIT) {
            let placeholders: Vec<String> = chunk.iter().map(|_| "?".to_string()).collect();
            let placeholders_str = placeholders.join(", ");
            let query_str = format!(
                r#"SELECT i.order_id, i.id, i.item_name, i.item_name_normalized, pm.product_name, i.quantity
                   FROM items i
                   LEFT JOIN product_master pm ON TRIM(i.item_name) = pm.raw_name
                   WHERE i.order_id IN ({})
                   ORDER BY i.order_id, i.id"#,
                placeholders_str
            );
            let mut q = sqlx::query_as::<_, ItemRow>(&query_str);
            for id in chunk {
                q = q.bind(id);
            }
            let rows: Vec<ItemRow> = q
                .fetch_all(tx.as_mut())
                .await
                .map_err(|e| format!("Failed to fetch items: {e}"))?;
            for (order_id, id, item_name, item_name_normalized, pm_name, quantity) in rows {
                items_by_order.entry(order_id).or_default().push((
                    id,
                    item_name,
                    item_name_normalized,
                    pm_name,
                    quantity,
                ));
            }
        }

        // 受信アイテムの product_master を一括先引き（商品コード差異等の吸収用）
        let incoming_raw_names: Vec<String> = order_info
            .items
            .iter()
            .filter(|item| !item.name.trim().is_empty())
            .map(|item| item.name.trim().to_string())
            .collect();
        let mut incoming_pm_map: HashMap<String, String> = HashMap::new();
        for chunk in incoming_raw_names.chunks(BIND_LIMIT) {
            let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let sql = format!(
                "SELECT raw_name, product_name FROM product_master WHERE raw_name IN ({}) AND product_name IS NOT NULL",
                placeholders
            );
            let mut query = sqlx::query_as::<_, (String, String)>(&sql);
            for name in chunk {
                query = query.bind(name);
            }
            let rows: Vec<(String, String)> = query
                .fetch_all(tx.as_mut())
                .await
                .map_err(|e| format!("Failed to fetch product_master for incoming items: {e}"))?;
            for (raw_name, product_name) in rows {
                incoming_pm_map.insert(raw_name, product_name);
            }
        }

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

                    let product_master_name = incoming_pm_map.get(product_name).map(|s| s.as_str());
                    let found = items.iter().find(
                        |(_, item_name, item_name_normalized, item_pm_name, _)| {
                            item_names_match(
                                product_name,
                                product_master_name,
                                item_name,
                                item_name_normalized.as_deref(),
                                item_pm_name.as_deref(),
                            )
                        },
                    );

                    if let Some((item_id, _, _, _, current_qty)) = found {
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
                                vec.retain(|(id, _, _, _, _)| *id != item_id);
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
                                    vec.iter_mut().find(|(id, _, _, _, _)| *id == item_id)
                                {
                                    entry.4 = new_qty;
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
                // NOTE: order_emails と orders は物理削除しない。
                // get_unparsed_emails は order_emails の不在を「未パース」のシグナルとして使うため、
                // これらを削除すると過去パース済みメールが未パースに戻り、再パースで元注文が復活する。
                // そのため deliveries のみ削除し、order と order_emails は保持する。
                sqlx::query("DELETE FROM deliveries WHERE order_id = ?")
                    .bind(order_id)
                    .execute(tx.as_mut())
                    .await
                    .map_err(|e| format!("Failed to delete deliveries for empty order: {e}"))?;
                log::info!(
                    "apply_change_items: cleaned up deliveries for empty order {} (order and order_emails retained)",
                    order_id
                );
            }
        }

        Ok(())
    }

    /// save_order のトランザクション内ロジック（tx は呼び出し元で commit）
    pub(crate) async fn save_order_in_tx(
        tx: &mut sqlx::Transaction<'_, Sqlite>,
        order_info: &OrderInfo,
        email_id: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
    ) -> Result<i64, String> {
        // 注文番号は大文字小文字を区別せずマッチ（メールからそのまま保存するため表記が揺れる場合あり）
        let existing_order: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT id FROM orders
            WHERE order_number COLLATE NOCASE = ? AND shop_domain = ?
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

    /// 指定注文の商品を削除し、order_info の商品で置き換える（分割完了の元注文更新用）
    pub(crate) async fn replace_items_for_order_in_tx(
        tx: &mut sqlx::Transaction<'_, Sqlite>,
        order_id: i64,
        order_info: &OrderInfo,
    ) -> Result<(), String> {
        sqlx::query("DELETE FROM items WHERE order_id = ?")
            .bind(order_id)
            .execute(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to delete existing items: {e}"))?;
        log::debug!("Replaced items for order {} (split first order)", order_id);

        for item in &order_info.items {
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
        }
        Ok(())
    }

    /// apply_cancel のトランザクション内ロジック（tx は呼び出し元で commit）
    pub(crate) async fn apply_cancel_in_tx(
        tx: &mut sqlx::Transaction<'_, Sqlite>,
        cancel_info: &CancelInfo,
        email_id: i64,
        shop_domain: Option<String>,
        alternate_domains: Option<Vec<String>>,
    ) -> Result<i64, String> {
        let order_id = match Self::find_order_by_number_and_domain(
            tx,
            &cancel_info.order_number,
            &shop_domain,
            alternate_domains.as_deref(),
        )
        .await
        .map_err(|e| format!("Failed to find order: {e}"))?
        {
            Some(id) => id,
            None => {
                log::warn!(
                    "Cancel mail: order {} not found (shop_domain={:?}, alternate_domains={:?})",
                    cancel_info.order_number,
                    shop_domain,
                    alternate_domains
                );
                return Err(format!(
                    "Order {} not found for cancel",
                    cancel_info.order_number
                ));
            }
        };

        type ItemRow = (i64, String, Option<String>, Option<String>, i64);
        let items: Vec<ItemRow> = sqlx::query_as(
            r#"
            SELECT i.id, i.item_name, i.item_name_normalized, pm.product_name, i.quantity
            FROM items i
            LEFT JOIN product_master pm ON TRIM(i.item_name) = pm.raw_name
            WHERE i.order_id = ?
            ORDER BY i.id
            "#,
        )
        .bind(order_id)
        .fetch_all(tx.as_mut())
        .await
        .map_err(|e| format!("Failed to fetch items: {e}"))?;

        let product_name = cancel_info.product_name.trim();

        let cancel_product_master_name: Option<String> = if !product_name.is_empty() {
            sqlx::query_scalar(
                r#"
                SELECT product_name FROM product_master
                WHERE raw_name = TRIM(?) AND product_name IS NOT NULL
                LIMIT 1
                "#,
            )
            .bind(product_name)
            .fetch_optional(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to fetch product_master for cancel item: {e}"))?
            .flatten()
        } else {
            None
        };

        if product_name.is_empty() {
            sqlx::query("DELETE FROM items WHERE order_id = ?")
                .bind(order_id)
                .execute(tx.as_mut())
                .await
                .map_err(|e| format!("Failed to delete items: {e}"))?;
            log::info!(
                "Cancel applied (entire order): removed {} items from order {}",
                items.len(),
                order_id
            );
        } else {
            let matched =
                items
                    .iter()
                    .find(|(_, item_name, item_name_normalized, item_pm_name, _)| {
                        item_names_match(
                            product_name,
                            cancel_product_master_name.as_deref(),
                            item_name,
                            item_name_normalized.as_deref(),
                            item_pm_name.as_deref(),
                        )
                    });

            match matched {
                Some((item_id, _, _, _, current_qty)) => {
                    let item_id = *item_id;
                    let current_qty = *current_qty;

                    if cancel_info.cancel_quantity <= 0 {
                        log::warn!(
                            "Invalid cancel quantity {} for product '{}' in order {}",
                            cancel_info.cancel_quantity,
                            product_name,
                            order_id
                        );
                        return Err(format!(
                            "Invalid cancel quantity {} for product '{}'",
                            cancel_info.cancel_quantity, product_name
                        ));
                    }

                    let new_qty = current_qty - cancel_info.cancel_quantity;

                    if new_qty <= 0 {
                        sqlx::query("DELETE FROM items WHERE id = ?")
                            .bind(item_id)
                            .execute(tx.as_mut())
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
                            .execute(tx.as_mut())
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
                    return Err(format!("Product '{}' not found in order", product_name));
                }
            }
        }

        let existing_link: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT order_id FROM order_emails
            WHERE order_id = ? AND email_id = ?
            LIMIT 1
            "#,
        )
        .bind(order_id)
        .bind(email_id)
        .fetch_optional(tx.as_mut())
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
            .execute(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to link order to email: {e}"))?;
        }

        Ok(order_id)
    }

    /// apply_order_number_change のトランザクション内ロジック（tx は呼び出し元で commit）
    pub(crate) async fn apply_order_number_change_in_tx(
        tx: &mut sqlx::Transaction<'_, Sqlite>,
        change_info: &OrderNumberChangeInfo,
        email_id: i64,
        change_email_internal_date: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
        alternate_domains: Option<Vec<String>>,
    ) -> Result<i64, String> {
        let existing_order_id = Self::find_order_by_number_and_domain(
            tx,
            &change_info.old_order_number,
            &shop_domain,
            alternate_domains.as_deref(),
        )
        .await
        .map_err(|e| format!("Failed to find order: {e}"))?;

        let order_id = if let Some(id) = existing_order_id {
            id
        } else {
            log::warn!(
                "Order number change: old order {} not found (shop_domain={:?}, alternate_domains={:?}); creating new order with {}",
                change_info.old_order_number,
                shop_domain,
                alternate_domains,
                change_info.new_order_number
            );

            let order_date_str = change_email_internal_date.and_then(|ts| {
                chrono::DateTime::from_timestamp_millis(ts)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            });

            let new_order_id = sqlx::query(
                r#"
                INSERT INTO orders (order_number, order_date, shop_domain, shop_name)
                VALUES (?, ?, ?, ?)
                "#,
            )
            .bind(&change_info.new_order_number)
            .bind(&order_date_str)
            .bind(shop_domain.as_deref())
            .bind(shop_name.as_deref())
            .execute(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to insert order for number change fallback: {e}"))?
            .last_insert_rowid();

            log::info!(
                "Order number change fallback: created new order {} with number {}",
                new_order_id,
                change_info.new_order_number
            );
            new_order_id
        };

        sqlx::query("UPDATE orders SET order_number = ? WHERE id = ?")
            .bind(&change_info.new_order_number)
            .bind(order_id)
            .execute(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to update order number: {e}"))?;
        log::info!(
            "Order number changed: {} -> {} (order_id={})",
            change_info.old_order_number,
            change_info.new_order_number,
            order_id
        );

        let existing_link: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT order_id FROM order_emails
            WHERE order_id = ? AND email_id = ?
            LIMIT 1
            "#,
        )
        .bind(order_id)
        .bind(email_id)
        .fetch_optional(tx.as_mut())
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
            .execute(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to link order to email: {e}"))?;
        }

        Ok(order_id)
    }

    /// apply_consolidation のトランザクション内ロジック（tx は呼び出し元で commit）
    pub(crate) async fn apply_consolidation_in_tx(
        tx: &mut sqlx::Transaction<'_, Sqlite>,
        consolidation_info: &ConsolidationInfo,
        email_id: i64,
        shop_domain: Option<String>,
        alternate_domains: Option<Vec<String>>,
    ) -> Result<i64, String> {
        let mut order_ids: Vec<i64> = Vec::new();
        let mut seen = HashSet::new();
        for old_num in &consolidation_info.old_order_numbers {
            if let Some(id) = Self::find_order_by_number_and_domain(
                tx,
                old_num,
                &shop_domain,
                alternate_domains.as_deref(),
            )
            .await
            .map_err(|e| format!("Failed to find order: {e}"))?
            {
                if seen.insert(id) {
                    order_ids.push(id);
                }
            }
        }
        order_ids.sort_unstable();
        let first_order_id = match order_ids.first() {
            Some(&id) => id,
            None => {
                return Err("No orders found for consolidation".to_string());
            }
        };

        sqlx::query("UPDATE orders SET order_number = ? WHERE id = ?")
            .bind(&consolidation_info.new_order_number)
            .bind(first_order_id)
            .execute(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to update order number: {e}"))?;
        log::info!(
            "Consolidation: order id={} updated to new number {}",
            first_order_id,
            consolidation_info.new_order_number
        );

        let existing_link: Option<(i64,)> = sqlx::query_as(
            r#"SELECT order_id FROM order_emails WHERE order_id = ? AND email_id = ? LIMIT 1"#,
        )
        .bind(first_order_id)
        .bind(email_id)
        .fetch_optional(tx.as_mut())
        .await
        .map_err(|e| format!("Failed to check order_email link: {e}"))?;
        if existing_link.is_none() {
            sqlx::query("INSERT INTO order_emails (order_id, email_id) VALUES (?, ?)")
                .bind(first_order_id)
                .bind(email_id)
                .execute(tx.as_mut())
                .await
                .map_err(|e| format!("Failed to link order to email: {e}"))?;
        }

        for &order_id in &order_ids[1..] {
            sqlx::query("DELETE FROM items WHERE order_id = ?")
                .bind(order_id)
                .execute(tx.as_mut())
                .await
                .map_err(|e| {
                    format!("Failed to delete items for consolidated order {order_id}: {e}")
                })?;
            sqlx::query("DELETE FROM deliveries WHERE order_id = ?")
                .bind(order_id)
                .execute(tx.as_mut())
                .await
                .map_err(|e| {
                    format!("Failed to delete deliveries for consolidated order {order_id}: {e}")
                })?;
            log::info!(
                "Consolidation: cleared items/deliveries for superseded order id={}",
                order_id
            );
        }

        Ok(first_order_id)
    }

    /// apply_send_and_replace_items のトランザクション内ロジック（tx は呼び出し元で commit）
    pub(crate) async fn apply_send_and_replace_items_in_tx(
        tx: &mut sqlx::Transaction<'_, Sqlite>,
        order_info: &OrderInfo,
        email_id: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
        alternate_domains: Option<Vec<String>>,
    ) -> Result<i64, String> {
        let existing_order_id = Self::find_order_by_number_and_domain(
            tx,
            &order_info.order_number,
            &shop_domain,
            alternate_domains.as_deref(),
        )
        .await
        .map_err(|e| format!("Failed to find order: {e}"))?;

        let order_id = if let Some(id) = existing_order_id {
            log::debug!(
                "[dmm_send] found existing order id={} for number {} (shop_domain={:?}, alternate_domains={:?})",
                id,
                order_info.order_number,
                shop_domain,
                alternate_domains
            );
            if !order_info.items.is_empty() {
                Self::replace_items_for_order_in_tx(tx, id, order_info).await?;
                log::info!(
                    "[dmm_send] replaced items for existing order {} with {} items from send mail",
                    id,
                    order_info.items.len()
                );
            } else {
                log::info!(
                    "[dmm_send] skipping items replacement for order {} (items empty in send mail, keeping existing items)",
                    id
                );
            }
            id
        } else {
            log::warn!(
                "[dmm_send] existing order not found for {}, creating new order from send mail (shop_domain={:?}, alternate_domains={:?})",
                order_info.order_number,
                shop_domain,
                alternate_domains
            );
            let new_id =
                Self::save_order_in_tx(tx, order_info, email_id, shop_domain.clone(), shop_name)
                    .await?;
            log::info!(
                "[dmm_send] created new order {} from send mail (items={})",
                new_id,
                order_info.items.len()
            );
            // save_order_in_tx 内で deliveries / order_emails も処理済み
            return Ok(new_id);
        };

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
                log::debug!("Added new delivery info for order {} (send mail)", order_id);
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
                log::debug!("Updated delivery info for order {} (send mail)", order_id);
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
            .map_err(|e| format!("Failed to check order_email link: {e}"))?;

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
                log::debug!(
                    "Linked order {} to email {} (send mail)",
                    order_id,
                    email_id_val
                );
            }
        }

        Ok(order_id)
    }

    /// apply_split_first_order のトランザクション内ロジック（tx は呼び出し元で commit）
    pub(crate) async fn apply_split_first_order_in_tx(
        tx: &mut sqlx::Transaction<'_, Sqlite>,
        order_info: &OrderInfo,
        email_id: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
        alternate_domains: Option<Vec<String>>,
    ) -> Result<i64, String> {
        let order_id = match Self::find_order_by_number_and_domain(
            tx,
            &order_info.order_number,
            &shop_domain,
            alternate_domains.as_deref(),
        )
        .await
        .map_err(|e| format!("Failed to find order: {e}"))?
        {
            Some(existing_id) => {
                Self::replace_items_for_order_in_tx(tx, existing_id, order_info).await?;
                if order_info.order_date.is_some() {
                    sqlx::query(
                        r#"
                        UPDATE orders
                        SET order_date = COALESCE(?, order_date)
                        WHERE id = ?
                        "#,
                    )
                    .bind(&order_info.order_date)
                    .bind(existing_id)
                    .execute(tx.as_mut())
                    .await
                    .map_err(|e| format!("Failed to update order date: {e}"))?;
                }
                if let Some(email_id_val) = email_id {
                    let existing_link: Option<(i64,)> = sqlx::query_as(
                        r#"SELECT order_id FROM order_emails WHERE order_id = ? AND email_id = ? LIMIT 1"#,
                    )
                    .bind(existing_id)
                    .bind(email_id_val)
                    .fetch_optional(tx.as_mut())
                    .await
                    .map_err(|e| format!("Failed to check order_email link: {e}"))?;
                    if existing_link.is_none() {
                        sqlx::query(
                            r#"INSERT INTO order_emails (order_id, email_id) VALUES (?, ?)"#,
                        )
                        .bind(existing_id)
                        .bind(email_id_val)
                        .execute(tx.as_mut())
                        .await
                        .map_err(|e| format!("Failed to link order to email: {e}"))?;
                    }
                }
                log::info!(
                    "Split first order: updated existing order {} (order_number={})",
                    existing_id,
                    order_info.order_number
                );
                existing_id
            }
            None => {
                let id = Self::save_order_in_tx(tx, order_info, email_id, shop_domain, shop_name)
                    .await?;
                log::debug!(
                    "Split first order: created new order {} (order_number={})",
                    id,
                    order_info.order_number
                );
                id
            }
        };

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
        alternate_domains: Option<Vec<String>>,
    ) -> Result<i64, String> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to start transaction: {e}"))?;
        let result = Self::apply_cancel_in_tx(
            &mut tx,
            cancel_info,
            email_id,
            shop_domain,
            alternate_domains,
        )
        .await;
        match result {
            Ok(order_id) => {
                tx.commit()
                    .await
                    .map_err(|e| format!("Failed to commit transaction: {e}"))?;
                Ok(order_id)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e)
            }
        }
    }

    async fn apply_order_number_change(
        &self,
        change_info: &OrderNumberChangeInfo,
        email_id: i64,
        change_email_internal_date: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
        alternate_domains: Option<Vec<String>>,
    ) -> Result<i64, String> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to start transaction: {e}"))?;
        let result = Self::apply_order_number_change_in_tx(
            &mut tx,
            change_info,
            email_id,
            change_email_internal_date,
            shop_domain,
            shop_name,
            alternate_domains,
        )
        .await;
        match result {
            Ok(order_id) => {
                tx.commit()
                    .await
                    .map_err(|e| format!("Failed to commit transaction: {e}"))?;
                Ok(order_id)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e)
            }
        }
    }

    async fn apply_consolidation(
        &self,
        consolidation_info: &ConsolidationInfo,
        email_id: i64,
        shop_domain: Option<String>,
        _shop_name: Option<String>,
        alternate_domains: Option<Vec<String>>,
    ) -> Result<i64, String> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to start transaction: {e}"))?;
        let result = Self::apply_consolidation_in_tx(
            &mut tx,
            consolidation_info,
            email_id,
            shop_domain,
            alternate_domains,
        )
        .await;
        match result {
            Ok(order_id) => {
                tx.commit()
                    .await
                    .map_err(|e| format!("Failed to commit transaction: {e}"))?;
                Ok(order_id)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e)
            }
        }
    }

    async fn apply_send_and_replace_items(
        &self,
        order_info: &OrderInfo,
        email_id: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
        alternate_domains: Option<Vec<String>>,
    ) -> Result<i64, String> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to start transaction: {e}"))?;
        let result = Self::apply_send_and_replace_items_in_tx(
            &mut tx,
            order_info,
            email_id,
            shop_domain,
            shop_name,
            alternate_domains,
        )
        .await;
        match result {
            Ok(order_id) => {
                tx.commit()
                    .await
                    .map_err(|e| format!("Failed to commit transaction: {e}"))?;
                Ok(order_id)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e)
            }
        }
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
        let result = Self::apply_change_items_in_tx(
            &mut tx,
            order_info,
            shop_domain,
            change_email_internal_date,
        )
        .await;
        match result {
            Ok(()) => {
                tx.commit()
                    .await
                    .map_err(|e| format!("Failed to commit transaction: {e}"))?;
                Ok(())
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e)
            }
        }
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
        let result = async {
            Self::apply_change_items_in_tx(
                &mut tx,
                order_info,
                shop_domain.clone(),
                change_email_internal_date,
            )
            .await?;
            Self::save_order_in_tx(&mut tx, order_info, email_id, shop_domain, shop_name).await
        }
        .await;
        match result {
            Ok(order_id) => {
                tx.commit()
                    .await
                    .map_err(|e| format!("Failed to commit transaction: {e}"))?;
                Ok(order_id)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e)
            }
        }
    }

    async fn apply_split_first_order(
        &self,
        order_info: &OrderInfo,
        email_id: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
        alternate_domains: Option<Vec<String>>,
    ) -> Result<i64, String> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to start transaction: {e}"))?;
        let result = Self::apply_split_first_order_in_tx(
            &mut tx,
            order_info,
            email_id,
            shop_domain,
            shop_name,
            alternate_domains,
        )
        .await;
        match result {
            Ok(order_id) => {
                tx.commit()
                    .await
                    .map_err(|e| format!("Failed to commit transaction: {e}"))?;
                Ok(order_id)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::cancel_info::CancelInfo;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // テーブル作成（migrationsと同等の定義）

        // emails テーブル
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

        // orders テーブル
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

        // items テーブル
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

        // deliveries テーブル
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

        // order_emails テーブル
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

        // product_master テーブル（apply_change_items / apply_cancel の LEFT JOIN で使用）
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
                    image_url: None,
                },
                OrderItem {
                    name: "商品B".to_string(),
                    manufacturer: None,
                    model_number: None,
                    unit_price: 500,
                    quantity: 1,
                    subtotal: 500,
                    image_url: None,
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
                None,
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid cancel quantity"));
    }

    #[tokio::test]
    async fn test_apply_cancel_entire_order_no_product_name() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('KC-99999', 'mail.dmm.com', 'DMM')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = 'KC-99999'")
                .fetch_one(&pool)
                .await
                .expect("get order id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品A', 1)"#)
            .bind(order_id.0)
            .execute(&pool)
            .await
            .expect("insert item");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品B', 2)"#)
            .bind(order_id.0)
            .execute(&pool)
            .await
            .expect("insert item");
        sqlx::query("INSERT INTO emails (message_id, body_plain) VALUES ('cancel-email-7', '')")
            .execute(&pool)
            .await
            .expect("insert email");
        let email_id: (i64,) =
            sqlx::query_as("SELECT id FROM emails WHERE message_id = 'cancel-email-7'")
                .fetch_one(&pool)
                .await
                .expect("get email id");

        let cancel_info = CancelInfo {
            order_number: "KC-99999".to_string(),
            product_name: "".to_string(),
            cancel_quantity: 1,
        };
        let result = repo
            .apply_cancel(
                &cancel_info,
                email_id.0,
                Some("mail.dmm.com".to_string()),
                None,
                Some(vec!["mono.dmm.com".to_string()]), // DMM alternate domain
            )
            .await;
        assert!(result.is_ok(), "apply_cancel failed: {:?}", result.err());
        assert_eq!(result.unwrap(), order_id.0);

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(order_id.0)
            .fetch_one(&pool)
            .await
            .expect("count items");
        assert_eq!(count.0, 0, "all items should be removed");
    }

    #[tokio::test]
    async fn test_apply_order_number_change() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('KC-26407532', 'mail.dmm.com', 'DMM')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = 'KC-26407532'")
                .fetch_one(&pool)
                .await
                .expect("get order id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品A', 1)"#)
            .bind(order_id.0)
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

        let change_info = crate::parsers::order_number_change_info::OrderNumberChangeInfo {
            old_order_number: "KC-26407532".to_string(),
            new_order_number: "BS-26888944".to_string(),
        };
        let result = repo
            .apply_order_number_change(
                &change_info,
                email_id.0,
                None,
                Some("mail.dmm.com".to_string()),
                None,
                Some(vec!["mono.dmm.com".to_string()]), // DMM alternate domain
            )
            .await;
        assert!(
            result.is_ok(),
            "apply_order_number_change failed: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), order_id.0);

        let row: (String,) = sqlx::query_as("SELECT order_number FROM orders WHERE id = ?")
            .bind(order_id.0)
            .fetch_one(&pool)
            .await
            .expect("get order");
        assert_eq!(row.0, "BS-26888944");
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
                image_url: None,
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

        // 残り商品 0 で deliveries がクリーンアップされること（order/order_emails は再パース防止のため保持）
        let order_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE id = ?")
            .bind(old_order_id.0)
            .fetch_one(&pool)
            .await
            .expect("check order");
        assert_eq!(
            order_exists.0, 1,
            "empty order is retained (deliveries cleaned only)"
        );
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
                image_url: None,
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
                image_url: None,
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
                image_url: None,
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
                image_url: None,
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

        // 両方の元注文は保持されること（残り商品0、deliveries クリーンアップ。再パース防止のため order は削除しない）
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
        assert_eq!(order1_exists.0, 1, "empty order 1 is retained");
        assert_eq!(order2_exists.0, 1, "empty order 2 is retained");
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
                image_url: None,
            }],
            subtotal: Some(2000),
            shipping_fee: None,
            total_amount: Some(2000),
        };

        let result = repo
            .apply_change_items(&order_info, Some("1999.co.jp".to_string()), None)
            .await;
        assert!(result.is_ok());

        // 同一注文内の2行とも削除され、注文は保持されること（deliveries クリーンアップのみ）
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
        assert_eq!(order_exists.0, 1, "empty order is retained");
    }

    #[tokio::test]
    async fn test_apply_change_items_respects_change_email_internal_date() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // cutoff = 2024-06-01 00:00:00 UTC (1717200000000 ms)
        let cutoff_ts = 1717200000000i64;

        // 注文1: order_date が cutoff より前 → 対象になる
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name, order_date, created_at) VALUES ('99-7100-0001', '1999.co.jp', 'ホビーサーチ', '2024-01-01 00:00:00', '2024-01-01 00:00:00')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order 1");
        let order1_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-7100-0001'")
                .fetch_one(&pool)
                .await
                .expect("get order 1 id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品D', 1)"#)
            .bind(order1_id.0)
            .execute(&pool)
            .await
            .expect("insert item");

        // 注文2: order_date が cutoff より後 → 対象外
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name, order_date, created_at) VALUES ('99-7100-0002', '1999.co.jp', 'ホビーサーチ', '2024-12-01 00:00:00', '2024-12-01 00:00:00')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order 2");
        let order2_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-7100-0002'")
                .fetch_one(&pool)
                .await
                .expect("get order 2 id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品D', 1)"#)
            .bind(order2_id.0)
            .execute(&pool)
            .await
            .expect("insert item");

        // 注文3: order_date が NULL、created_at が cutoff より前 → COALESCE で created_at を使用、対象になる
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name, order_date, created_at) VALUES ('99-7100-0003', '1999.co.jp', 'ホビーサーチ', NULL, '2024-01-15 00:00:00')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order 3");
        let order3_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-7100-0003'")
                .fetch_one(&pool)
                .await
                .expect("get order 3 id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品D', 1)"#)
            .bind(order3_id.0)
            .execute(&pool)
            .await
            .expect("insert item");

        // 組み換え後は商品D が2個（注文1と3から。注文2は対象外）
        let order_info = crate::parsers::OrderInfo {
            order_number: "25-0918-1710".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![crate::parsers::OrderItem {
                name: "商品D".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 800,
                quantity: 2,
                subtotal: 1600,
                image_url: None,
            }],
            subtotal: Some(1600),
            shipping_fee: None,
            total_amount: Some(1600),
        };

        let result = repo
            .apply_change_items(&order_info, Some("1999.co.jp".to_string()), Some(cutoff_ts))
            .await;
        assert!(result.is_ok());

        // 注文1: 商品削除 → 注文は保持（deliveries クリーンアップのみ）
        let order1_items: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(order1_id.0)
            .fetch_one(&pool)
            .await
            .expect("count order 1 items");
        assert_eq!(
            order1_items.0, 0,
            "order 1 (before cutoff) items should be removed"
        );
        let order1_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE id = ?")
            .bind(order1_id.0)
            .fetch_one(&pool)
            .await
            .expect("check order 1");
        assert_eq!(order1_exists.0, 1, "order 1 is retained");

        // 注文2: cutoff より後なので対象外 → 商品が残る
        let order2_items: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(order2_id.0)
            .fetch_one(&pool)
            .await
            .expect("count order 2 items");
        assert_eq!(
            order2_items.0, 1,
            "order 2 (after cutoff) should keep its item"
        );

        // 注文3: order_date NULL だが created_at < cutoff なので対象 → 商品削除、注文は保持
        let order3_items: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(order3_id.0)
            .fetch_one(&pool)
            .await
            .expect("count order 3 items");
        assert_eq!(order3_items.0, 0, "order 3 items should be removed");
        let order3_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE id = ?")
            .bind(order3_id.0)
            .fetch_one(&pool)
            .await
            .expect("check order 3");
        assert_eq!(order3_exists.0, 1, "order 3 is retained");
    }

    #[tokio::test]
    // order_date は JST 日付文字列（例: '2022-06-05'）で保存されるが、cutoff は UTC タイムスタンプ。
    // UTC に変換すると '2022-06-04 17:59:04' となり、'2022-06-05' > '2022-06-04...' で対象外になるバグの再現・回帰テスト。
    // '+9 hours' で JST 変換することで '2022-06-05 02:59:04' となり、'2022-06-05' < '2022-06-05 02:59:04' = TRUE になる。
    async fn test_apply_change_items_jst_order_date_included_in_utc_cutoff() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // cutoff = email 186 の internal_date: 2022-06-04 17:59:04 UTC = 2022-06-05 02:59:04 JST
        let cutoff_ts = 1654365544000i64;

        // 注文: order_date = '2022-06-05'（JST 日付）で、おまとめメール受信時刻より前（JST では同日だが早い時刻）
        // → JST 変換後の cutoff '2022-06-05 02:59:04' より前なので対象になるべき
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name, order_date, created_at) VALUES ('00006', 'p-bandai.jp', 'プレミアムバンダイ', '2022-06-05', '2022-06-04 15:00:00')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let order_id: (i64,) = sqlx::query_as("SELECT id FROM orders WHERE order_number = '00006'")
            .fetch_one(&pool)
            .await
            .expect("get order id");
        sqlx::query(
            r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, 'Figure-rise Standard Amplified メタルガルルモン（ブラックＶｅｒ．）', 1)"#,
        )
        .bind(order_id.0)
        .execute(&pool)
        .await
        .expect("insert item");

        let order_info = crate::parsers::OrderInfo {
            order_number: "00007".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![crate::parsers::OrderItem {
                name: "Figure-rise Standard Amplified メタルガルルモン（ブラックＶｅｒ．）"
                    .to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 4950,
                quantity: 1,
                subtotal: 4950,
                image_url: None,
            }],
            subtotal: Some(4950),
            shipping_fee: None,
            total_amount: Some(4950),
        };

        let result = repo
            .apply_change_items(
                &order_info,
                Some("p-bandai.jp".to_string()),
                Some(cutoff_ts),
            )
            .await;
        assert!(result.is_ok(), "apply_change_items failed: {:?}", result);

        // JST 変換なしでは order_date '2022-06-05' > UTC '2022-06-04 17:59:04' となり対象外になる（バグ）
        // JST 変換後は '2022-06-05' < '2022-06-05 02:59:04' となり対象になる（修正後）
        let item_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(order_id.0)
            .fetch_one(&pool)
            .await
            .expect("count items");
        assert_eq!(
            item_count.0, 0,
            "JST 変換により order_date '2022-06-05' が UTC cutoff '2022-06-04 17:59:04' より前と判定され、商品が削除されるべき"
        );
    }

    #[tokio::test]
    // created_at は UTC で保存されるため、+9 hours を加算せずに UTC cutoff と比較する必要がある。
    // created_at が UTC cutoff より後の注文は、旧コード（COALESCE + '+9 hours'）では誤って対象に含まれてしまう
    // （'2024-06-01 01:00:00' < '2024-06-01 09:00:00' → TRUE）ため、正しく除外されることを確認する。
    async fn test_apply_change_items_created_at_utc_not_shifted() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // cutoff = 2024-06-01 00:00:00 UTC (1717200000000 ms)
        // JST では 2024-06-01 09:00:00
        let cutoff_ts = 1717200000000i64;

        // 注文: order_date = NULL, created_at = cutoff UTC + 1 hour (= '2024-06-01 01:00:00 UTC')
        // UTC cutoff より後 → 対象外になるべき
        // 旧コード（COALESCE + '+9 hours'）では '2024-06-01 01:00:00' < '2024-06-01 09:00:00' → TRUE で誤包含
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name, order_date, created_at) VALUES ('COALESCE-001', '1999.co.jp', 'ホビーサーチ', NULL, '2024-06-01 01:00:00')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = 'COALESCE-001'")
                .fetch_one(&pool)
                .await
                .expect("get order id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品E', 1)"#)
            .bind(order_id.0)
            .execute(&pool)
            .await
            .expect("insert item");

        let order_info = crate::parsers::OrderInfo {
            order_number: "COALESCE-002".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![crate::parsers::OrderItem {
                name: "商品E".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 1000,
                quantity: 1,
                subtotal: 1000,
                image_url: None,
            }],
            subtotal: Some(1000),
            shipping_fee: None,
            total_amount: Some(1000),
        };

        let result = repo
            .apply_change_items(
                &order_info,
                Some("1999.co.jp".to_string()),
                Some(cutoff_ts),
            )
            .await;
        assert!(result.is_ok(), "apply_change_items failed: {:?}", result);

        // created_at '2024-06-01 01:00:00' UTC は cutoff '2024-06-01 00:00:00' UTC より後なので対象外
        // 商品は削除されず残るべき
        let item_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(order_id.0)
            .fetch_one(&pool)
            .await
            .expect("count items");
        assert_eq!(
            item_count.0, 1,
            "created_at が UTC cutoff より後の注文は対象外のため、既存商品は削除されずに残るべき"
        );
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
                image_url: None,
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

        // 元注文から商品が削除され、注文は保持されること（deliveries クリーンアップのみ）
        let old_order_items: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
                .bind(old_order_id.0)
                .fetch_one(&pool)
                .await
                .expect("count old order items");
        assert_eq!(old_order_items.0, 0, "old order items should be removed");
        let old_order_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE id = ?")
            .bind(old_order_id.0)
            .fetch_one(&pool)
            .await
            .expect("check old order");
        assert_eq!(old_order_exists.0, 1, "old order is retained");

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
        assert_eq!(
            link_count.0, 1,
            "order_emails should link new order to email"
        );
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
                image_url: None,
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
        assert_eq!(
            item_count.0, 1,
            "old order items should remain after rollback"
        );
    }

    // --- apply_split_first_order 統合テスト ---

    #[tokio::test]
    async fn test_apply_split_first_order_existing_order() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // 既存注文を作成
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('KC-12345', 'mail.dmm.com', 'DMM通販')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = 'KC-12345'")
                .fetch_one(&pool)
                .await
                .expect("get order id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity, price) VALUES (?, '旧商品A', 2, 1000)"#)
            .bind(order_id.0)
            .execute(&pool)
            .await
            .expect("insert item");

        // メールを作成
        sqlx::query("INSERT INTO emails (message_id, body_plain) VALUES ('split-email-1', '')")
            .execute(&pool)
            .await
            .expect("insert email");
        let email_id: (i64,) =
            sqlx::query_as("SELECT id FROM emails WHERE message_id = 'split-email-1'")
                .fetch_one(&pool)
                .await
                .expect("get email id");

        // 分割完了の先頭注文（既存注文の items を差し替え）
        let order_info = crate::parsers::OrderInfo {
            order_number: "KC-12345".to_string(),
            order_date: Some("2024-06-01 10:00:00".to_string()),
            delivery_address: None,
            delivery_info: None,
            items: vec![crate::parsers::OrderItem {
                name: "新商品A".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 500,
                quantity: 1,
                subtotal: 500,
                image_url: None,
            }],
            subtotal: Some(500),
            shipping_fee: None,
            total_amount: Some(500),
        };

        let result = repo
            .apply_split_first_order(
                &order_info,
                Some(email_id.0),
                Some("mail.dmm.com".to_string()),
                Some("DMM通販".to_string()),
                Some(vec!["mono.dmm.com".to_string()]),
            )
            .await;
        assert!(result.is_ok());
        let result_id = result.unwrap();
        assert_eq!(result_id, order_id.0, "should update existing order");

        // items が差し替わっていること
        let items: Vec<(String, i64)> =
            sqlx::query_as("SELECT item_name, quantity FROM items WHERE order_id = ?")
                .bind(order_id.0)
                .fetch_all(&pool)
                .await
                .expect("fetch items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, "新商品A");
        assert_eq!(items[0].1, 1);

        // order_emails に紐づいていること
        let link_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM order_emails WHERE order_id = ? AND email_id = ?")
                .bind(order_id.0)
                .bind(email_id.0)
                .fetch_one(&pool)
                .await
                .expect("count order_emails");
        assert_eq!(link_count.0, 1);
    }

    #[tokio::test]
    async fn test_apply_split_first_order_no_existing() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // メールを作成
        sqlx::query("INSERT INTO emails (message_id, body_plain) VALUES ('split-email-2', '')")
            .execute(&pool)
            .await
            .expect("insert email");
        let email_id: (i64,) =
            sqlx::query_as("SELECT id FROM emails WHERE message_id = 'split-email-2'")
                .fetch_one(&pool)
                .await
                .expect("get email id");

        // 既存注文がない場合は新規作成
        let order_info = crate::parsers::OrderInfo {
            order_number: "KC-99999".to_string(),
            order_date: Some("2024-06-01 10:00:00".to_string()),
            delivery_address: None,
            delivery_info: None,
            items: vec![crate::parsers::OrderItem {
                name: "新商品B".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 800,
                quantity: 1,
                subtotal: 800,
                image_url: None,
            }],
            subtotal: Some(800),
            shipping_fee: None,
            total_amount: Some(800),
        };

        let result = repo
            .apply_split_first_order(
                &order_info,
                Some(email_id.0),
                Some("mail.dmm.com".to_string()),
                Some("DMM通販".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());
        let new_id = result.unwrap();

        // 新規注文が作成されていること
        let order: (String,) = sqlx::query_as("SELECT order_number FROM orders WHERE id = ?")
            .bind(new_id)
            .fetch_one(&pool)
            .await
            .expect("get order");
        assert_eq!(order.0, "KC-99999");

        // items が保存されていること
        let items: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(new_id)
            .fetch_one(&pool)
            .await
            .expect("count items");
        assert_eq!(items.0, 1);
    }

    // --- apply_consolidation 統合テスト ---

    #[tokio::test]
    async fn test_apply_consolidation_merges_orders() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // 2つの注文を作成
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('KC-00001', 'mail.dmm.com', 'DMM通販')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order 1");
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('KC-00002', 'mail.dmm.com', 'DMM通販')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order 2");

        let order1_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = 'KC-00001'")
                .fetch_one(&pool)
                .await
                .expect("get order 1 id");
        let order2_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = 'KC-00002'")
                .fetch_one(&pool)
                .await
                .expect("get order 2 id");

        // 各注文に商品を追加
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品1', 1)"#)
            .bind(order1_id.0)
            .execute(&pool)
            .await
            .expect("insert item 1");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity) VALUES (?, '商品2', 1)"#)
            .bind(order2_id.0)
            .execute(&pool)
            .await
            .expect("insert item 2");

        // メールを作成
        sqlx::query("INSERT INTO emails (message_id, body_plain) VALUES ('merge-email-1', '')")
            .execute(&pool)
            .await
            .expect("insert email");
        let email_id: (i64,) =
            sqlx::query_as("SELECT id FROM emails WHERE message_id = 'merge-email-1'")
                .fetch_one(&pool)
                .await
                .expect("get email id");

        let consolidation_info = crate::parsers::consolidation_info::ConsolidationInfo {
            old_order_numbers: vec!["KC-00001".to_string(), "KC-00002".to_string()],
            new_order_number: "KC-NEW-001".to_string(),
        };

        let result = repo
            .apply_consolidation(
                &consolidation_info,
                email_id.0,
                Some("mail.dmm.com".to_string()),
                Some("DMM通販".to_string()),
                Some(vec!["mono.dmm.com".to_string()]),
            )
            .await;
        assert!(result.is_ok());
        let result_id = result.unwrap();
        assert_eq!(result_id, order1_id.0, "should return first order id");

        // 先頭注文の番号が新番号に更新されていること
        let order1_number: (String,) =
            sqlx::query_as("SELECT order_number FROM orders WHERE id = ?")
                .bind(order1_id.0)
                .fetch_one(&pool)
                .await
                .expect("get order 1 number");
        assert_eq!(order1_number.0, "KC-NEW-001");

        // 先頭注文の items は残っていること
        let order1_items: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(order1_id.0)
            .fetch_one(&pool)
            .await
            .expect("count order 1 items");
        assert_eq!(order1_items.0, 1, "first order items should remain");

        // 2番目の注文の items は削除されていること
        let order2_items: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(order2_id.0)
            .fetch_one(&pool)
            .await
            .expect("count order 2 items");
        assert_eq!(order2_items.0, 0, "second order items should be cleared");

        // order_emails に紐づいていること
        let link_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM order_emails WHERE order_id = ? AND email_id = ?")
                .bind(order1_id.0)
                .bind(email_id.0)
                .fetch_one(&pool)
                .await
                .expect("count order_emails");
        assert_eq!(link_count.0, 1);
    }

    // --- apply_send_and_replace_items 統合テスト ---

    #[tokio::test]
    async fn test_apply_send_and_replace_items_existing_order() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // 既存注文を作成
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain, shop_name) VALUES ('BS-11111', 'mail.dmm.com', 'DMM通販')"#,
        )
        .execute(&pool)
        .await
        .expect("insert order");
        let order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = 'BS-11111'")
                .fetch_one(&pool)
                .await
                .expect("get order id");
        sqlx::query(r#"INSERT INTO items (order_id, item_name, quantity, price) VALUES (?, '旧商品X', 2, 1000)"#)
            .bind(order_id.0)
            .execute(&pool)
            .await
            .expect("insert item");

        // メールを作成
        sqlx::query("INSERT INTO emails (message_id, body_plain) VALUES ('send-email-1', '')")
            .execute(&pool)
            .await
            .expect("insert email");
        let email_id: (i64,) =
            sqlx::query_as("SELECT id FROM emails WHERE message_id = 'send-email-1'")
                .fetch_one(&pool)
                .await
                .expect("get email id");

        let order_info = crate::parsers::OrderInfo {
            order_number: "BS-11111".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: Some(crate::parsers::DeliveryInfo {
                carrier: "佐川急便".to_string(),
                tracking_number: "364631890991".to_string(),
                delivery_date: None,
                delivery_time: None,
                carrier_url: None,
            }),
            items: vec![crate::parsers::OrderItem {
                name: "発送商品X".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 800,
                quantity: 1,
                subtotal: 800,
                image_url: None,
            }],
            subtotal: Some(800),
            shipping_fee: None,
            total_amount: Some(800),
        };

        let result = repo
            .apply_send_and_replace_items(
                &order_info,
                Some(email_id.0),
                Some("mail.dmm.com".to_string()),
                Some("DMM通販".to_string()),
                Some(vec!["mono.dmm.com".to_string()]),
            )
            .await;
        assert!(result.is_ok());
        let result_id = result.unwrap();
        assert_eq!(result_id, order_id.0);

        // items が差し替わっていること
        let items: Vec<(String, i64)> =
            sqlx::query_as("SELECT item_name, quantity FROM items WHERE order_id = ?")
                .bind(order_id.0)
                .fetch_all(&pool)
                .await
                .expect("fetch items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, "発送商品X");

        // deliveries が作成されていること
        let deliveries: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT tracking_number, carrier, delivery_status FROM deliveries WHERE order_id = ?",
        )
        .bind(order_id.0)
        .fetch_all(&pool)
        .await
        .expect("fetch deliveries");
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].0, "364631890991");
        assert_eq!(deliveries[0].1, "佐川急便");
        assert_eq!(deliveries[0].2, "shipped");

        // order_emails に紐づいていること
        let link_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM order_emails WHERE order_id = ? AND email_id = ?")
                .bind(order_id.0)
                .bind(email_id.0)
                .fetch_one(&pool)
                .await
                .expect("count order_emails");
        assert_eq!(link_count.0, 1);
    }

    #[tokio::test]
    async fn test_apply_send_and_replace_items_no_existing() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // メールを作成
        sqlx::query("INSERT INTO emails (message_id, body_plain) VALUES ('send-email-2', '')")
            .execute(&pool)
            .await
            .expect("insert email");
        let email_id: (i64,) =
            sqlx::query_as("SELECT id FROM emails WHERE message_id = 'send-email-2'")
                .fetch_one(&pool)
                .await
                .expect("get email id");

        let order_info = crate::parsers::OrderInfo {
            order_number: "BS-99999".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: Some(crate::parsers::DeliveryInfo {
                carrier: "ヤマト運輸".to_string(),
                tracking_number: "111222333444".to_string(),
                delivery_date: None,
                delivery_time: None,
                carrier_url: None,
            }),
            items: vec![crate::parsers::OrderItem {
                name: "新規商品Y".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 1200,
                quantity: 1,
                subtotal: 1200,
                image_url: None,
            }],
            subtotal: Some(1200),
            shipping_fee: None,
            total_amount: Some(1200),
        };

        let result = repo
            .apply_send_and_replace_items(
                &order_info,
                Some(email_id.0),
                Some("mail.dmm.com".to_string()),
                Some("DMM通販".to_string()),
                None,
            )
            .await;
        assert!(result.is_ok());
        let new_id = result.unwrap();

        // 新規注文が作成されていること
        let order: (String,) = sqlx::query_as("SELECT order_number FROM orders WHERE id = ?")
            .bind(new_id)
            .fetch_one(&pool)
            .await
            .expect("get order");
        assert_eq!(order.0, "BS-99999");

        // items が保存されていること
        let items: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(new_id)
            .fetch_one(&pool)
            .await
            .expect("count items");
        assert_eq!(items.0, 1);

        // deliveries が作成されていること
        let deliveries: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM deliveries WHERE order_id = ?")
                .bind(new_id)
                .fetch_one(&pool)
                .await
                .expect("count deliveries");
        assert_eq!(deliveries.0, 1);
    }

    // --- item_names_match 単体テスト ---

    #[test]
    fn test_item_names_match_exact() {
        assert!(item_names_match("商品A", None, "商品A", None, None,));
    }

    #[test]
    fn test_item_names_match_via_product_master() {
        // 商品コード差がある場合でも product_master.product_name が一致すればマッチすること
        // "アニュラス 267064 新条アカネ(ニューオーダー)" vs "アニュラス 67064 新条アカネ(ニューオーダー)"
        assert!(item_names_match(
            "アニュラス 267064 新条アカネ(ニューオーダー)",
            Some("新条アカネ(ニューオーダー)"),
            "アニュラス 67064 新条アカネ(ニューオーダー)",
            Some("アニュラス67064新条アカネニューオーダー"),
            Some("新条アカネ(ニューオーダー)"),
        ));
    }

    #[test]
    fn test_item_names_match_no_match_without_product_master() {
        // product_master なしでは商品コード差がある場合にマッチしないこと
        assert!(!item_names_match(
            "アニュラス 267064 新条アカネ(ニューオーダー)",
            None,
            "アニュラス 67064 新条アカネ(ニューオーダー)",
            Some("アニュラス67064新条アカネニューオーダー"),
            None,
        ));
    }

    #[test]
    fn test_item_names_match_product_master_empty_strings_do_not_match() {
        // product_master_name が空文字の場合はマッチしないこと（誤マッチ防止）
        assert!(!item_names_match(
            "アニュラス 267064 新条アカネ(ニューオーダー)",
            Some(""),
            "アニュラス 67064 新条アカネ(ニューオーダー)",
            None,
            Some(""),
        ));
    }

    #[test]
    fn test_item_names_match_different_product_master_names_do_not_match() {
        // product_master_name が異なる場合はマッチしないこと
        assert!(!item_names_match(
            "商品X コード111",
            Some("商品X"),
            "商品Y コード222",
            None,
            Some("商品Y"),
        ));
    }

    // --- apply_change_items + product_master 統合テスト ---

    #[tokio::test]
    async fn test_apply_change_items_via_product_master_match() {
        // product_master に両方の商品名が同一 product_name で登録されている場合、
        // 商品コード差があっても組み換えが成功すること
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // 元注文に旧商品コード付き商品名を登録
        sqlx::query(
            r#"INSERT INTO orders (order_number, shop_domain) VALUES ('99-OLD-001', 'annulus.co.jp')"#,
        )
        .execute(&pool)
        .await
        .expect("insert old order");
        let old_order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = '99-OLD-001'")
                .fetch_one(&pool)
                .await
                .expect("get old order id");
        sqlx::query(
            r#"INSERT INTO items (order_id, item_name, item_name_normalized, quantity)
               VALUES (?, 'アニュラス 67064 新条アカネ(ニューオーダー)', 'アニュラス67064新条アカネニューオーダー', 1)"#,
        )
        .bind(old_order_id.0)
        .execute(&pool)
        .await
        .expect("insert old item");

        // product_master に両方の商品名を同一 product_name で登録
        sqlx::query(
            r#"INSERT INTO product_master (raw_name, normalized_name, product_name)
               VALUES ('アニュラス 67064 新条アカネ(ニューオーダー)', 'アニュラス67064新条アカネニューオーダー', '新条アカネ(ニューオーダー)')"#,
        )
        .execute(&pool)
        .await
        .expect("insert product_master old");
        sqlx::query(
            r#"INSERT INTO product_master (raw_name, normalized_name, product_name)
               VALUES ('アニュラス 267064 新条アカネ(ニューオーダー)', 'アニュラス267064新条アカネニューオーダー', '新条アカネ(ニューオーダー)')"#,
        )
        .execute(&pool)
        .await
        .expect("insert product_master new");

        // 新商品コード付き商品名で組み換えメールを受信
        let order_info = crate::parsers::OrderInfo {
            order_number: "25-NEW-9999".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![crate::parsers::OrderItem {
                name: "アニュラス 267064 新条アカネ(ニューオーダー)".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 5049,
                quantity: 1,
                subtotal: 5049,
                image_url: None,
            }],
            subtotal: Some(5049),
            shipping_fee: None,
            total_amount: Some(5049),
        };

        let result = repo
            .apply_change_items(&order_info, Some("annulus.co.jp".to_string()), None)
            .await;
        assert!(
            result.is_ok(),
            "apply_change_items failed: {:?}",
            result.err()
        );

        // 元注文から商品が削除されていること
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(old_order_id.0)
            .fetch_one(&pool)
            .await
            .expect("count items");
        assert_eq!(
            count.0, 0,
            "item should be removed from old order via product_master match"
        );
    }
}
