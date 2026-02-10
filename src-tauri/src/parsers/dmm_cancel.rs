//! DMM通販「ご注文キャンセルのお知らせ」メール用パーサー
//!
//! 送信元: info@mail.dmm.com
//! 件名: DMM通販：ご注文キャンセルのお知らせ
//!
//! ご注文番号・商品名・キャンセル個数を抽出する。
//! 注文全体のキャンセル時は商品名が記載されない場合がある。

use crate::parsers::cancel_info::CancelInfo;
use regex::Regex;

/// DMM通販 注文キャンセルメール用パーサー
pub struct DmmCancelParser;

impl DmmCancelParser {
    /// メール本文からキャンセル情報を抽出する
    pub fn parse_cancel(&self, email_body: &str) -> Result<CancelInfo, String> {
        let lines: Vec<&str> = email_body.lines().collect();

        let order_number = extract_order_number(&lines)?;
        let product_name = extract_product_name(&lines).unwrap_or_default();
        let cancel_quantity = extract_cancel_quantity(&lines);

        Ok(CancelInfo {
            order_number,
            product_name: product_name.trim().to_string(),
            cancel_quantity,
        })
    }
}

/// 注文番号を抽出（ご注文番号：KC-25278366 形式）
/// 大文字・小文字の両方でパースし、そのまま使用（将来の注文詳細ページURL対応のため）
fn extract_order_number(lines: &[&str]) -> Result<String, String> {
    let prefix_re = Regex::new(r"ご注文番号\s*[：:]\s*([A-Za-z]{2}-\d+)")
        .map_err(|e| format!("Regex error: {e}"))?;
    for line in lines {
        if let Some(captures) = prefix_re.captures(line) {
            if let Some(m) = captures.get(1) {
                return Ok(m.as_str().trim().to_string());
            }
        }
    }
    Err("Order number with prefix (KC-, BS-, etc.) not found".to_string())
}

/// 商品名を抽出（商品名　　：... 形式）
/// 注文全体キャンセル時は商品名が記載されない場合があり、その場合は None を返す
fn extract_product_name(lines: &[&str]) -> Option<String> {
    let re = Regex::new(r"商品名\s*[：:]\s*(.+)").ok()?;

    for line in lines {
        if let Some(captures) = re.captures(line) {
            if let Some(m) = captures.get(1) {
                let s = m.as_str().trim().to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }

    None
}

/// キャンセル個数を抽出（DMM 形式では明示されないためデフォルト 1）
fn extract_cancel_quantity(_lines: &[&str]) -> i64 {
    // DMM のキャンセルメールは 1 商品単位で送られ、個数は未記載のため 1 を返す
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dmm_cancel() {
        let email = r#"原田 裕基 様

DMM通販をご利用いただき、ありがとうございます。
下記の注文商品のキャンセルが完了いたしました。

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
■　キャンセルされた注文内容
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
ご注文日：2024/06/14
ご注文番号：KC-25278366
商品名　　：【11月再生産分】HG 1/144 ガンダムブレイカー バトロジープログラム

キャンセルされた注文は、注文内容一覧ページからも削除されます。
"#;
        let parser = DmmCancelParser;
        let result = parser.parse_cancel(email);

        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let info = result.unwrap();

        assert_eq!(info.order_number, "KC-25278366");
        assert_eq!(
            info.product_name,
            "【11月再生産分】HG 1/144 ガンダムブレイカー バトロジープログラム"
        );
        assert_eq!(info.cancel_quantity, 1);
    }

    #[test]
    fn test_parse_dmm_cancel_lowercase_prefix() {
        let email = r#"ご注文番号：kc-12345678
商品名：サンプル商品"#;
        let parser = DmmCancelParser;
        let result = parser.parse_cancel(email);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.order_number, "kc-12345678");
        assert_eq!(info.product_name, "サンプル商品");
    }

    #[test]
    fn test_parse_dmm_cancel_no_product_name() {
        let email = r#"DMM通販をご利用いただき、ありがとうございます。
下記の注文のキャンセルが完了いたしました。

ご注文日：2024/06/14
ご注文番号：KC-25278366

キャンセルされた注文は、注文内容一覧ページからも削除されます。"#;
        let parser = DmmCancelParser;
        let result = parser.parse_cancel(email);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.order_number, "KC-25278366");
        assert!(info.product_name.is_empty());
        assert_eq!(info.cancel_quantity, 1);
    }
}
