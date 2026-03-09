use super::{
    body_to_lines, detect_carrier, extract_order_number, extract_rakuten_items,
    extract_tracking_number,
};
use crate::parsers::{DeliveryInfo, EmailParser, OrderInfo};

/// あみあみ直販 発送案内メール用パーサー
///
/// 件名：`[219908570] あみあみ発送案内`
/// 送信元：`shop@amiami.com`
///
/// 注文番号は `受注番号：219908570` 形式（引用符なし）。
/// 商品テーブル形式は楽天と同一（パイプ区切り）。
/// 配送会社はメール本文中の追跡 URL から判定する（sagawa-exp.co.jp → 佐川急便）。
/// 金額情報は含まれないため、confirm 側で登録済みの値を保持する。
pub struct AmiamiSendParser;

impl EmailParser for AmiamiSendParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let body_lines = body_to_lines(email_body);
        let lines: Vec<&str> = body_lines.iter().map(|s| s.as_str()).collect();

        let order_number =
            extract_order_number(&lines).ok_or_else(|| "Order number not found".to_string())?;

        let tracking_number = extract_tracking_number(&lines)
            .ok_or_else(|| "Tracking number not found".to_string())?;

        let carrier = detect_carrier(email_body).ok_or_else(|| "Carrier not found".to_string())?;

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
        r#"以下のお荷物は本日、佐川急便にて発送いたしました。

=========================================================================

パソコンによる配送状況の確認

　配送状況につきましては、以下の佐川急便のサイトでご確認できます。

 荷物お問合せ番号：515596488142
・佐川急便 荷物追跡ページ
　https://www.sagawa-exp.co.jp/

受注番号：219908570
発送予定：
＜                     商品名              | 単価 | 数量 | 金額 ＞
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
メダロット 1/6 KXK00-M クロスメサイア プラモデル[コトブキヤ]【発売済・在庫品】 | 3,630円 | 1 | 3,630円
[中古](本BランクA-/箱B)LBXナイトメア プラモデル 「ダンボール戦機」[BANDAI SPIRITS]【発売済・在庫品】 | 1,180円 | 1 | 1,180円
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
      　    送料 : 500円
    　合計金額 : 7,980円

    　支払い方法 : クレジット通常
    　配送方法 : 普通郵便
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
クレジット決済額 : 7,980円
"#
    }

    #[test]
    fn test_parse_send_order_number() {
        let order = AmiamiSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.order_number, "219908570");
    }

    #[test]
    fn test_parse_send_tracking_number() {
        let order = AmiamiSendParser.parse(sample_send()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.tracking_number, "515596488142");
    }

    #[test]
    fn test_parse_send_carrier_sagawa() {
        let order = AmiamiSendParser.parse(sample_send()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.carrier, "佐川急便");
    }

    #[test]
    fn test_parse_send_carrier_url_none() {
        let order = AmiamiSendParser.parse(sample_send()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert!(delivery.carrier_url.is_none());
    }

    #[test]
    fn test_parse_send_item_count() {
        let order = AmiamiSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_send_item_names() {
        let order = AmiamiSendParser.parse(sample_send()).unwrap();
        assert!(order.items[0].name.contains("クロスメサイア"));
        assert!(order.items[1].name.contains("LBXナイトメア"));
    }

    #[test]
    fn test_parse_send_no_amounts() {
        let order = AmiamiSendParser.parse(sample_send()).unwrap();
        assert!(order.subtotal.is_none());
        assert!(order.shipping_fee.is_none());
        assert!(order.total_amount.is_none());
    }

    #[test]
    fn test_parse_send_no_order_number_returns_error() {
        let sep = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
        let body = format!("荷物お問合せ番号：515596488142\nhttps://www.sagawa-exp.co.jp/\n{sep}\n商品A | 500円 | 1 | 500円\n{sep}");
        assert!(AmiamiSendParser.parse(&body).is_err());
    }

    #[test]
    fn test_parse_send_no_tracking_number_returns_error() {
        let sep = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
        let body = format!("受注番号：219908570\nhttps://www.sagawa-exp.co.jp/\n{sep}\n商品A | 500円 | 1 | 500円\n{sep}");
        assert!(AmiamiSendParser.parse(&body).is_err());
    }

    #[test]
    fn test_parse_send_no_carrier_returns_error() {
        let sep = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
        let body = format!("受注番号：219908570\n荷物お問合せ番号：515596488142\n{sep}\n商品A | 500円 | 1 | 500円\n{sep}");
        assert!(AmiamiSendParser.parse(&body).is_err());
    }
}
