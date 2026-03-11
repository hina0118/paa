use super::{body_to_lines, extract_mypage_url, extract_send_order_number};
use crate::parsers::{EmailParser, OrderInfo};

/// 駿河屋マーケットプレイス 商品発送メール用パーサー
///
/// 件名：`商品発送のお知らせ` を含む
/// 送信元：`reference@suruga-ya.jp`
///
/// 取引番号は `取引番号：M2603039345` 形式（`M` + 10桁）。
/// 商品明細・金額・追跡情報はメール本文に含まれないため全て空。マイページHTMLで補完する。
pub struct SurugayaMpSendParser;

impl EmailParser for SurugayaMpSendParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let lines = body_to_lines(email_body);
        let lines_ref: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();

        let order_number = extract_send_order_number(&lines_ref)
            .ok_or_else(|| "Order number not found".to_string())?;

        Ok(OrderInfo {
            order_number,
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![], // マイページHTMLで補完
            subtotal: None,
            shipping_fee: None,
            total_amount: None,
        })
    }
}

/// メール本文からマイページURLを抽出するユーティリティ
pub fn parse_mypage_url(email_body: &str) -> Option<String> {
    extract_mypage_url(email_body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_send() -> &'static str {
        r#"<meta http-equiv="Content-Type" content="text/html; charset=ISO-2022-JP">
<p>---------------------------------------------------<br/>
本メールは、送信専用アドレスとなります。<br/>
返信にてのお問い合わせにつきましては、ご返信いたしかねます。<br/>
---------------------------------------------------</p>

<p>取引番号：M2603039345<br/>
山田太郎様</p>

<p>この度は駿河屋マーケットプレイスをご利用頂きまして、誠にありがとうございます。</p>

<p>商品を発送致しました。<br/>
詳細はマイページをご確認ください。<br/>
▼こちらより確認をお願い致します。<br/>
<a href="https://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=M2603039345">https://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=M2603039345</a></p>

<p>またのご利用をお待ちしております。<br/>
「駿河屋」<br/>
http://www.suruga-ya.jp/</p>"#
    }

    #[test]
    fn test_parse_send_order_number() {
        let order = SurugayaMpSendParser.parse(sample_send()).unwrap();
        assert_eq!(order.order_number, "M2603039345");
    }

    #[test]
    fn test_parse_send_items_empty() {
        let order = SurugayaMpSendParser.parse(sample_send()).unwrap();
        assert!(order.items.is_empty());
    }

    #[test]
    fn test_parse_send_no_amounts() {
        let order = SurugayaMpSendParser.parse(sample_send()).unwrap();
        assert!(order.subtotal.is_none());
        assert!(order.shipping_fee.is_none());
        assert!(order.total_amount.is_none());
    }

    #[test]
    fn test_parse_send_no_delivery_info() {
        let order = SurugayaMpSendParser.parse(sample_send()).unwrap();
        assert!(order.delivery_info.is_none());
    }

    #[test]
    fn test_parse_send_mypage_url_from_href() {
        let url = parse_mypage_url(sample_send()).unwrap();
        assert_eq!(
            url,
            "https://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=M2603039345"
        );
    }

    #[test]
    fn test_parse_send_no_order_number_returns_error() {
        let body = "<p>商品を発送致しました。</p>";
        assert!(SurugayaMpSendParser.parse(body).is_err());
    }
}
