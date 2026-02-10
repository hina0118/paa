//! ホビーサーチ 注文キャンセルメール用パーサー
//!
//! [キャンセル] セクションから注文番号・商品名・キャンセル個数を抽出する。

use crate::parsers::cancel_info::CancelInfo;
use regex::Regex;

/// キャンセルメール用パーサー
pub struct HobbySearchCancelParser;

impl HobbySearchCancelParser {
    /// メール本文からキャンセル情報を抽出する
    pub fn parse_cancel(&self, email_body: &str) -> Result<CancelInfo, String> {
        let lines: Vec<&str> = email_body.lines().collect();

        let order_number = extract_order_number(&lines)?;
        let product_name = extract_product_name(&lines)?;
        let cancel_quantity = extract_cancel_quantity(&lines)?;

        Ok(CancelInfo {
            order_number,
            product_name: product_name.trim().to_string(),
            cancel_quantity,
        })
    }
}

/// 注文番号を抽出（注文番号 ： XX-XXXX-XXXX 形式）
fn extract_order_number(lines: &[&str]) -> Result<String, String> {
    let re =
        Regex::new(r"注文番号\s*[：:]\s*(\d+-\d+-\d+)").map_err(|e| format!("Regex error: {e}"))?;

    for line in lines {
        if let Some(captures) = re.captures(line) {
            if let Some(m) = captures.get(1) {
                return Ok(m.as_str().to_string());
            }
        }
    }

    Err("Order number not found".to_string())
}

/// 商品名を抽出（商品名 ： ... 形式）
fn extract_product_name(lines: &[&str]) -> Result<String, String> {
    let re = Regex::new(r"商品名\s*[：:]\s*(.+)").map_err(|e| format!("Regex error: {e}"))?;

    for line in lines {
        if let Some(captures) = re.captures(line) {
            if let Some(m) = captures.get(1) {
                return Ok(m.as_str().to_string());
            }
        }
    }

    Err("Product name not found".to_string())
}

/// キャンセル個数を抽出（キャンセル個数 ： N 形式）
/// 見つからない場合は 1 をデフォルトとする（形式違いのメールに対応）
fn extract_cancel_quantity(lines: &[&str]) -> Result<i64, String> {
    let re = Regex::new(r"キャンセル個数\s*[：:＝=]\s*(\d+)")
        .map_err(|e| format!("Regex error: {e}"))?;

    for line in lines {
        if let Some(captures) = re.captures(line) {
            if let Some(m) = captures.get(1) {
                return m
                    .as_str()
                    .parse::<i64>()
                    .map_err(|e| format!("Invalid cancel quantity: {e}"));
            }
        }
    }

    // キャンセル個数が明示されていない場合は 1 とする
    Ok(1)
}

#[cfg(all(test, not(ci)))]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hobbysearch_cancel() {
        let sample_email = include_str!("../../../sample/hobbysearch_mail_cancel.txt");
        let parser = HobbySearchCancelParser;
        let result = parser.parse_cancel(sample_email);

        assert!(result.is_ok());
        let info = result.unwrap();

        assert_eq!(info.order_number, "99-9999-9999");
        assert_eq!(
            info.product_name,
            "【抽選販売】 30MS オプションパーツセット22(ターボコスチュームα)[カラーB] (プラモデル)"
        );
        assert_eq!(info.cancel_quantity, 1);
    }
}
