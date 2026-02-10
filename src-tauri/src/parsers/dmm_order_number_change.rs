//! DMM通販「配送センター変更に伴うご注文番号変更のお知らせ」メール用パーサー
//!
//! 送信元: info@mail.dmm.com
//! 件名: DMM通販：配送センター変更に伴うご注文番号変更のお知らせ
//!
//! 旧注文番号・新注文番号を抽出する。

use crate::parsers::order_number_change_info::OrderNumberChangeInfo;
use regex::Regex;

/// DMM通販 配送センター変更に伴う注文番号変更メール用パーサー
pub struct DmmOrderNumberChangeParser;

impl DmmOrderNumberChangeParser {
    /// メール本文から注文番号変更情報を抽出する
    pub fn parse_order_number_change(
        &self,
        email_body: &str,
    ) -> Result<OrderNumberChangeInfo, String> {
        let lines: Vec<&str> = email_body.lines().collect();

        let (old_num, new_num) = extract_order_numbers(&lines)?;

        Ok(OrderNumberChangeInfo {
            old_order_number: old_num.trim().to_string(),
            new_order_number: new_num.trim().to_string(),
        })
    }
}

/// ご注文番号：旧番号　→　新番号 形式を抽出
fn extract_order_numbers(lines: &[&str]) -> Result<(String, String), String> {
    // ご注文番号：kc-26407532　→　bs-26888944 または ご注文番号：kc-25889483　⇒　bs-26799949 形式
    let re = Regex::new(r"ご注文番号\s*[：:]\s*([A-Za-z]{2}-\d+)\s*[→⇒　]\s*([A-Za-z]{2}-\d+)")
        .map_err(|e| format!("Regex error: {e}"))?;

    for line in lines {
        if let Some(captures) = re.captures(line) {
            if let (Some(old_m), Some(new_m)) = (captures.get(1), captures.get(2)) {
                return Ok((old_m.as_str().to_string(), new_m.as_str().to_string()));
            }
        }
    }

    Err("Order number change not found".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dmm_order_number_change() {
        let email = r#"DMM通販をご利用いただき、ありがとうございます。

配送センター変更に伴い、下記の注文番号が変更となりましたのでお知らせいたします。

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
ご注文番号：kc-26407532　→　bs-26888944

対象商品：
EXPO2025 ENTRY GRADE 1/144 RX-78F00/E ガンダム
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"#;
        let parser = DmmOrderNumberChangeParser;
        let result = parser.parse_order_number_change(email);

        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let info = result.unwrap();
        // メールの表記をそのまま使用（将来の注文詳細ページURL対応のため）
        assert_eq!(info.old_order_number, "kc-26407532");
        assert_eq!(info.new_order_number, "bs-26888944");
    }

    #[test]
    fn test_parse_dmm_order_number_change_double_arrow() {
        let email = r#"ご注文番号：kc-25889483　⇒　bs-26799949

対象商品 ：
30MS SIS-N00 ソウレイ［カラーB］"#;
        let parser = DmmOrderNumberChangeParser;
        let result = parser.parse_order_number_change(email);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.old_order_number, "kc-25889483");
        assert_eq!(info.new_order_number, "bs-26799949");
    }

    #[test]
    fn test_parse_dmm_order_number_change_uppercase() {
        let email = r#"ご注文番号：KC-26407532　→　BS-26888944"#;
        let parser = DmmOrderNumberChangeParser;
        let result = parser.parse_order_number_change(email);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.old_order_number, "KC-26407532");
        assert_eq!(info.new_order_number, "BS-26888944");
    }
}
