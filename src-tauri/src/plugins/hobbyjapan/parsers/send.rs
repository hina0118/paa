use super::{
    body_to_lines, extract_carrier, extract_items, extract_order_number, extract_tracking_number,
};
use crate::parsers::{DeliveryInfo, EmailParser, OrderInfo};
use crate::plugins::JAPANPOST_TRACKING_URL;

/// HJ OnlineShop 発送通知メール用パーサー
///
/// 件名：`【HJ OnlineShop】ご注文品発送のお知らせ`
/// 送信元：`shop@hobbyjapan.co.jp`
///
/// プレーンテキスト形式（ISO-2022-JP / 7bit）。
/// 金額・注文日は confirm 側で登録済みのため None を返す。
pub struct HjSendParser;

impl EmailParser for HjSendParser {
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

        let carrier_url =
            if carrier.contains("ゆうパック") || carrier.contains("ゆうパケット") {
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
        "原田\u{3000}裕基\u{3000}様\n\
         \n\
         いつもご利用いただき、誠にありがとうございます。\n\
         ホビージャパンオンラインショップです。\n\
         \n\
         本日ご注文品を発送いたしました。\n\
         \n\
         --------------------------------------------------\n\
         【オーダーID】HJ20260321_051158_43\n\
         【ご注文日】2026年03月21日\n\
         【ご注文者】原田\u{3000}裕基\u{3000}様\n\
         \n\
         【お届け先】\n\
         〒8120044\n\
         福岡県福岡市博多区住代1-7-1リアン・シェルブルー東公園\u{3000}505号\n\
         TEL09016717298\n\
         原田\u{3000}裕基\u{3000}様\n\
         \n\
         【ご注文品】\n\
         \u{3000}1.GALHolic深青【2026年5月再生産分】（2026年5月〜6月順次発送予定）\n\
         \u{3000}\u{3000}価格：\u{FFE5}7,880 x 数量：1 = 合計：\u{FFE5}7,880\n\
         \n\
         【お買上げ金額】\n\
         \u{3000}商品金額合計：\u{FFE5}7,880\n\
         \u{3000}送料：\u{FFE5}700\n\
         \u{3000}手数料：\u{FFE5}0\n\
         \u{3000}注文金額合計：\u{FFE5}8,580\n\
         \u{3000}\n\
         【お支払い方法】クレジットカード決済\n\
         \n\
         【配送希望日】希望なし\n\
         【配送希望時間帯】希望なし\n\
         \n\
         【配送業者】日本郵便 ゆうパック\n\
         【荷物お問い合わせ番号】560045430206\n\
         --------------------------------------------------\n"
    }

    #[test]
    fn test_parse_order_number() {
        let order = HjSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.order_number, "HJ20260321_051158_43");
    }

    #[test]
    fn test_parse_carrier() {
        let order = HjSendParser.parse(sample_send()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.carrier, "日本郵便 ゆうパック");
    }

    #[test]
    fn test_parse_tracking_number() {
        let order = HjSendParser.parse(sample_send()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.tracking_number, "560045430206");
    }

    #[test]
    fn test_parse_carrier_url() {
        let order = HjSendParser.parse(sample_send()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(
            delivery.carrier_url,
            Some(crate::plugins::JAPANPOST_TRACKING_URL.to_string())
        );
    }

    #[test]
    fn test_parse_item_count() {
        let order = HjSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.items.len(), 1);
    }

    #[test]
    fn test_parse_item_name() {
        let order = HjSendParser.parse(sample_send()).unwrap();
        assert_eq!(
            order.items[0].name,
            "GALHolic深青【2026年5月再生産分】（2026年5月〜6月順次発送予定）"
        );
    }

    #[test]
    fn test_parse_no_order_date() {
        let order = HjSendParser.parse(sample_send()).unwrap();
        assert!(order.order_date.is_none());
    }

    #[test]
    fn test_parse_no_amounts() {
        let order = HjSendParser.parse(sample_send()).unwrap();
        assert!(order.subtotal.is_none());
        assert!(order.shipping_fee.is_none());
        assert!(order.total_amount.is_none());
    }

    #[test]
    fn test_parse_no_order_number_returns_error() {
        let body = "【ご注文品】\n\u{3000}1.商品A\n\u{3000}\u{3000}価格：\u{FFE5}1,000 x 数量：1 = 合計：\u{FFE5}1,000\n【お買上げ金額】\n【配送業者】日本郵便 ゆうパック\n【荷物お問い合わせ番号】560045430206\n";
        assert!(HjSendParser.parse(body).is_err());
    }

    #[test]
    fn test_parse_no_carrier_returns_error() {
        let body = "【オーダーID】HJ20260321_051158_43\n【ご注文品】\n\u{3000}1.商品A\n\u{3000}\u{3000}価格：\u{FFE5}1,000 x 数量：1 = 合計：\u{FFE5}1,000\n【お買上げ金額】\n【荷物お問い合わせ番号】560045430206\n";
        assert!(HjSendParser.parse(body).is_err());
    }

    #[test]
    fn test_parse_no_tracking_number_returns_error() {
        let body = "【オーダーID】HJ20260321_051158_43\n【ご注文品】\n\u{3000}1.商品A\n\u{3000}\u{3000}価格：\u{FFE5}1,000 x 数量：1 = 合計：\u{FFE5}1,000\n【お買上げ金額】\n【配送業者】日本郵便 ゆうパック\n";
        assert!(HjSendParser.parse(body).is_err());
    }

    #[test]
    fn test_parse_no_items_returns_error() {
        let body = "【オーダーID】HJ20260321_051158_43\n【ご注文品】\n【お買上げ金額】\n【配送業者】日本郵便 ゆうパック\n【荷物お問い合わせ番号】560045430206\n";
        assert!(HjSendParser.parse(body).is_err());
    }
}
