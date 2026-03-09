use crate::parsers::cancel_info::CancelInfo;
use once_cell::sync::Lazy;
use regex::Regex;

/// あみあみ キャンセルメール用パーサー
///
/// 以下の2種類のキャンセルメールに対応する。
///
/// ## `order@amiami.com` - キャンセルご依頼の内容確認
/// 件名：`あみあみ　キャンセルご依頼の内容確認`
/// 注文番号形式：`受注番号 : 226512861`
///
/// ## `shop@amiami.com` - キャンセルを承りました
/// 件名：`あみあみ　キャンセルを承りました`
/// 注文番号形式：`ご注文226512861以下商品につきまして`
///
/// いずれも `product_name = ""` で全件キャンセルとして処理する。
pub struct AmiamiCancelParser;

/// 注文番号を抽出するパターン（9桁以上）
///
/// - `受注番号 : 226512861` 形式（スペース + 半角コロン）
/// - `受注番号 ： 226512861` 形式（全角コロン）
/// - `ご注文226512861以下商品` 形式（直接埋め込み）
static ORDER_NUMBER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:受注番号\s*[：:]\s*|ご注文)(\d{9,})").expect("Invalid ORDER_NUMBER_RE")
});

impl AmiamiCancelParser {
    /// メール本文からキャンセル情報を抽出する
    pub fn parse_cancel(&self, email_body: &str) -> Result<CancelInfo, String> {
        let order_number = ORDER_NUMBER_RE
            .captures(email_body)
            .map(|c| c[1].to_string())
            .ok_or_else(|| "Order number not found".to_string())?;

        Ok(CancelInfo {
            order_number,
            // あみあみのキャンセルメールは注文全体のキャンセルを示すため
            // product_name を空にして全件削除として処理する
            product_name: String::new(),
            cancel_quantity: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cancel_request() -> &'static str {
        // order@amiami.com からの「キャンセルご依頼の内容確認」メール
        r#"山田 太郎 様

キャンセルリクエストを承りました。
スタッフが確認の上、翌営業日中には、可能であればキャンセル処理をおこないご案内差し上げます。

受注番号 : 226512861
全てキャンセル

[FIGURE-169665] : メガミデバイス PUNI☆MOFU トゥ 1/1 プラモデル[コトブキヤ]《１１月予約》
"#
    }

    fn sample_cancel_confirmed() -> &'static str {
        // shop@amiami.com からの「キャンセルを承りました」メール
        r#"お客様へ

　ご利用ありがとうございます。

　ご注文226512861以下商品につきまして、キャンセルとさせて頂きましたことご連絡さしあげます。

[メガミデバイス PUNI☆MOFU トゥ 1/1 プラモデル[コトブキヤ]《１１月予約》] x 1

　ありがとうございました。

2024.5.1
"#
    }

    #[test]
    fn test_parse_cancel_request_order_number() {
        let parser = AmiamiCancelParser;
        let result = parser.parse_cancel(sample_cancel_request()).unwrap();
        assert_eq!(result.order_number, "226512861");
    }

    #[test]
    fn test_parse_cancel_request_product_name_empty() {
        let parser = AmiamiCancelParser;
        let result = parser.parse_cancel(sample_cancel_request()).unwrap();
        // 全件キャンセルのため product_name は空
        assert!(result.product_name.is_empty());
    }

    #[test]
    fn test_parse_cancel_confirmed_order_number() {
        let parser = AmiamiCancelParser;
        let result = parser.parse_cancel(sample_cancel_confirmed()).unwrap();
        assert_eq!(result.order_number, "226512861");
    }

    #[test]
    fn test_parse_cancel_confirmed_product_name_empty() {
        let parser = AmiamiCancelParser;
        let result = parser.parse_cancel(sample_cancel_confirmed()).unwrap();
        assert!(result.product_name.is_empty());
    }

    #[test]
    fn test_parse_cancel_no_order_number_returns_error() {
        let body = "キャンセルリクエストを承りました。\n全てキャンセル";
        let parser = AmiamiCancelParser;
        assert!(parser.parse_cancel(body).is_err());
    }
}
