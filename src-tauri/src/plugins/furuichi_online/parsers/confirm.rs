use super::{
    body_to_lines, extract_items, extract_order_date, extract_order_number, extract_shipping_fee,
    extract_subtotal, extract_total_amount,
};
use crate::parsers::{EmailParser, OrderInfo};

/// ふるいちオンライン 注文確認メール用パーサー
///
/// 件名：`【ふるいちオンライン】 ご注文ありがとうございます`
/// 送信元：`info@furu1.online`
///
/// プレーンテキスト形式（quoted-printable UTF-8）。
/// 注文日はメール本文に含まれるため `order_date` を直接設定する。
pub struct FuruichiConfirmParser;

impl EmailParser for FuruichiConfirmParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let body_lines = body_to_lines(email_body);
        let lines: Vec<&str> = body_lines.iter().map(|s| s.as_str()).collect();

        let order_number =
            extract_order_number(&lines).ok_or_else(|| "Order number not found".to_string())?;

        let order_date = extract_order_date(&lines);

        let items = extract_items(&lines);
        if items.is_empty() {
            return Err("No items found".to_string());
        }

        let subtotal = extract_subtotal(&lines);
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

    fn sample_confirm() -> &'static str {
        r#"---------------------------------------------
このメールはお客様の注文に関する大切なメールです。
届くまで保存してください。
本メールは自動配信メールです。
---------------------------------------------


山田太郎様

この度は、ふるいちオンラインをご利用いただきありがとうございます。

下記内容にてご注文を承りましたのでお知らせいたします。
お客様のご注文内容をご確認ください。

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
    fn test_parse_confirm_order_number() {
        let order = FuruichiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.order_number, "100409780");
    }

    #[test]
    fn test_parse_confirm_order_date() {
        let order = FuruichiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.order_date, Some("2026-03-03 22:25:08".to_string()));
    }

    #[test]
    fn test_parse_confirm_item_count() {
        let order = FuruichiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_confirm_item_names() {
        let order = FuruichiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].name, "03ゼウスⅠ　カルノージャート");
        assert_eq!(order.items[1].name, "030カルノージャート　エクサ");
    }

    #[test]
    fn test_parse_confirm_item_quantities() {
        let order = FuruichiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[1].quantity, 1);
    }

    #[test]
    fn test_parse_confirm_item_prices_are_zero() {
        // ふるいちオンラインは商品行に単価を含まないため 0 とする
        let order = FuruichiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].unit_price, 0);
        assert_eq!(order.items[0].subtotal, 0);
    }

    #[test]
    fn test_parse_confirm_amounts() {
        let order = FuruichiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.subtotal, Some(6158));
        assert_eq!(order.shipping_fee, Some(0));
        assert_eq!(order.total_amount, Some(6158));
    }

    #[test]
    fn test_parse_confirm_no_delivery_info() {
        let order = FuruichiConfirmParser.parse(sample_confirm()).unwrap();
        assert!(order.delivery_info.is_none());
    }

    #[test]
    fn test_parse_confirm_no_order_number_returns_error() {
        let body = "ご注文日：2026-03-03 22:25:08\nご注文商品：\n商品A:1個\n-----";
        assert!(FuruichiConfirmParser.parse(body).is_err());
    }

    #[test]
    fn test_parse_confirm_no_items_returns_error() {
        let body = "ご注文番号：100409780\nご注文日：2026-03-03 22:25:08\nご注文商品：\n-----";
        assert!(FuruichiConfirmParser.parse(body).is_err());
    }
}
