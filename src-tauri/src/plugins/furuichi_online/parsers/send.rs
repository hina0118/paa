use super::{
    body_to_lines, extract_carrier, extract_items, extract_order_number, extract_tracking_number,
};
use crate::parsers::{DeliveryInfo, EmailParser, OrderInfo};
use crate::plugins::JAPANPOST_TRACKING_URL;

/// ふるいちオンライン 発送通知メール用パーサー
///
/// 件名：`【ふるいちオンライン】商品発送のお知らせ`
/// 送信元：`info@furu1.online`
///
/// プレーンテキスト形式（quoted-printable UTF-8）。
/// この送信メールには金額情報が含まれず、本パーサーも金額系フィールドを
/// 返さない／更新しない（価格情報は confirm 側で登録済みのものを利用する）。
pub struct FuruichiSendParser;

impl EmailParser for FuruichiSendParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let body_lines = body_to_lines(email_body);
        let lines: Vec<&str> = body_lines.iter().map(|s| s.as_str()).collect();

        let order_number =
            extract_order_number(&lines).ok_or_else(|| "Order number not found".to_string())?;

        let carrier = extract_carrier(&lines).ok_or_else(|| "Carrier not found".to_string())?;

        let tracking_number = extract_tracking_number(&lines)
            .ok_or_else(|| "Tracking number not found".to_string())?;

        let items = extract_items(&lines);
        if items.is_empty() {
            return Err("No items found".to_string());
        }

        // ゆうパケット・ゆうパック系の場合は日本郵便追跡 URL を設定する
        // 他の配送会社は carrier_url なし（delivery_check が別途対応）
        let carrier_url = if carrier.contains("ゆうパケット") || carrier.contains("ゆうパック")
        {
            Some(JAPANPOST_TRACKING_URL.to_string())
        } else {
            None
        };

        let delivery_info = DeliveryInfo {
            carrier,
            tracking_number,
            delivery_date: None,
            delivery_time: None,
            carrier_url,
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
        r#"---------------------------------------------
このメールはお客様の注文に関する大切なメールです。
届くまで保存してください。
本メールは自動配信メールです。
---------------------------------------------


山田太郎様

この度は、ふるいちオンラインをご利用いただきありがとうございます。

ご注文の商品を本日発送いたしました。

発送内容
---------------------------------------------
配送会社：ゆうパケット
伝票番号：680156937342



下記URLで伝票番号を入れて調べると、荷物の現在位置が確認できます。
---------------------------------------------
日本郵スポ
https://trackings.post.japanpost.jp/services/srv/search/input

佐川急便
http://k2k.sagawa-exp.co.jp/p/sagawa/web/okurijoinput.jsp
---------------------------------------------

ご注文内容
---------------------------------------------
ご注文番号：100409780
ご注文日：2026-03-03 22:25:08
ご注文者名：山田太郎
お支払い方法：Amazon Pay
---------------------------------------------
お届け先
〒1000001
東京都千代田区丸の内1-1-1 テストマンション101号

Tel：09016717298
山田太郎様
---------------------------------------------
ご注文商品：
03ゼウスⅠ　カルノージャート:1個
030カルノージャート　エクサ:1個

---------------------------------------------
商品小計（税込）「6,158」円
送料(税込)「0」円
クーポン利用「0」円
ポイント利用「0」ポイント

---------------------------------------------
ご注文金額合計（税込）「6,158」円
---------------------------------------------
"#
    }

    #[test]
    fn test_parse_send_order_number() {
        let order = FuruichiSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.order_number, "100409780");
    }

    #[test]
    fn test_parse_send_carrier() {
        let order = FuruichiSendParser.parse(sample_send()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.carrier, "ゆうパケット");
    }

    #[test]
    fn test_parse_send_tracking_number() {
        let order = FuruichiSendParser.parse(sample_send()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.tracking_number, "680156937342");
    }

    #[test]
    fn test_parse_send_carrier_url_yupacket() {
        let order = FuruichiSendParser.parse(sample_send()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(
            delivery.carrier_url,
            Some(crate::plugins::JAPANPOST_TRACKING_URL.to_string())
        );
    }

    #[test]
    fn test_parse_send_carrier_url_other() {
        // ゆうパケット・ゆうパック以外は carrier_url なし
        let body = "ご注文番号：100409780\n配送会社：佐川急便\n伝票番号：123456789012\nご注文商品：\n商品A:1個\n-----";
        let order = FuruichiSendParser.parse(body).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.carrier, "佐川急便");
        assert!(delivery.carrier_url.is_none());
    }

    #[test]
    fn test_parse_send_item_count() {
        let order = FuruichiSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_send_item_names() {
        let order = FuruichiSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.items[0].name, "03ゼウスⅠ　カルノージャート");
        assert_eq!(order.items[1].name, "030カルノージャート　エクサ");
    }

    #[test]
    fn test_parse_send_no_amounts() {
        // send パーサーは金額を返さない（confirm の価格を保持するため）
        let order = FuruichiSendParser.parse(sample_send()).unwrap();
        assert!(order.subtotal.is_none());
        assert!(order.shipping_fee.is_none());
        assert!(order.total_amount.is_none());
    }

    #[test]
    fn test_parse_send_no_order_date() {
        let order = FuruichiSendParser.parse(sample_send()).unwrap();
        assert!(order.order_date.is_none());
    }

    #[test]
    fn test_parse_send_no_order_number_returns_error() {
        let body = "配送会社：ゆうパケット\n伝票番号：680156937342\nご注文商品：\n商品A:1個\n-----";
        assert!(FuruichiSendParser.parse(body).is_err());
    }

    #[test]
    fn test_parse_send_no_carrier_returns_error() {
        let body = "ご注文番号：100409780\n伝票番号：680156937342\nご注文商品：\n商品A:1個\n-----";
        assert!(FuruichiSendParser.parse(body).is_err());
    }

    #[test]
    fn test_parse_send_no_tracking_number_returns_error() {
        let body = "ご注文番号：100409780\n配送会社：ゆうパケット\nご注文商品：\n商品A:1個\n-----";
        assert!(FuruichiSendParser.parse(body).is_err());
    }

    #[test]
    fn test_parse_send_no_items_returns_error() {
        let body = "ご注文番号：100409780\n配送会社：ゆうパケット\n伝票番号：680156937342\nご注文商品：\n-----";
        assert!(FuruichiSendParser.parse(body).is_err());
    }
}
