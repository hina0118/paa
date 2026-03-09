use super::{
    body_to_lines, extract_direct_items, extract_direct_shipping_fee, extract_direct_subtotal,
    extract_direct_total, extract_order_number_quoted,
};
use crate::parsers::{EmailParser, OrderInfo};

/// あみあみ直販 注文確認メール用パーサー
///
/// 件名：`あみあみ ご注文[219908570]内容確認`
/// 送信元：`order@amiami.com`
///
/// 注文番号は `受注番号 "219908570"` 形式（引用符付き）。
/// 注文日は本文に含まれないため、`dispatch()` 側で `apply_internal_date()` を使用する。
pub struct AmiamiConfirmParser;

impl EmailParser for AmiamiConfirmParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let body_lines = body_to_lines(email_body);
        let lines: Vec<&str> = body_lines.iter().map(|s| s.as_str()).collect();

        let order_number = extract_order_number_quoted(&lines)
            .ok_or_else(|| "Order number not found".to_string())?;

        let items = extract_direct_items(&lines);
        if items.is_empty() {
            return Err("No items found".to_string());
        }

        let subtotal = extract_direct_subtotal(&lines);
        let shipping_fee = extract_direct_shipping_fee(&lines);
        let total_amount = extract_direct_total(&lines);

        Ok(OrderInfo {
            order_number,
            order_date: None, // internal_date で補完
            delivery_address: None,
            delivery_info: None,
            items,
            subtotal,
            shipping_fee,
            total_amount,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_confirm() -> &'static str {
        r#"「あみあみ」をご利用頂き、誠にありがとうございます。

お客様のご注文は受注番号 "219908570"にて承りました。

【発売済み・在庫品】のみのご注文に対するクレジットカードでのお支払い手続きを承りました。

◆受注番号　　　：219908570
◆お支払い方法　：クレジットカード
商品名：[中古](本BランクA-/箱B)HG ブレイサー・フェニックス 「パシフィック・リム：アップライジング」より プラモデル[バンダイ]【発売済・在庫品】
単価：\1,690
個数：1
小計：\1,690

商品名：[中古](本BランクA-/箱B)ダンボール戦機 プラモデル 009 LBXジョーカー[バンダイ]【発売済・在庫品】
単価：\980
個数：1
小計：\980

商品名：[中古](本BランクA-/箱B)LBXナイトメア プラモデル 「ダンボール戦機」[BANDAI SPIRITS]【発売済・在庫品】
単価：\1,180
個数：1
小計：\1,180

商品名：メダロット 1/6 KXK00-M クロスメサイア プラモデル[コトブキヤ]【発売済・在庫品】
単価：\3,630
個数：1
小計：\3,630


●小計　　　　　：\7,480
●送料　　　　　：\500
●合計　　　　　：7,980円

◆決済方法　　：クレジットカード
"#
    }

    #[test]
    fn test_parse_confirm_order_number() {
        let order = AmiamiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.order_number, "219908570");
    }

    #[test]
    fn test_parse_confirm_no_order_date() {
        let order = AmiamiConfirmParser.parse(sample_confirm()).unwrap();
        assert!(order.order_date.is_none());
    }

    #[test]
    fn test_parse_confirm_item_count() {
        let order = AmiamiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items.len(), 4);
    }

    #[test]
    fn test_parse_confirm_item_names() {
        let order = AmiamiConfirmParser.parse(sample_confirm()).unwrap();
        assert!(order.items[0].name.contains("HG ブレイサー・フェニックス"));
        assert!(order.items[3].name.contains("メダロット"));
    }

    #[test]
    fn test_parse_confirm_item_prices() {
        let order = AmiamiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].unit_price, 1690);
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[0].subtotal, 1690);
        assert_eq!(order.items[3].unit_price, 3630);
        assert_eq!(order.items[3].subtotal, 3630);
    }

    #[test]
    fn test_parse_confirm_subtotal() {
        let order = AmiamiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.subtotal, Some(7480));
    }

    #[test]
    fn test_parse_confirm_shipping_fee() {
        let order = AmiamiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.shipping_fee, Some(500));
    }

    #[test]
    fn test_parse_confirm_total_amount() {
        let order = AmiamiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.total_amount, Some(7980));
    }

    #[test]
    fn test_parse_confirm_no_delivery_info() {
        let order = AmiamiConfirmParser.parse(sample_confirm()).unwrap();
        assert!(order.delivery_info.is_none());
    }

    #[test]
    fn test_parse_confirm_no_order_number_returns_error() {
        let body =
            "商品名：テスト商品\n単価：\\1,000\n個数：1\n小計：\\1,000\n■合計　　　：1,000円";
        assert!(AmiamiConfirmParser.parse(body).is_err());
    }

    #[test]
    fn test_parse_confirm_no_items_returns_error() {
        let body = r#"受注番号 "219908570"にて承りました。"#;
        assert!(AmiamiConfirmParser.parse(body).is_err());
    }
}
