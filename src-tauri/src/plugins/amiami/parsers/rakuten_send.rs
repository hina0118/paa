use super::{
    body_to_lines, detect_carrier, extract_order_number, extract_rakuten_items,
    extract_tracking_number,
};
use crate::parsers::{DeliveryInfo, EmailParser, OrderInfo};

/// あみあみ楽天市場 発送案内メール用パーサー
///
/// 件名：`[739419973] あみあみ発送案内`
/// 送信元：`amiami_2@shop.rakuten.co.jp`
///
/// 配送会社はメール本文中の追跡 URL から判定する（kuronekoyamato.co.jp → ヤマト運輸）。
/// 金額情報は含まれないため、confirm 側で登録済みの値を保持する。
pub struct AmiamiRakutenSendParser;

impl EmailParser for AmiamiRakutenSendParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let body_lines = body_to_lines(email_body);
        let lines: Vec<&str> = body_lines.iter().map(|s| s.as_str()).collect();

        let order_number =
            extract_order_number(&lines).ok_or_else(|| "Order number not found".to_string())?;

        let tracking_number = extract_tracking_number(&lines)
            .ok_or_else(|| "Tracking number not found".to_string())?;

        let carrier = detect_carrier(email_body)
            .ok_or_else(|| "Carrier not found".to_string())?;

        let items = extract_rakuten_items(&lines);
        if items.is_empty() {
            return Err("No items found".to_string());
        }

        let delivery_info = DeliveryInfo {
            carrier,
            tracking_number,
            delivery_date: None,
            delivery_time: None,
            carrier_url: None,
            delivery_status: None,
        };

        Ok(OrderInfo {
            order_number,
            order_date: None,
            delivery_address: None,
            delivery_info: Some(delivery_info),
            items,
            subtotal: None,
            shipping_fee: None,
            total_amount: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_send() -> &'static str {
        r#"以下のお荷物は本日、ヤマト運輸にて発送いたしました。

=========================================================================

パソコンによる配送状況の確認

　配送状況につきましては、以下のヤマト運輸のサイトでご確認できます。

 荷物お問合せ番号：397404561713
・ヤマト運輸 荷物追跡ページ
　http://www.kuronekoyamato.co.jp/top.html

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
    fn test_parse_send_order_number() {
        let order = AmiamiRakutenSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.order_number, "739419973");
    }

    #[test]
    fn test_parse_send_tracking_number() {
        let order = AmiamiRakutenSendParser.parse(sample_send()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.tracking_number, "397404561713");
    }

    #[test]
    fn test_parse_send_carrier_yamato() {
        let order = AmiamiRakutenSendParser.parse(sample_send()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.carrier, "ヤマト運輸");
    }

    #[test]
    fn test_parse_send_carrier_url_none() {
        // carrier_url は None（delivery_check に委ねる）
        let order = AmiamiRakutenSendParser.parse(sample_send()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert!(delivery.carrier_url.is_none());
    }

    #[test]
    fn test_parse_send_item_count() {
        let order = AmiamiRakutenSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_send_item_names() {
        let order = AmiamiRakutenSendParser.parse(sample_send()).unwrap();
        assert_eq!(
            order.items[0].name,
            "30MM 1/144 eEXM-21 ラビオット [ネイビー] プラモデル（再販）[BANDAI SPIRITS]【発売済・在庫品】"
        );
    }

    #[test]
    fn test_parse_send_no_amounts() {
        let order = AmiamiRakutenSendParser.parse(sample_send()).unwrap();
        assert!(order.subtotal.is_none());
        assert!(order.shipping_fee.is_none());
        assert!(order.total_amount.is_none());
    }

    #[test]
    fn test_parse_send_no_order_number_returns_error() {
        let sep = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
        let body = format!("荷物お問合せ番号：397404561713\nhttp://www.kuronekoyamato.co.jp/\n{sep}\n商品A | 500円 | 1 | 500円\n{sep}");
        assert!(AmiamiRakutenSendParser.parse(&body).is_err());
    }

    #[test]
    fn test_parse_send_no_tracking_number_returns_error() {
        let sep = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
        let body = format!("受注番号：739419973\nhttp://www.kuronekoyamato.co.jp/\n{sep}\n商品A | 500円 | 1 | 500円\n{sep}");
        assert!(AmiamiRakutenSendParser.parse(&body).is_err());
    }

    #[test]
    fn test_parse_send_no_carrier_returns_error() {
        let sep = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
        let body = format!("受注番号：739419973\n荷物お問合せ番号：397404561713\n{sep}\n商品A | 500円 | 1 | 500円\n{sep}");
        assert!(AmiamiRakutenSendParser.parse(&body).is_err());
    }

    #[test]
    fn test_parse_send_no_items_returns_error() {
        let body = "受注番号：739419973\n荷物お問合せ番号：397404561713\nhttp://www.kuronekoyamato.co.jp/";
        assert!(AmiamiRakutenSendParser.parse(body).is_err());
    }
}
