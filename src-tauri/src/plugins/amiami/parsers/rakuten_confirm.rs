use super::{
    body_to_lines, extract_order_number, extract_rakuten_items, extract_shipping_fee,
    extract_total_amount,
};
use crate::parsers::{EmailParser, OrderInfo};

/// あみあみ楽天市場 注文確認メール用パーサー
///
/// 件名：`[739419973] あみあみ ご注文確認案内`
/// 送信元：`amiami@shop.rakuten.co.jp`
///
/// 注文日は本文に含まれないため、`dispatch()` 側で `apply_internal_date()` を使用する。
pub struct AmiamiRakutenConfirmParser;

impl EmailParser for AmiamiRakutenConfirmParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let body_lines = body_to_lines(email_body);
        let lines: Vec<&str> = body_lines.iter().map(|s| s.as_str()).collect();

        let order_number =
            extract_order_number(&lines).ok_or_else(|| "Order number not found".to_string())?;

        let items = extract_rakuten_items(&lines);
        if items.is_empty() {
            return Err("No items found".to_string());
        }

        let shipping_fee = extract_shipping_fee(&lines);
        let total_amount = extract_total_amount(&lines);

        Ok(OrderInfo {
            order_number,
            order_date: None, // internal_date で補完
            delivery_address: None,
            delivery_info: None,
            items,
            subtotal: None,
            shipping_fee,
            total_amount,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_confirm() -> &'static str {
        r#"お客様のご注文は受注番号：739419973 でお承りしました。
（楽天市場でのご注文番号とは異なります）
ご注文内容は以下の通りとなりますのでご確認ください。

受注番号：739419973
発送予定：
＜                     商品名              | 単価 | 数量 | 金額 ＞
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
30MM 1/144 eEXM-21 ラビオット [ネイビー] プラモデル（再販）[BANDAI SPIRITS]【発売済・在庫品】 | 1,411円 | 1 | 1,411円
30MM 1/144 eEXM-17 アルト[パープル] プラモデル[BANDAI SPIRITS]【発売済・在庫品】 | 1,050円 | 1 | 1,050円
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
      　    送料 : 500円
    　合計金額 : 2,961円

    　支払い方法 : クレジット通常
    　配送方法 : 普通郵便
    　時間指定 : 19〜21時
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
クレジット決済額 : 2,961円
"#
    }

    #[test]
    fn test_parse_confirm_order_number() {
        let order = AmiamiRakutenConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.order_number, "739419973");
    }

    #[test]
    fn test_parse_confirm_no_order_date() {
        // 注文日は本文にないため None
        let order = AmiamiRakutenConfirmParser.parse(sample_confirm()).unwrap();
        assert!(order.order_date.is_none());
    }

    #[test]
    fn test_parse_confirm_item_count() {
        let order = AmiamiRakutenConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_confirm_item_names() {
        let order = AmiamiRakutenConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(
            order.items[0].name,
            "30MM 1/144 eEXM-21 ラビオット [ネイビー] プラモデル（再販）[BANDAI SPIRITS]【発売済・在庫品】"
        );
        assert_eq!(
            order.items[1].name,
            "30MM 1/144 eEXM-17 アルト[パープル] プラモデル[BANDAI SPIRITS]【発売済・在庫品】"
        );
    }

    #[test]
    fn test_parse_confirm_item_prices() {
        let order = AmiamiRakutenConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].unit_price, 1411);
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[0].subtotal, 1411);
        assert_eq!(order.items[1].unit_price, 1050);
        assert_eq!(order.items[1].quantity, 1);
        assert_eq!(order.items[1].subtotal, 1050);
    }

    #[test]
    fn test_parse_confirm_shipping_fee() {
        let order = AmiamiRakutenConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.shipping_fee, Some(500));
    }

    #[test]
    fn test_parse_confirm_total_amount() {
        let order = AmiamiRakutenConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.total_amount, Some(2961));
    }

    #[test]
    fn test_parse_confirm_no_delivery_info() {
        let order = AmiamiRakutenConfirmParser.parse(sample_confirm()).unwrap();
        assert!(order.delivery_info.is_none());
    }

    #[test]
    fn test_parse_confirm_no_order_number_returns_error() {
        let sep = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
        let body = format!("{sep}\n商品A | 1,000円 | 1 | 1,000円\n{sep}");
        assert!(AmiamiRakutenConfirmParser.parse(&body).is_err());
    }

    #[test]
    fn test_parse_confirm_no_items_returns_error() {
        let body = "受注番号：739419973\n送料 : 500円";
        assert!(AmiamiRakutenConfirmParser.parse(body).is_err());
    }
}
