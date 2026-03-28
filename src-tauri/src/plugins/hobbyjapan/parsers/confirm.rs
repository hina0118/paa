use super::{
    body_to_lines, extract_items, extract_order_date, extract_order_number, extract_shipping_fee,
    extract_subtotal, extract_total_amount,
};
use crate::parsers::{EmailParser, OrderInfo};

/// HJ OnlineShop 注文確認メール用パーサー
///
/// 件名：`【HJ OnlineShop】ご注文を受け付けました`
/// 送信元：`shop@hobbyjapan.co.jp`
///
/// プレーンテキスト形式（ISO-2022-JP / 7bit）。
/// 注文日はメール本文の `【ご注文日】2026年03月21日` から取得する。
pub struct HjConfirmParser;

impl EmailParser for HjConfirmParser {
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
        "原田\u{3000}裕基\u{3000}様\n\
         \n\
         ご利用ありがとうございます。\n\
         ホビージャパンオンラインショップです。\n\
         \n\
         下記のとおりご注文を承りました。\n\
         内容に間違いがないか必ずご確認ください。\n\
         \n\
         --------------------------------------------------\n\
         【オーダーID】HJ20260321_051302_88\n\
         【ご注文日】2026年03月21日\n\
         【ご注文者】原田\u{3000}裕基\u{3000}様\n\
         \n\
         【お届け先】\n\
         〒8120044\n\
         福岡県福岡市博多区寿町1-7-1リアン・シェルブルー東公園\u{3000}505号\n\
         TEL09016717298\n\
         原田\u{3000}裕基\u{3000}様\n\
         \n\
         【ご注文品】\n\
         \u{3000}1.GALHolic深青\n\
         \u{3000}\u{3000}価格：\u{FFE5}7,880 x 数量：1 = 合計：\u{FFE5}7,880\n\
         \n\
         【お買上げ金額】\n\
         \u{3000}商品金額合計：\u{FFE5}7,880\n\
         \u{3000}送料：\u{FFE5}700\n\
         \u{3000}手数料：\u{FFE5}0\n\
         \u{3000}注文金額合計：\u{FFE5}8,580\n\
         \u{3000}\n\
         【お支払い方法】クレジットカード決済\u{3000}お支払い回数：一括払い\n\
         \u{3000}AEONレジ番号：00003476068231\n\
         \n\
         --------------------------------------------------\n"
    }

    #[test]
    fn test_parse_order_number() {
        let order = HjConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.order_number, "HJ20260321_051302_88");
    }

    #[test]
    fn test_parse_order_date() {
        let order = HjConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.order_date, Some("2026-03-21".to_string()));
    }

    #[test]
    fn test_parse_item_count() {
        let order = HjConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items.len(), 1);
    }

    #[test]
    fn test_parse_item_name() {
        let order = HjConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].name, "GALHolic深青");
    }

    #[test]
    fn test_parse_item_price() {
        let order = HjConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].unit_price, 7880);
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[0].subtotal, 7880);
    }

    #[test]
    fn test_parse_amounts() {
        let order = HjConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.subtotal, Some(7880));
        assert_eq!(order.shipping_fee, Some(700));
        assert_eq!(order.total_amount, Some(8580));
    }

    #[test]
    fn test_parse_no_delivery_info() {
        let order = HjConfirmParser.parse(sample_confirm()).unwrap();
        assert!(order.delivery_info.is_none());
    }

    #[test]
    fn test_parse_no_order_number_returns_error() {
        let body = "【ご注文日】2026年03月21日\n【ご注文品】\n\u{3000}1.商品A\n\u{3000}\u{3000}価格：\u{FFE5}1,000 x 数量：1 = 合計：\u{FFE5}1,000\n【お買上げ金額】\n";
        assert!(HjConfirmParser.parse(body).is_err());
    }

    #[test]
    fn test_parse_no_items_returns_error() {
        let body = "【オーダーID】HJ20260321_051302_88\n【ご注文品】\n【お買上げ金額】\n";
        assert!(HjConfirmParser.parse(body).is_err());
    }
}
