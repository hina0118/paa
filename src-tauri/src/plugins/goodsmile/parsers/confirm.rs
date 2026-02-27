use super::{
    extract_items, extract_order_date, extract_order_number, extract_shipping_fee,
    extract_total_amount,
};
use crate::parsers::{EmailParser, OrderInfo};

/// グッドスマイルカンパニー 注文確認メール用パーサー
///
/// 件名：`ご注文完了のお知らせ (ご注文番号_XXXX)`
/// 送信元：`shop@goodsmile.jp`（SendGrid 経由）
///
/// テキストパートから注文番号・日時・商品情報・金額を抽出する。
/// 注文日時は英語形式（`Feb 01, 2025 4:48:07 PM`）のため chrono でパースする。
pub struct GoodSmileConfirmParser;

impl EmailParser for GoodSmileConfirmParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let lines: Vec<&str> = email_body.lines().collect();

        let order_number = extract_order_number(&lines)
            .ok_or_else(|| "Order number not found".to_string())?;

        let order_date = extract_order_date(&lines);

        let items = extract_items(&lines);
        if items.is_empty() {
            return Err("No items found".to_string());
        }

        // 商品ごとの小計の合計を注文小計とする
        let subtotal: i64 = items.iter().map(|i| i.subtotal).sum();
        let subtotal = if subtotal > 0 { Some(subtotal) } else { None };

        let shipping_fee = extract_shipping_fee(&lines);
        let total_amount = extract_total_amount(&lines);

        Ok(OrderInfo {
            order_number,
            order_date,
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

    fn sample_confirm_email() -> &'static str {
        r#"※このメールはシステムより自動送信されています。
山田 太郎様
この度は、グッドスマイルカンパニー公式ショップをご利用頂き誠にありがとうございます。
以下の商品について、ご注文を受付いたしました。
このメールは商品のお届けまで大切に保管してください。
＜ご注文内容＞
マイアカウントからもご確認いただけます。
---------------------------------
ご注文番号: CpBk4quaORPw
ご注文日時: Feb 01, 2025 4:48:07 PM
お支払方法:クレジットカード
※コンビニ決済でご注文のお客様は、本メールの送信日を含む７日以内にお支払いをお願いいたします。
メールアドレス: test.user@example.com
発送先情報:
1000001
福岡県
千代田区
千代1-7-1
テストマンション505号
山田 太郎 様
電話: 09000000000
配送方法:　佐川急便_送料無料
商品:MODEROID バーンドラゴン
発売時期：2025/9
数量：1
小計：￥5,900
配送料 ￥0
クーポン割引額 ￥0
合計 ￥5,900
◆商品の発送について
---------------------------------
ご予約商品の場合、出荷日時が確定いたしましたらご登録のメールアドレス宛にご連絡いたします。
"#
    }

    #[test]
    fn test_parse_confirm_order_number() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_email())
            .unwrap();
        assert_eq!(order.order_number, "CpBk4quaORPw");
    }

    #[test]
    fn test_parse_confirm_order_date() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_email())
            .unwrap();
        // Feb 01, 2025 4:48:07 PM → 2025-02-01 16:48
        assert_eq!(order.order_date, Some("2025-02-01 16:48".to_string()));
    }

    #[test]
    fn test_parse_confirm_item_count() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_email())
            .unwrap();
        assert_eq!(order.items.len(), 1);
    }

    #[test]
    fn test_parse_confirm_item_name() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_email())
            .unwrap();
        assert_eq!(order.items[0].name, "MODEROID バーンドラゴン");
    }

    #[test]
    fn test_parse_confirm_item_quantity() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_email())
            .unwrap();
        assert_eq!(order.items[0].quantity, 1);
    }

    #[test]
    fn test_parse_confirm_item_subtotal_and_unit_price() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_email())
            .unwrap();
        assert_eq!(order.items[0].subtotal, 5900);
        assert_eq!(order.items[0].unit_price, 5900);
    }

    #[test]
    fn test_parse_confirm_amounts() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_email())
            .unwrap();
        assert_eq!(order.subtotal, Some(5900));
        assert_eq!(order.shipping_fee, Some(0));
        assert_eq!(order.total_amount, Some(5900));
    }

    #[test]
    fn test_parse_confirm_no_delivery_info() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_email())
            .unwrap();
        assert!(order.delivery_info.is_none());
    }

    #[test]
    fn test_parse_confirm_no_order_number_returns_error() {
        let result = GoodSmileConfirmParser.parse("商品:テスト商品\n数量：1\n小計：￥1,000\n合計 ￥1,000");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_confirm_no_items_returns_error() {
        let result = GoodSmileConfirmParser.parse("ご注文番号: ABC123\nご注文日時: Feb 01, 2025 4:48:07 PM");
        assert!(result.is_err());
    }
}
