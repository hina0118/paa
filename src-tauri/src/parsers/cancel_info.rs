//! キャンセルメールから抽出した情報（全店舗共通）

/// キャンセルメールから抽出した情報
#[derive(Debug, Clone)]
pub struct CancelInfo {
    pub order_number: String,
    pub product_name: String,
    pub cancel_quantity: i64,
}
