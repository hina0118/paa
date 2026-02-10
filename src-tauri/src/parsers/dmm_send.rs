//! DMM通販「ご注文商品を発送いたしました」メール用パーサー
//!
//! 送信元: info@mail.dmm.com
//! 件名: DMM通販：ご注文商品を発送いたしました
//!
//! このメールは発送完了を通知し、配送業者とお問い合わせ番号（追跡番号）を含みます。
//! 最終的な発送状態を deliveries テーブルに反映するため、DeliveryInfo を中心に抽出します。
//! HTML メールが多いため、本文に `<html>` が含まれる場合は HTML からテキストを抽出してからパースします。

use super::{DeliveryAddress, DeliveryInfo, EmailParser, OrderInfo, OrderItem};
use regex::Regex;
use scraper::Html;

/// DMM通販 発送完了メール用パーサー
pub struct DmmSendParser;

impl EmailParser for DmmSendParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        if email_body.contains("<html") {
            // HTML メールは dmm_confirm と同じロジックで商品＋金額をパースしつつ、
            // dmm_send 独自の配送情報も付与する。
            let document = Html::parse_document(email_body);

            // 確定メールと同じロジックで注文番号・商品・金額を取得
            let order_number = super::dmm_confirm::extract_order_number_from_html(&document)?;
            let delivery_address =
                super::dmm_confirm::extract_delivery_address_from_html(&document);
            let items = super::dmm_confirm::extract_items_from_html(&document)?;
            let (subtotal, shipping_fee, total_amount) =
                super::dmm_confirm::extract_amounts_from_html(&document);

            // 発送メール特有の配送業者・お問い合わせ番号をテキストから抽出
            // text() はテキストノード間に区切りを入れないため、\n で結合して改行を保持する
            let mut text = String::new();
            for t in document.root_element().text() {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(t);
            }
            let lines: Vec<&str> = text.lines().collect();
            let delivery_info = extract_delivery_info(&lines);

            Ok(OrderInfo {
                order_number,
                order_date: None,
                delivery_address,
                delivery_info,
                items,
                subtotal,
                shipping_fee,
                total_amount,
            })
        } else {
            // プレーンテキストのみの場合は、配送情報だけを抽出（商品・金額は空）
            let lines: Vec<&str> = email_body.lines().collect();

            let order_number = extract_order_number(&lines)?;
            let delivery_address = extract_delivery_address(&lines);
            let delivery_info = extract_delivery_info(&lines);

            Ok(OrderInfo {
                order_number,
                order_date: None,
                delivery_address,
                delivery_info,
                items: Vec::<OrderItem>::new(),
                subtotal: None,
                shipping_fee: None,
                total_amount: None,
            })
        }
    }
}

/// ご注文番号: KC-xxxx / BS-xxxx を抽出
fn extract_order_number(lines: &[&str]) -> Result<String, String> {
    // 大文字・小文字両対応、接頭辞必須
    let patterns = [
        Regex::new(r"ご注文番号\s*[：:]\s*([A-Za-z]{2}-\d+)"),
        Regex::new(r"注文番号\s*[：:]\s*([A-Za-z]{2}-\d+)"),
    ];

    for line in lines {
        let line = line.trim();
        for re in patterns.iter().flatten() {
            if let Some(cap) = re.captures(line) {
                if let Some(m) = cap.get(1) {
                    return Ok(m.as_str().to_string());
                }
            }
        }
    }

    Err("Order number with prefix (KC-, BS-, etc.) not found in send mail".to_string())
}

/// 受取人のお名前：○○ 様
fn extract_delivery_address(lines: &[&str]) -> Option<DeliveryAddress> {
    let re = Regex::new(r"受取人のお名前\s*[：:]\s*(.+)").ok()?;

    for line in lines {
        let line = line.trim();
        if let Some(cap) = re.captures(line) {
            if let Some(m) = cap.get(1) {
                let name = m.as_str().trim().trim_end_matches('様').trim().to_string();
                if !name.is_empty() {
                    return Some(DeliveryAddress {
                        name,
                        postal_code: None,
                        address: None,
                    });
                }
            }
        }
    }
    None
}

/// 配送業者とお問い合わせ番号（追跡番号）を抽出
///
/// 例:
/// - 配送業者：佐川急便
/// - お問い合わせ番号：364631890991
/// - お問い合わせ伝票番号：364629550353
fn extract_delivery_info(lines: &[&str]) -> Option<DeliveryInfo> {
    let carrier_re = Regex::new(r"配送業者\s*[：:]\s*(.+)").ok()?;
    let tracking_re =
        Regex::new(r"(お問い合わせ伝票番号|お問い合わせ番号|お問合せ番号)\s*[：:]\s*([\d\-]+)")
            .ok()?;

    let mut carrier: Option<String> = None;
    let mut tracking: Option<String> = None;

    for line in lines {
        let line = line.trim();

        if carrier.is_none() {
            if let Some(cap) = carrier_re.captures(line) {
                if let Some(m) = cap.get(1) {
                    let value = m.as_str().trim();
                    if !value.is_empty() {
                        carrier = Some(value.to_string());
                    }
                }
            }
        }

        if tracking.is_none() {
            if let Some(cap) = tracking_re.captures(line) {
                if let Some(m) = cap.get(2) {
                    let value = m.as_str().trim();
                    if !value.is_empty() {
                        tracking = Some(value.to_string());
                    }
                }
            }
        }
    }

    if let (Some(carrier), Some(tracking_number)) = (carrier, tracking) {
        Some(DeliveryInfo {
            carrier,
            tracking_number,
            delivery_date: None,
            delivery_time: None,
            carrier_url: None,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dmm_send_basic() {
        let body = r#"テスト 太郎 様

DMM通販をご利用いただき、ありがとうございます。
ご注文商品を発送いたしました。

ご注文番号：BS-27322313
お問い合わせ番号：364631890991
配送業者：佐川急便

受取人のお名前：テスト 太郎 様
"#;
        let parser = DmmSendParser;
        let result = parser.parse(body);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let order = result.unwrap();
        assert_eq!(order.order_number, "BS-27322313");
        assert!(order.delivery_info.is_some());
        let info = order.delivery_info.unwrap();
        assert_eq!(info.carrier, "佐川急便");
        assert_eq!(info.tracking_number, "364631890991");
        assert!(order.delivery_address.is_some());
        assert_eq!(order.delivery_address.as_ref().unwrap().name, "テスト 太郎");
        // dmm_send は発送情報のみを扱うため、items は空のまま
        assert_eq!(order.items.len(), 0);
    }

    #[test]
    fn test_parse_dmm_send_html() {
        // HTML メールでは dmm_confirm と同じロジックで注文番号・商品・金額をパースし、
        // 配送業者・お問い合わせ番号もテキストから抽出する
        // 商品リンクは dmmref=gMono_Mail_Purchase を含む href が必要
        let body = r#"<html><body>
<table>
<tr><td>BS-27322313</td><td>発送元：千葉配送センター</td><td>発送：2024/06/15</td></tr>
</table>
<table>
<tr>
<td><a href="https://www.dmm.com/mono/detail/?dmmref=gMono_Mail_Purchase&i3_ref=mail_purchase&i3_ord=1">テスト商品A</a></td>
</tr>
</table>
<table>
<tr><td>商品小計</td><td>1,000円</td></tr>
<tr><td>送料</td><td>550円</td></tr>
<tr><td>合計</td><td>1,550円</td></tr>
</table>
<p>配送業者：佐川急便</p>
<p>お問い合わせ番号：364631890991</p>
</body></html>"#;
        let parser = DmmSendParser;
        let result = parser.parse(body);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let order = result.unwrap();
        assert_eq!(order.order_number, "BS-27322313");
        assert_eq!(order.items.len(), 1);
        assert!(order.items[0].name.contains("テスト商品A"));
        // HTML パスでは配送情報もテキストから抽出される
        assert!(
            order.delivery_info.is_some(),
            "delivery_info should be extracted from HTML text"
        );
        let info = order.delivery_info.unwrap();
        assert_eq!(info.carrier, "佐川急便");
        assert_eq!(info.tracking_number, "364631890991");
    }
}
