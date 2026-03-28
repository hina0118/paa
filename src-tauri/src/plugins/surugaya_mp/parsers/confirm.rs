use super::{body_to_lines, extract_confirm_order_number, extract_mypage_url};
use crate::parsers::{EmailParser, OrderInfo};

/// 駿河屋マーケットプレイス 注文受付メール用パーサー
///
/// 件名：`ご注文受付のお知らせ` を含む
/// 送信元：`order@suruga-ya.jp`
///
/// 取引番号は `お客様のご注文番号 [ M2502021943 ] になります。` 形式（`M` + 10桁）。
/// 注文日は本文に含まれないため、`dispatch()` 側で `apply_internal_date()` を使用する。
/// 商品明細はメール本文に含まれないため `items` は空。マイページHTMLで補完する。
pub struct SurugayaMpConfirmParser;

impl EmailParser for SurugayaMpConfirmParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let lines = body_to_lines(email_body);
        let lines_ref: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();

        let order_number = extract_confirm_order_number(&lines_ref)
            .ok_or_else(|| "Order number not found".to_string())?;

        Ok(OrderInfo {
            order_number,
            order_date: None, // internal_date で補完
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
///
/// `dispatch()` 側で `htmls` テーブルへの URL 登録に使用するため `pub` にする。
pub fn parse_mypage_url(email_body: &str) -> Option<String> {
    extract_mypage_url(email_body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_confirm() -> &'static str {
        // Gmail API が UTF-8 デコード済みのメール本文（HTMLタグあり）
        r#"<meta http-equiv="Content-Type" content="text/html; charset=ISO-2022-JP">
本メールは、送信専用アドレスとなります。<br />
返信にてのお問い合わせにつきましては、ご返信いたしかねます。<br />
-----------------------------------------------------------------------------------------<br />
<br />
山田太郎 様<br />
<br />
この度は駿河屋マーケットプレイスをご利用頂きまして、誠にありがとうございます。<br />
このメールはシステムにより自動送信されています。<br />
<br />
ご注文の受付は毎朝8時となっておりますので、それ以降のご注文に関しましては、全て翌日処理となります。<br />
<br />
お客様のご注文番号 [ M2502021943 ] になります。<br />
ご注文詳細はマイページをご確認ください。<br />
https://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=M2502021943<br />
<br />
またのご利用をお待ちしております。<br />
(株)エーツー 「駿河屋」<br />
http://www.suruga-ya.jp/
"#
    }

    #[test]
    fn test_parse_confirm_order_number() {
        let order = SurugayaMpConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.order_number, "M2502021943");
    }

    #[test]
    fn test_parse_confirm_no_order_date() {
        let order = SurugayaMpConfirmParser.parse(sample_confirm()).unwrap();
        assert!(order.order_date.is_none());
    }

    #[test]
    fn test_parse_confirm_items_empty() {
        // メール本文に商品明細なし → 空
        let order = SurugayaMpConfirmParser.parse(sample_confirm()).unwrap();
        assert!(order.items.is_empty());
    }

    #[test]
    fn test_parse_confirm_no_amounts() {
        let order = SurugayaMpConfirmParser.parse(sample_confirm()).unwrap();
        assert!(order.subtotal.is_none());
        assert!(order.shipping_fee.is_none());
        assert!(order.total_amount.is_none());
    }

    #[test]
    fn test_parse_confirm_mypage_url() {
        let url = parse_mypage_url(sample_confirm()).unwrap();
        assert_eq!(
            url,
            "https://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=M2502021943"
        );
    }

    #[test]
    fn test_parse_confirm_no_order_number_returns_error() {
        let body = "<p>ご注文ありがとうございます。</p>";
        assert!(SurugayaMpConfirmParser.parse(body).is_err());
    }
}
