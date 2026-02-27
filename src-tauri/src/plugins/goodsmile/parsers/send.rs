use super::{
    extract_carrier, extract_delivery_time, extract_order_number, extract_send_items,
    extract_tracking_number,
};
use crate::parsers::{DeliveryInfo, EmailParser, OrderInfo};

/// グッドスマイルカンパニー 発送通知メール用パーサー
///
/// 件名：`ご注文商品発送のお知らせ(ご注文番号：XXXX）`
/// 送信元：`shop@goodsmile.jp`（SendGrid 経由）
///
/// テキストパートから注文番号・配送情報（追跡番号・配送業者）・商品情報を抽出する。
/// 金額情報は発送通知メールに含まれないため、subtotal / shipping_fee / total_amount は None。
/// 追跡番号は `追跡番号：http://...` の URL からではなく `配送番号：` 行から取得する。
pub struct GoodSmileSendParser;

impl EmailParser for GoodSmileSendParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let lines: Vec<&str> = email_body.lines().collect();

        let order_number = extract_order_number(&lines)
            .ok_or_else(|| "Order number not found".to_string())?;

        let items = extract_send_items(&lines);
        if items.is_empty() {
            return Err("No items found".to_string());
        }

        let tracking_number = extract_tracking_number(&lines)
            .ok_or_else(|| "Tracking number not found".to_string())?;

        let carrier =
            extract_carrier(&lines).ok_or_else(|| "Carrier not found".to_string())?;

        let delivery_time = extract_delivery_time(&lines);

        let delivery_info = DeliveryInfo {
            carrier,
            tracking_number,
            delivery_date: None,
            delivery_time,
            carrier_url: None,
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

    fn sample_send_email() -> &'static str {
        r#"※このメールはシステムより自動送信されています。
原田 裕基 様
日頃よりグッドスマイルカンパニー公式ショップをご利用いただき誠にありがとうございます。
ご注文いただいておりました商品の出荷が完了いたしました。
------------------------------------
ご注文詳細
注文番号: CpBk4quaORPw
お支払方法: クレジットカード
発送先情報:
8120044
福岡県
福岡市博多区
千代1-7-1
リアンシエルブルー東公園505号
原田 裕基
電話: 09016717298
配送情報:
配送番号：564841939476
MODEROID バーンドラゴン
4580590207912 1
配送元：佐川急便(送料無料)
配送時間：指定なし
追跡番号：http://k2k.sagawa-exp.co.jp/p/web/okurijosearch.do?okurijoNo=564841939476
------------------------------------
※追跡データが反映されるまで少々お時間がかかります。予めご了承下さい。
"#
    }

    #[test]
    fn test_parse_send_order_number() {
        let order = GoodSmileSendParser.parse(sample_send_email()).unwrap();
        assert_eq!(order.order_number, "CpBk4quaORPw");
    }

    #[test]
    fn test_parse_send_item_count() {
        let order = GoodSmileSendParser.parse(sample_send_email()).unwrap();
        assert_eq!(order.items.len(), 1);
    }

    #[test]
    fn test_parse_send_item_name() {
        let order = GoodSmileSendParser.parse(sample_send_email()).unwrap();
        assert_eq!(order.items[0].name, "MODEROID バーンドラゴン");
    }

    #[test]
    fn test_parse_send_item_jan_and_quantity() {
        let order = GoodSmileSendParser.parse(sample_send_email()).unwrap();
        assert_eq!(
            order.items[0].model_number,
            Some("4580590207912".to_string())
        );
        assert_eq!(order.items[0].quantity, 1);
    }

    #[test]
    fn test_parse_send_tracking_number() {
        let order = GoodSmileSendParser.parse(sample_send_email()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.tracking_number, "564841939476");
    }

    #[test]
    fn test_parse_send_carrier() {
        let order = GoodSmileSendParser.parse(sample_send_email()).unwrap();
        let delivery = order.delivery_info.unwrap();
        // 括弧以降（送料無料）は除去される
        assert_eq!(delivery.carrier, "佐川急便");
    }

    #[test]
    fn test_parse_send_delivery_time_shitenashi_is_none() {
        let order = GoodSmileSendParser.parse(sample_send_email()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert!(delivery.delivery_time.is_none());
    }

    #[test]
    fn test_parse_send_no_amounts() {
        let order = GoodSmileSendParser.parse(sample_send_email()).unwrap();
        assert!(order.subtotal.is_none());
        assert!(order.shipping_fee.is_none());
        assert!(order.total_amount.is_none());
    }

    #[test]
    fn test_parse_send_no_order_date() {
        let order = GoodSmileSendParser.parse(sample_send_email()).unwrap();
        assert!(order.order_date.is_none());
    }

    #[test]
    fn test_parse_send_no_order_number_returns_error() {
        let result = GoodSmileSendParser.parse(
            "配送情報:\n配送番号：123456\nテスト商品\n4580590207912 1\n配送元：佐川急便",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_send_no_items_returns_error() {
        let result =
            GoodSmileSendParser.parse("注文番号: ABC123\n配送情報:\n配送番号：123456\n配送元：佐川急便");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_send_no_tracking_number_returns_error() {
        let result = GoodSmileSendParser.parse(
            "注文番号: ABC123\n配送情報:\nテスト商品\n4580590207912 1\n配送元：佐川急便",
        );
        assert!(result.is_err());
    }
}
