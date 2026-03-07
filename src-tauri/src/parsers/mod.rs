use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::{Arc, Mutex};

// 定数はemail_parse_taskモジュールからエクスポート
pub use email_parse_task::{EMAIL_PARSE_EVENT_NAME, EMAIL_PARSE_TASK_NAME};

/// パース対象メールの情報（get_unparsed_emails の戻り値）
#[derive(Debug, Clone, FromRow)]
pub struct EmailRow {
    #[sqlx(rename = "id")]
    pub email_id: i64,
    pub message_id: String,
    pub body_plain: Option<String>,
    pub body_html: Option<String>,
    pub from_address: Option<String>,
    pub subject: Option<String>,
    pub internal_date: Option<i64>,
}

/// body_html があれば使用、なければ body_plain を返す（タグ除去は行わない）。
/// DMM 等は HTML から直接パースするため、HTML 優先で精度が上がる。
pub fn get_body_for_parse(row: &EmailRow) -> String {
    let html = row.body_html.as_deref().unwrap_or("").trim();
    if !html.is_empty() {
        return html.to_string();
    }
    row.body_plain.as_deref().unwrap_or("").to_string()
}

// キャンセル情報（全店舗共通）
pub mod cancel_info;
// 注文番号変更情報（全店舗共通）
pub mod order_number_change_info;
// まとめ完了情報（全店舗共通）
pub mod consolidation_info;

// BatchTask 実装
pub mod email_parse_task;
pub use email_parse_task::{
    EmailParseContext, EmailParseInput, EmailParseOutput, EmailParseTask, ShopSettingsCache,
};

/// パース状態管理
///
/// 進捗テーブル削除後はメモリのみで状態を管理する。
/// last_error はエラー時に設定され、次回 start でクリアされる。
#[derive(Clone)]
pub struct ParseState {
    pub should_cancel: Arc<Mutex<bool>>,
    pub is_running: Arc<Mutex<bool>>,
    /// 直近のエラーメッセージ（エラー時のみ。start でクリア）
    pub last_error: Arc<Mutex<Option<String>>>,
}

impl Default for ParseState {
    fn default() -> Self {
        Self {
            should_cancel: Arc::new(Mutex::new(false)),
            is_running: Arc::new(Mutex::new(false)),
            last_error: Arc::new(Mutex::new(None)),
        }
    }
}

impl ParseState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_cancel(&self) {
        if let Ok(mut cancel) = self.should_cancel.lock() {
            *cancel = true;
            log::info!("Parse cancellation requested");
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.should_cancel.lock().map(|c| *c).unwrap_or(false)
    }

    /// エラーを記録（get_parse_status で error として返す）
    pub fn set_error(&self, msg: &str) {
        if let Ok(mut err) = self.last_error.lock() {
            *err = Some(msg.to_string());
        }
    }

    /// エラーをクリア
    pub fn clear_error(&self) {
        if let Ok(mut err) = self.last_error.lock() {
            *err = None;
        }
    }

    /// 強制的に idle にリセット
    pub fn force_idle(&self) {
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }
        self.clear_error();
    }

    pub fn start(&self) -> Result<(), String> {
        let mut running = self.is_running.lock().map_err(|e| e.to_string())?;
        if *running {
            return Err("Parse is already running".to_string());
        }
        *running = true;
        *self.should_cancel.lock().map_err(|e| e.to_string())? = false;
        self.clear_error();
        Ok(())
    }

    pub fn finish(&self) {
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }
        if let Ok(mut cancel) = self.should_cancel.lock() {
            *cancel = false;
        }
    }
}

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
    /// deliveries テーブルに登録する初期ステータス（None の場合は "shipped"）
    pub delivery_status: Option<String>,
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
    /// 商品画像URL（注文確認メールに含まれる場合、images テーブルへ登録する）
    pub image_url: Option<String>,
}

/// メールパーサーのトレイト
pub trait EmailParser {
    /// メール本文から注文情報をパースする
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String>;

    /// 1通のメールから複数注文を返す場合に実装する。
    /// デフォルトは None（単一注文として parse() を使用）。
    /// Some(Ok(orders)) を返すとバッチで各注文を save_order する。
    fn parse_multi(&self, _email_body: &str) -> Option<Result<Vec<OrderInfo>, String>> {
        None
    }
}

/// パースメタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseMetadata {
    pub parse_status: String,
    pub last_parse_started_at: Option<String>,
    pub last_parse_completed_at: Option<String>,
    pub total_parsed_count: i64,
    pub last_error_message: Option<String>,
    pub batch_size: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ParseState Tests ====================

    #[test]
    fn test_parse_state_new() {
        let state = ParseState::new();
        assert!(!state.is_cancelled());
        assert!(state.should_cancel.lock().unwrap().eq(&false));
        assert!(state.is_running.lock().unwrap().eq(&false));
    }

    #[test]
    fn test_parse_state_request_cancel() {
        let state = ParseState::new();
        assert!(!state.is_cancelled());

        state.request_cancel();
        assert!(state.is_cancelled());
    }

    #[test]
    fn test_parse_state_start_success() {
        let state = ParseState::new();
        let result = state.start();
        assert!(result.is_ok());
        assert!(*state.is_running.lock().unwrap());
    }

    #[test]
    fn test_parse_state_start_already_running() {
        let state = ParseState::new();

        // 最初のstart
        let result = state.start();
        assert!(result.is_ok());

        // 2回目のstartはエラー
        let result = state.start();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Parse is already running");
    }

    #[test]
    fn test_parse_state_start_resets_cancel_flag() {
        let state = ParseState::new();
        state.request_cancel();
        assert!(state.is_cancelled());

        let result = state.start();
        assert!(result.is_ok());
        assert!(!state.is_cancelled());
    }

    #[test]
    fn test_parse_state_finish() {
        let state = ParseState::new();
        state.start().unwrap();
        state.request_cancel();

        assert!(*state.is_running.lock().unwrap());
        assert!(state.is_cancelled());

        state.finish();

        assert!(!*state.is_running.lock().unwrap());
        assert!(!state.is_cancelled());
    }

    #[test]
    fn test_parse_state_finish_when_not_running() {
        let state = ParseState::new();
        // finishを呼んでもパニックしない
        state.finish();
        assert!(!*state.is_running.lock().unwrap());
    }

    #[test]
    fn test_parse_state_multiple_cycles() {
        let state = ParseState::new();

        // サイクル1
        state.start().unwrap();
        state.request_cancel();
        state.finish();

        // サイクル2
        state.start().unwrap();
        assert!(!state.is_cancelled());
        state.finish();

        // サイクル3
        state.start().unwrap();
        state.finish();

        assert!(!*state.is_running.lock().unwrap());
    }

    // ==================== get_body_for_parse Tests ====================

    #[test]
    fn test_get_body_for_parse_prefers_html() {
        let row = EmailRow {
            email_id: 1,
            message_id: "m1".to_string(),
            body_plain: Some("plain text".to_string()),
            body_html: Some("<p>html</p>".to_string()),
            from_address: None,
            subject: None,
            internal_date: None,
        };
        assert_eq!(get_body_for_parse(&row), "<p>html</p>");
    }

    #[test]
    fn test_get_body_for_parse_fallback_to_plain() {
        let row = EmailRow {
            email_id: 1,
            message_id: "m1".to_string(),
            body_plain: Some("plain text".to_string()),
            body_html: None,
            from_address: None,
            subject: None,
            internal_date: None,
        };
        assert_eq!(get_body_for_parse(&row), "plain text");
    }

    #[test]
    fn test_get_body_for_parse_html_only() {
        let row = EmailRow {
            email_id: 1,
            message_id: "m1".to_string(),
            body_plain: None,
            body_html: Some("<p>注文番号:12345</p>".to_string()),
            from_address: None,
            subject: None,
            internal_date: None,
        };
        let body = get_body_for_parse(&row);
        assert!(body.contains("注文番号:12345"));
        assert!(body.contains("<p>")); // HTML は生のまま返す（DMM 等が HTML からパースするため）
    }

    #[test]
    fn test_get_body_for_parse_empty_plain_uses_html() {
        let row = EmailRow {
            email_id: 1,
            message_id: "m1".to_string(),
            body_plain: Some("".to_string()),
            body_html: Some("<div>内容</div>".to_string()),
            from_address: None,
            subject: None,
            internal_date: None,
        };
        let body = get_body_for_parse(&row);
        assert!(body.contains("内容"));
        assert!(body.contains("<div>")); // HTML は生のまま返す
    }

    // ==================== Data Structure Tests ====================

    #[test]
    fn test_order_info_structure() {
        let order = OrderInfo {
            order_number: "ORD-001".to_string(),
            order_date: Some("2024-01-01".to_string()),
            delivery_address: None,
            delivery_info: None,
            items: vec![],
            subtotal: Some(1000),
            shipping_fee: Some(500),
            total_amount: Some(1500),
        };

        assert_eq!(order.order_number, "ORD-001");
        assert_eq!(order.order_date, Some("2024-01-01".to_string()));
        assert_eq!(order.total_amount, Some(1500));
    }

    #[test]
    fn test_order_info_with_items() {
        let item = OrderItem {
            name: "Test Product".to_string(),
            manufacturer: Some("Test Maker".to_string()),
            model_number: Some("MODEL-001".to_string()),
            unit_price: 1000,
            quantity: 2,
            subtotal: 2000,
            image_url: None,
        };

        let order = OrderInfo {
            order_number: "ORD-002".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![item],
            subtotal: None,
            shipping_fee: None,
            total_amount: None,
        };

        assert_eq!(order.items.len(), 1);
        assert_eq!(order.items[0].name, "Test Product");
        assert_eq!(order.items[0].subtotal, 2000);
    }

    #[test]
    fn test_delivery_address_structure() {
        let addr = DeliveryAddress {
            name: "山田太郎".to_string(),
            postal_code: Some("123-4567".to_string()),
            address: Some("東京都渋谷区".to_string()),
        };

        assert_eq!(addr.name, "山田太郎");
        assert_eq!(addr.postal_code, Some("123-4567".to_string()));
    }

    #[test]
    fn test_delivery_info_structure() {
        let info = DeliveryInfo {
            carrier: "ヤマト運輸".to_string(),
            tracking_number: "1234-5678-9012".to_string(),
            delivery_date: Some("2024-01-15".to_string()),
            delivery_time: Some("14:00-16:00".to_string()),
            carrier_url: Some("https://example.com/track".to_string()),
            delivery_status: None,
        };

        assert_eq!(info.carrier, "ヤマト運輸");
        assert_eq!(info.tracking_number, "1234-5678-9012");
    }

    #[test]
    fn test_parse_metadata_structure() {
        let metadata = ParseMetadata {
            parse_status: "idle".to_string(),
            last_parse_started_at: None,
            last_parse_completed_at: None,
            total_parsed_count: 0,
            last_error_message: None,
            batch_size: 100,
        };

        assert_eq!(metadata.parse_status, "idle");
        assert_eq!(metadata.batch_size, 100);
    }

    // ==================== Serialization Tests ====================

    #[test]
    fn test_order_info_serialization() {
        let order = OrderInfo {
            order_number: "ORD-003".to_string(),
            order_date: Some("2024-01-01".to_string()),
            delivery_address: None,
            delivery_info: None,
            items: vec![],
            subtotal: Some(1000),
            shipping_fee: None,
            total_amount: Some(1000),
        };

        let json = serde_json::to_string(&order).unwrap();
        assert!(json.contains("ORD-003"));

        let deserialized: OrderInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.order_number, "ORD-003");
    }

    #[test]
    fn test_parse_metadata_serialization() {
        let metadata = ParseMetadata {
            parse_status: "running".to_string(),
            last_parse_started_at: Some("2024-01-01T10:00:00Z".to_string()),
            last_parse_completed_at: None,
            total_parsed_count: 50,
            last_error_message: None,
            batch_size: 200,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: ParseMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.parse_status, "running");
        assert_eq!(deserialized.total_parsed_count, 50);
    }

    #[test]
    fn test_order_item_serialization() {
        let item = OrderItem {
            name: "ガンプラ HG".to_string(),
            manufacturer: Some("バンダイ".to_string()),
            model_number: Some("BAN-001".to_string()),
            unit_price: 2500,
            quantity: 1,
            subtotal: 2500,
            image_url: None,
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("ガンプラ HG"));
        assert!(json.contains("バンダイ"));

        let deserialized: OrderItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.unit_price, 2500);
    }

    #[test]
    fn test_delivery_info_serialization() {
        let info = DeliveryInfo {
            carrier: "佐川急便".to_string(),
            tracking_number: "9999-8888-7777".to_string(),
            delivery_date: None,
            delivery_time: None,
            carrier_url: None,
            delivery_status: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: DeliveryInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.carrier, "佐川急便");
        assert_eq!(deserialized.tracking_number, "9999-8888-7777");
    }

    // ==================== Clone Tests ====================

    #[test]
    fn test_parse_state_clone() {
        let state = ParseState::new();
        state.start().unwrap();

        let cloned = state.clone();
        // クローンは同じArcを共有
        assert!(*cloned.is_running.lock().unwrap());

        state.finish();
        assert!(!*cloned.is_running.lock().unwrap());
    }

    #[test]
    fn test_order_info_clone() {
        let order = OrderInfo {
            order_number: "ORD-CLONE".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![OrderItem {
                name: "Item".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 100,
                quantity: 1,
                subtotal: 100,
                image_url: None,
            }],
            subtotal: None,
            shipping_fee: None,
            total_amount: None,
        };

        let cloned = order.clone();
        assert_eq!(cloned.order_number, "ORD-CLONE");
        assert_eq!(cloned.items.len(), 1);
    }
}
