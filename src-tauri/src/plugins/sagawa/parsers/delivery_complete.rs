//! 佐川急便「配達完了通知サービス」メール用パーサー
//!
//! 送信元: info-nimotsu@sagawa-exp.co.jp
//!
//! 本文フォーマット（ISO-2022-JP デコード後）:
//!
//! ```text
//! ◆お問い合わせ送り状No.
//! 470551104391
//!
//! ◆お届け完了日時
//! 2026/03/04（水） 11:18
//! ```

use regex::Regex;

/// 佐川急便 配達完了メールのパース結果
#[derive(Debug, PartialEq)]
pub struct SagawaDeliveryInfo {
    /// 送り状番号
    pub tracking_number: String,
    /// 配達完了日時（SQLite DATETIME 形式: "YYYY-MM-DD HH:MM:00"）
    pub delivered_at: Option<String>,
}

/// 佐川急便 配達完了メールをパースする
pub fn parse(body: &str) -> Result<SagawaDeliveryInfo, String> {
    let tracking_number = extract_tracking_number(body)
        .ok_or_else(|| "送り状No.が見つかりません".to_string())?;
    let delivered_at = extract_delivered_at(body);
    Ok(SagawaDeliveryInfo {
        tracking_number,
        delivered_at,
    })
}

/// "◆お問い合わせ送り状No." の直後の非空行を抽出
fn extract_tracking_number(body: &str) -> Option<String> {
    let mut found_marker = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if found_marker {
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        } else if trimmed.contains("送り状No") || trimmed.contains("送り状No.") {
            found_marker = true;
        }
    }
    None
}

/// "◆お届け完了日時" の直後の行から日時を抽出し、"YYYY-MM-DD HH:MM:00" に変換
///
/// 入力例: "2026/03/04（水） 11:18"
fn extract_delivered_at(body: &str) -> Option<String> {
    let re = Regex::new(r"(\d{4})/(\d{2})/(\d{2})[^0-9]+(\d{2}):(\d{2})").ok()?;
    let mut found_marker = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if found_marker {
            if !trimmed.is_empty() {
                if let Some(cap) = re.captures(trimmed) {
                    let dt = format!(
                        "{}-{}-{} {}:{}:00",
                        &cap[1], &cap[2], &cap[3], &cap[4], &cap[5]
                    );
                    return Some(dt);
                }
                // 次行に日時がなければ諦める
                return None;
            }
        } else if trimmed.contains("お届け完了日時") || trimmed.contains("配達完了日時") {
            found_marker = true;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_BODY: &str = "\
福田裕基 様

いつも佐川急便をご利用いただきありがとうございます。
お客様宛のお荷物は下記日時に配達が完了しましたのでお知らせいたします。

◆お問い合わせ送り状No.
470551104391

◆お届け完了日時
2026/03/04（水） 11:18

◆配達担当営業所
福岡営業所
";

    #[test]
    fn test_parse_tracking_number() {
        let result = parse(SAMPLE_BODY).unwrap();
        assert_eq!(result.tracking_number, "470551104391");
    }

    #[test]
    fn test_parse_delivered_at() {
        let result = parse(SAMPLE_BODY).unwrap();
        assert_eq!(result.delivered_at, Some("2026-03-04 11:18:00".to_string()));
    }

    #[test]
    fn test_parse_no_tracking_number() {
        let body = "◆お届け完了日時\n2026/03/04（水） 11:18\n";
        assert!(parse(body).is_err());
    }

    #[test]
    fn test_parse_no_delivered_at() {
        let body = "◆お問い合わせ送り状No.\n123456789\n";
        let result = parse(body).unwrap();
        assert_eq!(result.tracking_number, "123456789");
        assert_eq!(result.delivered_at, None);
    }

    #[test]
    fn test_extract_delivered_at_various_weekdays() {
        // 曜日表記が変わっても抽出できること
        for weekday in &["月", "火", "水", "木", "金", "土", "日"] {
            let body = format!(
                "◆お問い合わせ送り状No.\n999\n\n◆お届け完了日時\n2025/12/25（{}） 09:05\n",
                weekday
            );
            let result = parse(&body).unwrap();
            assert_eq!(result.delivered_at, Some("2025-12-25 09:05:00".to_string()));
        }
    }
}
