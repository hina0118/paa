//! ご注文まとめ完了メールから抽出した情報（全店舗共通）

/// まとめ完了メールから抽出した情報（複数注文 → 1注文に統合）
#[derive(Debug, Clone)]
pub struct ConsolidationInfo {
    /// まとめる前の注文番号リスト（重複ありの可能性）
    pub old_order_numbers: Vec<String>,
    /// まとめた後の注文番号（1件）
    pub new_order_number: String,
}
