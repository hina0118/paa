use super::{
    body_to_lines, extract_items, extract_order_number, extract_shipping_fee, extract_subtotal,
    extract_total_amount,
};
use crate::parsers::{EmailParser, OrderInfo};

/// アニメイト通販 注文確認メール用パーサー
///
/// 件名：`【アニメイト通販】ご注文の確認`
/// 送信元：`info@animate-onlineshop.jp`
///
/// プレーンテキスト形式（ISO-2022-JP → UTF-8 デコード済みを想定）。
/// 注文日はメール本文に含まれないため、`dispatch()` 内で `apply_internal_date` によって補完する。
pub struct AnimateConfirmParser;

impl EmailParser for AnimateConfirmParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let body_lines = body_to_lines(email_body);
        let lines: Vec<&str> = body_lines.iter().map(|s| s.as_str()).collect();

        let order_number =
            extract_order_number(&lines).ok_or_else(|| "Order number not found".to_string())?;

        let items = extract_items(&lines);
        if items.is_empty() {
            return Err("No items found".to_string());
        }

        let subtotal = extract_subtotal(&lines);
        let shipping_fee = extract_shipping_fee(&lines);
        let total_amount = extract_total_amount(&lines);

        Ok(OrderInfo {
            order_number,
            order_date: None, // dispatch() 内で apply_internal_date が補完する
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
        r#"この度はアニメイト通販をご利用頂きまして誠にありがとうございます。

ご注文いただきありがとうございます。

ご注文内容をご確認ください。
本メールは到着まで大切に保管して下さい。

●ご注文番号
28928446
ご注文番号は大切に保管してください。

●お客さま情報
氏名:テスト 太郎
E-Mail:test@example.com

●配送先
氏名：テスト 太郎
住所：〒000-0000 東京都テスト市テスト町1-1
TEL：000-0000-0000

●ご注文内容
商品名: 【サウンドトラック】テスト商品A サウンドトラック4 Version.489【C99】
数量:1 個
単価:3,000円(税込)
発売日:2022年02月 中 発売予定
商品合計額:3,000円(税込)
=============
商品名: 【グッズ-セットもの】テスト商品B コンプリートBOX【C99】
数量:1 個
単価:5,000円(税込)
発売日:2022年02月 中 発売予定
商品合計額:5,000円(税込)
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
    fn test_parse_confirm_order_number() {
        let order = AnimateConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.order_number, "28928446");
    }

    #[test]
    fn test_parse_confirm_order_date_is_none() {
        // 注文日はメール本文に含まれない → dispatch で補完
        let order = AnimateConfirmParser.parse(sample_confirm()).unwrap();
        assert!(order.order_date.is_none());
    }

    #[test]
    fn test_parse_confirm_item_count() {
        let order = AnimateConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_confirm_item_names() {
        let order = AnimateConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(
            order.items[0].name,
            "【サウンドトラック】テスト商品A サウンドトラック4 Version.489【C99】"
        );
        assert_eq!(
            order.items[1].name,
            "【グッズ-セットもの】テスト商品B コンプリートBOX【C99】"
        );
    }

    #[test]
    fn test_parse_confirm_item_quantities() {
        let order = AnimateConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[1].quantity, 1);
    }

    #[test]
    fn test_parse_confirm_item_prices() {
        let order = AnimateConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].unit_price, 3000);
        assert_eq!(order.items[0].subtotal, 3000);
        assert_eq!(order.items[1].unit_price, 5000);
        assert_eq!(order.items[1].subtotal, 5000);
    }

    #[test]
    fn test_parse_confirm_amounts() {
        let order = AnimateConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.subtotal, Some(8000));
        assert_eq!(order.shipping_fee, Some(594));
        assert_eq!(order.total_amount, Some(8594));
    }

    #[test]
    fn test_parse_confirm_no_delivery_info() {
        let order = AnimateConfirmParser.parse(sample_confirm()).unwrap();
        assert!(order.delivery_info.is_none());
    }

    #[test]
    fn test_parse_confirm_no_order_number_returns_error() {
        let result = AnimateConfirmParser.parse("ご注文ありがとうございます。\n●ご注文内容\n商品名: テスト商品\n数量:1 個\n単価:1,000円(税込)\n商品合計額:1,000円(税込)\n支払方法：クレジット");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_confirm_no_items_returns_error() {
        let result =
            AnimateConfirmParser.parse("●ご注文番号\n28928446\n●ご注文内容\n支払方法：クレジット");
        assert!(result.is_err());
    }
}
