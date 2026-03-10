use super::{
    body_to_lines, extract_carrier, extract_order_number, extract_ship_date, extract_shipping_fee,
    extract_subtotal, extract_total_amount, extract_tracking_number,
};
use crate::parsers::{DeliveryInfo, EmailParser, OrderInfo};
use crate::plugins::JAPANPOST_TRACKING_URL;

/// 駿河屋 発送案内メール用パーサー
///
/// 件名：`発送のお知らせ` を含む
/// 送信元：`order@suruga-ya.jp`
///
/// 取引番号は `（取引番号：S2204166697）` 形式。
/// 商品一覧は発送メールに含まれないため `items` は空（confirm で登録済みの商品とマージ済み）。
/// 金額情報（商品合計・送料・支払合計金額）は本文から抽出する。
///
/// # 追跡番号について
/// ゆうパック等は `お問い合わせ番号：764336939516` として12桁の番号が記載される。
/// ゆうメール等の追跡不可配送は追跡番号フィールドが存在しない。
/// この場合、発送通知メールの受信をもって配達完了とみなし、`delivery_status = "delivered"` を設定する。
pub struct SurugayaSendParser;

impl EmailParser for SurugayaSendParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let body_lines = body_to_lines(email_body);
        let lines: Vec<&str> = body_lines.iter().map(|s| s.as_str()).collect();

        let order_number =
            extract_order_number(&lines).ok_or_else(|| "Order number not found".to_string())?;

        let tracking_number = extract_tracking_number(&lines);
        let carrier = extract_carrier(&lines);

        let delivery_date = extract_ship_date(&lines);
        let subtotal = extract_subtotal(&lines);
        let shipping_fee = extract_shipping_fee(&lines);
        let total_amount = extract_total_amount(&lines);

        // 追跡番号の有無で delivery_info の構築を切り替える
        let delivery_info = match tracking_number {
            Some(tracking) => {
                // ゆうパック等: 追跡番号あり → "shipped" (デフォルト) で登録、以降追跡で更新
                carrier.map(|c| DeliveryInfo {
                    carrier: c,
                    tracking_number: tracking,
                    delivery_date,
                    delivery_time: None,
                    carrier_url: Some(JAPANPOST_TRACKING_URL.to_string()),
                    delivery_status: None,
                })
            }
            None => {
                // ゆうメール等: 追跡不可 → 発送通知受信時点で配達完了とみなす
                carrier.map(|c| DeliveryInfo {
                    carrier: c,
                    tracking_number: String::new(),
                    delivery_date,
                    delivery_time: None,
                    carrier_url: None,
                    delivery_status: Some("delivered".to_string()),
                })
            }
        };

        Ok(OrderInfo {
            order_number,
            order_date: None,
            delivery_address: None,
            delivery_info,
            items: vec![],
            subtotal,
            shipping_fee,
            total_amount,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_send() -> &'static str {
        r#"原田裕基様 （取引番号：S2204166697）

「駿河屋」にてお買い上げいただき、誠にありがとうございます。

宅配業者、お問い合わせ番号、出荷日は以下のようになります。

商品合計　　　　　　：\9,108
代引手数料　　　　　：\0
送料　　　　　　　　：\0
支払合計金額　　　　：\9,108

お届け方法　　　　　：ゆうパック（日本郵便)
お問い合わせ番号　　：764336939516
到着予定日　　　　　：指定なし
配達時間　　　　　　：指定なし
出荷日　　　　　　　：2022/04/27

郵便追跡サービス
http://tracking.post.japanpost.jp/services/srv/search/
"#
    }

    fn sample_send_yumail() -> &'static str {
        // ゆうメール: 追跡番号なし（email 1090 相当）
        r#"原田裕基様 （取引番号：S2501067868）

「駿河屋」にてお買い上げいただき、誠にありがとうございます。

宅配業者、出荷日は以下のようになります。

商品合計　　　　　　：\5,100
代引手数料　　　　　：\0
送料・通信販売手数料：\0
支払合計金額　　　　：\5,100

お届け方法　　　　　：ゆうメール（日本郵便)
到着予定日　　　　　：指定なし
配達時間　　　　　　： -
出荷日　　　　　　　：2025/01/09
"#
    }

    // ─── ゆうパック（追跡あり）───

    #[test]
    fn test_parse_send_order_number() {
        let order = SurugayaSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.order_number, "S2204166697");
    }

    #[test]
    fn test_parse_send_tracking_number() {
        let order = SurugayaSendParser.parse(sample_send()).unwrap();
        assert_eq!(
            order.delivery_info.as_ref().unwrap().tracking_number,
            "764336939516"
        );
    }

    #[test]
    fn test_parse_send_carrier() {
        let order = SurugayaSendParser.parse(sample_send()).unwrap();
        assert_eq!(
            order.delivery_info.as_ref().unwrap().carrier,
            "ゆうパック（日本郵便）"
        );
    }

    #[test]
    fn test_parse_send_carrier_url() {
        let order = SurugayaSendParser.parse(sample_send()).unwrap();
        let url = order.delivery_info.as_ref().unwrap().carrier_url.as_deref();
        assert_eq!(url, Some(JAPANPOST_TRACKING_URL));
    }

    #[test]
    fn test_parse_send_delivery_date() {
        let order = SurugayaSendParser.parse(sample_send()).unwrap();
        assert_eq!(
            order
                .delivery_info
                .as_ref()
                .unwrap()
                .delivery_date
                .as_deref(),
            Some("2022-04-27 00:00:00")
        );
    }

    #[test]
    fn test_parse_send_subtotal() {
        let order = SurugayaSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.subtotal, Some(9108));
    }

    #[test]
    fn test_parse_send_shipping_fee_zero() {
        let order = SurugayaSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.shipping_fee, Some(0));
    }

    #[test]
    fn test_parse_send_total_amount() {
        let order = SurugayaSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.total_amount, Some(9108));
    }

    #[test]
    fn test_parse_send_items_empty() {
        // 発送メールには商品リストなし
        let order = SurugayaSendParser.parse(sample_send()).unwrap();
        assert!(order.items.is_empty());
    }

    // ─── ゆうメール（追跡なし）───

    #[test]
    fn test_parse_send_yumail_order_number() {
        let order = SurugayaSendParser.parse(sample_send_yumail()).unwrap();
        assert_eq!(order.order_number, "S2501067868");
    }

    #[test]
    fn test_parse_send_yumail_delivery_status_delivered() {
        // 追跡番号なし → 発送通知受信時点で配達完了とみなす
        let order = SurugayaSendParser.parse(sample_send_yumail()).unwrap();
        let di = order.delivery_info.as_ref().unwrap();
        assert_eq!(di.delivery_status.as_deref(), Some("delivered"));
    }

    #[test]
    fn test_parse_send_yumail_carrier() {
        let order = SurugayaSendParser.parse(sample_send_yumail()).unwrap();
        let di = order.delivery_info.as_ref().unwrap();
        assert_eq!(di.carrier, "ゆうメール（日本郵便）");
    }

    #[test]
    fn test_parse_send_yumail_tracking_number_empty() {
        // 追跡番号なしは空文字列で登録される
        let order = SurugayaSendParser.parse(sample_send_yumail()).unwrap();
        let di = order.delivery_info.as_ref().unwrap();
        assert!(di.tracking_number.is_empty());
    }

    #[test]
    fn test_parse_send_yumail_no_carrier_url() {
        // 追跡URLは存在しない
        let order = SurugayaSendParser.parse(sample_send_yumail()).unwrap();
        let di = order.delivery_info.as_ref().unwrap();
        assert!(di.carrier_url.is_none());
    }

    #[test]
    fn test_parse_send_yumail_amounts() {
        let order = SurugayaSendParser.parse(sample_send_yumail()).unwrap();
        assert_eq!(order.subtotal, Some(5100));
        assert_eq!(order.shipping_fee, Some(0));
        assert_eq!(order.total_amount, Some(5100));
    }

    #[test]
    fn test_parse_send_yumail_delivery_date() {
        let order = SurugayaSendParser.parse(sample_send_yumail()).unwrap();
        let di = order.delivery_info.as_ref().unwrap();
        assert_eq!(di.delivery_date.as_deref(), Some("2025-01-09 00:00:00"));
    }

    // ─── エラーケース ───

    #[test]
    fn test_parse_send_no_order_number_returns_error() {
        let body =
            "お問い合わせ番号　　：764336939516\nお届け方法　　　　　：ゆうパック（日本郵便)";
        assert!(SurugayaSendParser.parse(body).is_err());
    }

    #[test]
    fn test_parse_send_no_tracking_delivery_info_delivered() {
        // 追跡番号なし・配送会社あり → delivered で delivery_info が生成される
        let body = "（取引番号：S2204166697）\nお届け方法　　　　　：ゆうメール（日本郵便)";
        let order = SurugayaSendParser.parse(body).unwrap();
        let di = order.delivery_info.as_ref().unwrap();
        assert_eq!(di.delivery_status.as_deref(), Some("delivered"));
        assert!(di.tracking_number.is_empty());
    }

    #[test]
    fn test_parse_send_no_carrier_no_delivery_info() {
        // 配送会社なし（追跡あり）は delivery_info が None
        let body = "（取引番号：S2204166697）\nお問い合わせ番号　　：764336939516";
        let order = SurugayaSendParser.parse(body).unwrap();
        assert!(order.delivery_info.is_none());
    }
}
