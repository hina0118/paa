use super::{body_to_lines, extract_items, extract_order_number, extract_tracking_number};
use crate::parsers::{DeliveryInfo, EmailParser, OrderInfo};

/// ゆうパック追跡サービスの URL
const JAPANPOST_TRACKING_URL: &str =
    "https://trackings.post.japanpost.jp/services/srv/search/input";

/// アニメイト通販 出荷完了メール用パーサー
///
/// 件名：`【アニメイト通販】出荷完了のお知らせ`
/// 送信元：`info@animate-onlineshop.jp`
///
/// プレーンテキスト形式（ISO-2022-JP → UTF-8 デコード済みを想定）。
/// 配送会社はゆうパック固定（追跡 URL `trackings.post.japanpost.jp` により判定）。
/// 金額情報は発送通知メールにも含まれるが、`save_order_in_tx` の既存スキップロジックにより
/// confirm で登録済みの価格を上書きしない。
pub struct AnimateSendParser;

impl EmailParser for AnimateSendParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let body_lines = body_to_lines(email_body);
        let lines: Vec<&str> = body_lines.iter().map(|s| s.as_str()).collect();

        let order_number =
            extract_order_number(&lines).ok_or_else(|| "Order number not found".to_string())?;

        let tracking_number = extract_tracking_number(&lines)
            .ok_or_else(|| "Tracking number not found".to_string())?;

        let items = extract_items(&lines);
        if items.is_empty() {
            return Err("No items found".to_string());
        }

        let delivery_info = DeliveryInfo {
            carrier: "ゆうパック".to_string(),
            tracking_number,
            delivery_date: None,
            delivery_time: None,
            carrier_url: Some(JAPANPOST_TRACKING_URL.to_string()),
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
        r#"この度はアニメイト通販をご利用頂きまして誠にありがとうございます。

お客様からご注文いただいた商品を、本日発送させていただきました。
商品の到着まで今しばらくお待ちいただけますようお願いいたします。

●ご注文番号
28928446

●送り状番号
217565803081

配送情報の確認は、日本郵便のサイトよりご確認いただけます。
ゆうパック
　https://trackings.post.japanpost.jp/services/srv/search/input

●配送先
氏名：テスト 太郎
住所：〒000-0000 東京都テスト市テスト町1-1
TEL：000-0000-0000

●ご注文内容
商品名: 【グッズ-セットもの】テスト商品B コンプリートBOX【C99】
数量:1 個
単価:5,000円(税込)
発売日:2022年02月 中 発売予定
商品合計額:5,000円(税込)
=============
商品名: 【サウンドトラック】テスト商品A サウンドトラック4 Version.489【C99】
数量:1 個
単価:3,000円(税込)
発売日:2022年02月 中 発売予定
商品合計額:3,000円(税込)
支払方法：クレジット

●合計
商品合計:8,000円(税込)
送料:594円(税込)
手数料:0円(税込)
小計:8,594円(税込)
ポイント利用:0
クーポン利用:0円
合計額:8,594円(税込)

●配送方法
宅配便
"#
    }

    #[test]
    fn test_parse_send_order_number() {
        let order = AnimateSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.order_number, "28928446");
    }

    #[test]
    fn test_parse_send_tracking_number() {
        let order = AnimateSendParser.parse(sample_send()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.tracking_number, "217565803081");
    }

    #[test]
    fn test_parse_send_carrier() {
        let order = AnimateSendParser.parse(sample_send()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.carrier, "ゆうパック");
    }

    #[test]
    fn test_parse_send_carrier_url() {
        let order = AnimateSendParser.parse(sample_send()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(
            delivery.carrier_url,
            Some(
                "https://trackings.post.japanpost.jp/services/srv/search/input".to_string()
            )
        );
    }

    #[test]
    fn test_parse_send_item_count() {
        let order = AnimateSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_send_item_names() {
        let order = AnimateSendParser.parse(sample_send()).unwrap();
        assert_eq!(
            order.items[0].name,
            "【グッズ-セットもの】テスト商品B コンプリートBOX【C99】"
        );
        assert_eq!(
            order.items[1].name,
            "【サウンドトラック】テスト商品A サウンドトラック4 Version.489【C99】"
        );
    }

    #[test]
    fn test_parse_send_no_amounts() {
        // send パーサーは金額を返さない（confirm の価格を保持するため）
        let order = AnimateSendParser.parse(sample_send()).unwrap();
        assert!(order.subtotal.is_none());
        assert!(order.shipping_fee.is_none());
        assert!(order.total_amount.is_none());
    }

    #[test]
    fn test_parse_send_no_order_date() {
        let order = AnimateSendParser.parse(sample_send()).unwrap();
        assert!(order.order_date.is_none());
    }

    #[test]
    fn test_parse_send_no_order_number_returns_error() {
        let result = AnimateSendParser.parse("●送り状番号\n217565803081\n●ご注文内容\n商品名: テスト商品\n数量:1 個\n単価:1,000円(税込)\n商品合計額:1,000円(税込)\n支払方法：クレジット");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_send_no_tracking_number_returns_error() {
        let result = AnimateSendParser.parse("●ご注文番号\n28928446\n●ご注文内容\n商品名: テスト商品\n数量:1 個\n単価:1,000円(税込)\n商品合計額:1,000円(税込)\n支払方法：クレジット");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_send_no_items_returns_error() {
        let result =
            AnimateSendParser.parse("●ご注文番号\n28928446\n●送り状番号\n217565803081\n●ご注文内容\n支払方法：クレジット");
        assert!(result.is_err());
    }
}
