use super::{
    body_to_lines, extract_items, extract_order_date, extract_order_number, extract_shipping_fee,
    extract_total_amount,
};
use crate::parsers::{EmailParser, OrderInfo};

/// グッドスマイルカンパニー 注文確認メール用パーサー
///
/// 件名：`ご注文完了のお知らせ (ご注文番号_XXXX)`
/// 送信元：`shop@goodsmile.jp`（SendGrid 経由）
///
/// HTML / プレーンテキストどちらにも対応する。
/// HTML の場合は `<br>` を改行に変換後、タグを除去してから各フィールドを抽出する。
/// 注文日時は英語形式（`Feb 01, 2025 4:48:07 PM`）のため chrono でパースする。
pub struct GoodSmileConfirmParser;

impl EmailParser for GoodSmileConfirmParser {
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

    /// プレーンテキスト形式のサンプル（plain text パートの内容）
    fn sample_confirm_plain() -> &'static str {
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

    /// HTML 形式のサンプル（body_html パートの内容・実メールの構造に準拠）
    ///
    /// 実メールでは `配送料  ￥0`（スペース 2 つ）・`合計  ￥5,900`（スペース 2 つ）の
    /// フォーマットが使われるため、それを再現する。
    fn sample_confirm_html() -> &'static str {
        r#"<html><body>
※このメールはシステムより自動送信されています。<br><br>
山田 太郎様<br>
<br>
この度は、グッドスマイルカンパニー公式ショップをご利用頂き誠にありがとうございます。<br>
---------------------------------<br>
ご注文番号: CpBk4quaORPw<br>
ご注文日時: Feb 01, 2025 4:48:07 PM<br>
お支払方法:クレジットカード<br>
配送方法:　佐川急便_送料無料<br>
<br>
商品:<br>
MODEROID バーンドラゴン<br>
発売時期：2025/9<br>
数量：1<br>
小計：￥5,900<br>
<br>
     配送料  ￥0<br>
     クーポン割引額  ￥0<br>
 合計  ￥5,900<br>
</body></html>"#
    }

    // ─── プレーンテキストによるテスト ───

    #[test]
    fn test_parse_confirm_order_number() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_plain())
            .unwrap();
        assert_eq!(order.order_number, "CpBk4quaORPw");
    }

    #[test]
    fn test_parse_confirm_order_date() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_plain())
            .unwrap();
        // Feb 01, 2025 4:48:07 PM → 2025-02-01 16:48
        assert_eq!(order.order_date, Some("2025-02-01 16:48".to_string()));
    }

    #[test]
    fn test_parse_confirm_item_count() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_plain())
            .unwrap();
        assert_eq!(order.items.len(), 1);
    }

    #[test]
    fn test_parse_confirm_item_name() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_plain())
            .unwrap();
        assert_eq!(order.items[0].name, "MODEROID バーンドラゴン");
    }

    #[test]
    fn test_parse_confirm_item_quantity() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_plain())
            .unwrap();
        assert_eq!(order.items[0].quantity, 1);
    }

    #[test]
    fn test_parse_confirm_item_subtotal_and_unit_price() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_plain())
            .unwrap();
        assert_eq!(order.items[0].subtotal, 5900);
        assert_eq!(order.items[0].unit_price, 5900);
    }

    #[test]
    fn test_parse_confirm_amounts() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_plain())
            .unwrap();
        assert_eq!(order.subtotal, Some(5900));
        assert_eq!(order.shipping_fee, Some(0));
        assert_eq!(order.total_amount, Some(5900));
    }

    #[test]
    fn test_parse_confirm_no_delivery_info() {
        let order = GoodSmileConfirmParser
            .parse(sample_confirm_plain())
            .unwrap();
        assert!(order.delivery_info.is_none());
    }

    // ─── HTML によるテスト（実メールに近い形式） ───

    #[test]
    fn test_parse_confirm_html_order_number() {
        let order = GoodSmileConfirmParser.parse(sample_confirm_html()).unwrap();
        assert_eq!(order.order_number, "CpBk4quaORPw");
    }

    #[test]
    fn test_parse_confirm_html_order_date() {
        let order = GoodSmileConfirmParser.parse(sample_confirm_html()).unwrap();
        assert_eq!(order.order_date, Some("2025-02-01 16:48".to_string()));
    }

    #[test]
    fn test_parse_confirm_html_item_name_and_quantity() {
        let order = GoodSmileConfirmParser.parse(sample_confirm_html()).unwrap();
        assert_eq!(order.items.len(), 1);
        assert_eq!(order.items[0].name, "MODEROID バーンドラゴン");
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[0].subtotal, 5900);
    }

    /// HTML では `配送料  ￥0`（スペース 2 つ）が使われるため、正しく抽出できることを確認する
    #[test]
    fn test_parse_confirm_html_amounts_with_double_space() {
        let order = GoodSmileConfirmParser.parse(sample_confirm_html()).unwrap();
        assert_eq!(order.subtotal, Some(5900));
        assert_eq!(order.shipping_fee, Some(0));
        assert_eq!(order.total_amount, Some(5900));
    }

    // ─── エラーケース ───

    #[test]
    fn test_parse_confirm_no_order_number_returns_error() {
        let result =
            GoodSmileConfirmParser.parse("商品:テスト商品\n数量：1\n小計：￥1,000\n合計 ￥1,000");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_confirm_no_items_returns_error() {
        let result =
            GoodSmileConfirmParser.parse("ご注文番号: ABC123\nご注文日時: Feb 01, 2025 4:48:07 PM");
        assert!(result.is_err());
    }
}
