//! 注文番号変更メールから抽出した情報（全店舗共通）

/// 注文番号変更メールから抽出した情報
#[derive(Debug, Clone)]
pub struct OrderNumberChangeInfo {
    pub old_order_number: String,
    pub new_order_number: String,
}
