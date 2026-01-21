use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

pub mod hobbysearch;

/// 注文情報を表す構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderInfo {
    /// 注文番号
    pub order_number: String,
    /// 注文日
    pub order_date: Option<String>,
    /// 配送先情報
    pub delivery_address: Option<DeliveryAddress>,
    /// 配送情報
    pub delivery_info: Option<DeliveryInfo>,
    /// 商品リスト
    pub items: Vec<OrderItem>,
    /// 小計
    pub subtotal: Option<i64>,
    /// 送料
    pub shipping_fee: Option<i64>,
    /// 合計金額
    pub total_amount: Option<i64>,
}

/// 配送先情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryAddress {
    /// 宛名
    pub name: String,
    /// 郵便番号
    pub postal_code: Option<String>,
    /// 住所
    pub address: Option<String>,
}

/// 配送情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryInfo {
    /// 配送会社
    pub carrier: String,
    /// 配送伝票番号
    pub tracking_number: String,
    /// 配送日
    pub delivery_date: Option<String>,
    /// 配送時間
    pub delivery_time: Option<String>,
    /// 配送会社URL
    pub carrier_url: Option<String>,
}

/// 商品情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    /// 商品名
    pub name: String,
    /// メーカー・ブランド
    pub manufacturer: Option<String>,
    /// 型番・品番
    pub model_number: Option<String>,
    /// 単価
    pub unit_price: i64,
    /// 個数
    pub quantity: i64,
    /// 小計
    pub subtotal: i64,
}

/// メールパーサーのトレイト
pub trait EmailParser {
    /// メール本文から注文情報をパースする
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String>;
}

/// パーサータイプから適切なパーサーを取得する
pub fn get_parser(parser_type: &str) -> Option<Box<dyn EmailParser>> {
    match parser_type {
        "hobbysearch" => Some(Box::new(hobbysearch::HobbySearchParser)),
        _ => None,
    }
}

/// パース結果をデータベースに保存する
pub async fn save_order_to_db(
    pool: &SqlitePool,
    order_info: &OrderInfo,
    email_id: Option<i64>,
    shop_domain: Option<&str>,
) -> Result<i64, String> {
    // トランザクション開始
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    // 1. ordersテーブルに注文を保存
    let order_id = sqlx::query(
        r#"
        INSERT INTO orders (order_number, order_date, shop_domain)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(&order_info.order_number)
    .bind(&order_info.order_date)
    .bind(shop_domain)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to insert order: {}", e))?
    .last_insert_rowid();

    // 2. itemsテーブルに商品を保存
    for item in &order_info.items {
        sqlx::query(
            r#"
            INSERT INTO items (order_id, item_name, brand, price, quantity)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(order_id)
        .bind(&item.name)
        .bind(&item.manufacturer)
        .bind(item.unit_price)
        .bind(item.quantity)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert item: {}", e))?;
    }

    // 3. deliveriesテーブルに配送情報を保存
    if let Some(delivery_info) = &order_info.delivery_info {
        sqlx::query(
            r#"
            INSERT INTO deliveries (order_id, tracking_number, carrier, delivery_status)
            VALUES (?, ?, ?, 'shipped')
            "#,
        )
        .bind(order_id)
        .bind(&delivery_info.tracking_number)
        .bind(&delivery_info.carrier)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert delivery: {}", e))?;
    }

    // 4. order_emailsテーブルにメールとの関連を保存
    if let Some(email_id) = email_id {
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
        .map_err(|e| format!("Failed to link order to email: {}", e))?;
    }

    // トランザクションをコミット
    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    Ok(order_id)
}
