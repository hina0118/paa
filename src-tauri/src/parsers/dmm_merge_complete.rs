//! DMM通販「ご注文まとめ完了のお知らせ」メール用パーサー
//!
//! 送信元: info@mail.dmm.com
//! 件名: DMM通販：ご注文まとめ完了のお知らせ
//!
//! 複数注文を1注文にまとめた旨の通知。まとめる前の注文番号リストとまとめた後の注文番号を抽出する。

use crate::parsers::consolidation_info::ConsolidationInfo;
use regex::Regex;
use std::collections::HashSet;

/// DMM通販 ご注文まとめ完了お知らせメール用パーサー
pub struct DmmMergeCompleteParser;

impl DmmMergeCompleteParser {
    /// メール本文からまとめ完了情報を抽出する
    pub fn parse_consolidation(&self, email_body: &str) -> Result<ConsolidationInfo, String> {
        let new_number = extract_new_order_number(email_body)?;
        let old_numbers = extract_old_order_numbers(email_body)?;
        if old_numbers.is_empty() {
            return Err("No old order numbers found".to_string());
        }
        Ok(ConsolidationInfo {
            old_order_numbers: old_numbers,
            new_order_number: new_number,
        })
    }
}

/// まとめた後のご注文番号: KC-xxxxx を抽出
fn extract_new_order_number(body: &str) -> Result<String, String> {
    let re = Regex::new(r"まとめた後のご注文番号\s*[：:]\s*([A-Za-z]{2}-\d+)")
        .map_err(|e| format!("Regex error: {e}"))?;
    for line in body.lines() {
        if let Some(cap) = re.captures(line.trim()) {
            if let Some(m) = cap.get(1) {
                return Ok(m.as_str().trim().to_string());
            }
        }
    }
    Err("New order number (まとめた後のご注文番号) not found".to_string())
}

/// まとめる前のご注文番号ブロックから 1: KC-xxx, 2: KC-yyy 形式を抽出。
/// 同一番号の重複は除去し、出現順を保つ（look-ahead 非対応の regex のため、ブロックは「まとめた後」の手前まで）。
fn extract_old_order_numbers(body: &str) -> Result<Vec<String>, String> {
    let line_re =
        Regex::new(r"\d+\s*[：:]\s*([A-Za-z]{2}-\d+)").map_err(|e| format!("Regex error: {e}"))?;

    let mut numbers = Vec::new();
    let mut seen = HashSet::new();
    let mut in_block = false;
    for line in body.lines() {
        let line = line.trim();
        if line.contains("まとめた後のご注文番号") {
            break;
        }
        if line.contains("まとめる前のご注文番号") {
            in_block = true;
            continue;
        }
        if in_block {
            if let Some(cap) = line_re.captures(line) {
                if let Some(num) = cap.get(1) {
                    let s = num.as_str().trim().to_string();
                    if seen.insert(s.clone()) {
                        numbers.push(s);
                    }
                }
            }
        }
    }
    Ok(numbers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dmm_merge_complete() {
        let body = r#"DMM通販をご利用いただき、ありがとうございます。
ご注文まとめ手続きが完了いたしました。

■■ 注文番号変更のお知らせ ■■
以下のようにご注文番号が変更されました。

!!!!!!!!!!!!!!!!!!!!!!!!!!!
まとめる前のご注文番号
1: KC-25278407
2: KC-25278407
3: KC-25285201
!!!!!!!!!!!!!!!!!!!!!!!!!!!
========================================
まとめた後のご注文番号: KC-25285222
========================================

まとめる前に指定していた決済方法は削除されました。
"#;
        let parser = DmmMergeCompleteParser;
        let result = parser.parse_consolidation(body);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let info = result.unwrap();
        assert_eq!(info.new_order_number, "KC-25285222");
        // 重複除去後は2件（KC-25278407, KC-25285201）
        assert_eq!(info.old_order_numbers.len(), 2);
        assert_eq!(info.old_order_numbers[0], "KC-25278407");
        assert_eq!(info.old_order_numbers[1], "KC-25285201");
    }

    #[test]
    fn test_parse_dmm_merge_complete_new_number_only() {
        let body = "まとめた後のご注文番号: BS-12345678";
        let parser = DmmMergeCompleteParser;
        let result = parser.parse_consolidation(body);
        assert!(result.is_err()); // old numbers empty
    }

    #[test]
    fn test_extract_new_order_number() {
        let body = "まとめた後のご注文番号: KC-25285222";
        let n = extract_new_order_number(body).unwrap();
        assert_eq!(n, "KC-25285222");
    }

    #[test]
    fn test_extract_old_order_numbers() {
        let body = r#"まとめる前のご注文番号
1: KC-25278407
2: KC-25285201
まとめた後のご注文番号: KC-25285222"#;
        let nums = extract_old_order_numbers(body).unwrap();
        assert_eq!(nums, vec!["KC-25278407", "KC-25285201"]);
    }

    #[test]
    fn test_extract_old_order_numbers_dedup() {
        let body = r#"まとめる前のご注文番号
1: KC-25278407
2: KC-25278407
3: KC-25285201
まとめた後のご注文番号: KC-25285222"#;
        let nums = extract_old_order_numbers(body).unwrap();
        assert_eq!(nums, vec!["KC-25278407", "KC-25285201"], "重複は除去され出現順を保つ");
    }
}
