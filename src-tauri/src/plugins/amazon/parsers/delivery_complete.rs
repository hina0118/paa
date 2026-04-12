//! Amazon.co.jp「配達完了」メール用パーサー
//!
//! 送信元: order-update@amazon.co.jp
//!
//! 対応件名:
//! - `ご注文商品はお住まいの建物内の宅配ボックスに配達しました。`
//! - `配達完了:` / `配達完了：`
//! - `配達済み:` / `配達済み：`
//!
//! 本文から注文番号（例: `503-1234567-1234567`）を抽出する。

use regex::Regex;

/// Amazon 配達完了メールのパース結果
#[derive(Debug, PartialEq)]
pub struct AmazonDeliveryInfo {
    /// 注文番号（例: 503-1234567-1234567）
    pub order_number: String,
    /// 配達完了日時（SQLite DATETIME 形式: "YYYY-MM-DD HH:MM:00"）があれば
    pub delivered_at: Option<String>,
}

/// Amazon 配達完了メールをパースする
pub fn parse(body: &str) -> Result<AmazonDeliveryInfo, String> {
    let order_number =
        extract_order_number(body).ok_or_else(|| "注文番号が見つかりません".to_string())?;
    let delivered_at = extract_delivered_at(body);
    Ok(AmazonDeliveryInfo {
        order_number,
        delivered_at,
    })
}

/// 本文から Amazon 注文番号（NNN-NNNNNNN-NNNNNNN）を抽出する
fn extract_order_number(body: &str) -> Option<String> {
    let re = Regex::new(r"\b(\d{3}-\d{7}-\d{7})\b").ok()?;
    re.captures(body)
        .map(|cap| cap[1].to_string())
}

/// 本文から配達日時を抽出し "YYYY-MM-DD HH:MM:00" に変換する
///
/// 対応フォーマット例:
/// - `2026/04/12 14:30`
/// - `2026年4月12日 14:30`
fn extract_delivered_at(body: &str) -> Option<String> {
    // YYYY/MM/DD HH:MM 形式
    let re_slash =
        Regex::new(r"(\d{4})/(\d{1,2})/(\d{1,2})[^\d]+(\d{2}):(\d{2})").ok()?;
    if let Some(cap) = re_slash.captures(body) {
        return Some(format!(
            "{}-{:02}-{:02} {}:{}:00",
            &cap[1],
            cap[2].parse::<u32>().unwrap_or(0),
            cap[3].parse::<u32>().unwrap_or(0),
            &cap[4],
            &cap[5],
        ));
    }

    // YYYY年M月D日 HH:MM 形式
    let re_kanji =
        Regex::new(r"(\d{4})年(\d{1,2})月(\d{1,2})日[^\d]+(\d{2}):(\d{2})").ok()?;
    if let Some(cap) = re_kanji.captures(body) {
        return Some(format!(
            "{}-{:02}-{:02} {}:{}:00",
            &cap[1],
            cap[2].parse::<u32>().unwrap_or(0),
            cap[3].parse::<u32>().unwrap_or(0),
            &cap[4],
            &cap[5],
        ));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_BODY_TAKUHAI: &str = "\
Amazon.co.jp

山田 太郎 様

ご注文商品はお住まいの建物内の宅配ボックスに配達しました。

注文番号: 503-1234567-1234567
配達日時: 2026/04/12 14:30

ご利用ありがとうございました。
";

    const SAMPLE_BODY_DELIVERED: &str = "\
Amazon.co.jp

山田 太郎 様

配達完了: ご注文が配達されました。

ご注文番号
250-9876543-7654321

お届け日: 2026年4月10日 09:05

詳細はAmazonのウェブサイトでご確認ください。
";

    const SAMPLE_BODY_NO_DATE: &str = "\
Amazon.co.jp

配達済み：

注文番号: 112-1111111-2222222

お届け先: 東京都...
";

    #[test]
    fn test_parse_takuhai_box() {
        let result = parse(SAMPLE_BODY_TAKUHAI).unwrap();
        assert_eq!(result.order_number, "503-1234567-1234567");
        assert_eq!(result.delivered_at, Some("2026-04-12 14:30:00".to_string()));
    }

    #[test]
    fn test_parse_delivered_kanji_date() {
        let result = parse(SAMPLE_BODY_DELIVERED).unwrap();
        assert_eq!(result.order_number, "250-9876543-7654321");
        assert_eq!(result.delivered_at, Some("2026-04-10 09:05:00".to_string()));
    }

    #[test]
    fn test_parse_no_date() {
        let result = parse(SAMPLE_BODY_NO_DATE).unwrap();
        assert_eq!(result.order_number, "112-1111111-2222222");
        assert_eq!(result.delivered_at, None);
    }

    #[test]
    fn test_parse_no_order_number() {
        let body = "配達が完了しました。詳細はサイトをご確認ください。";
        assert!(parse(body).is_err());
    }

    #[test]
    fn test_extract_order_number_various() {
        assert_eq!(
            extract_order_number("注文番号: 503-1234567-1234567"),
            Some("503-1234567-1234567".to_string())
        );
        assert_eq!(
            extract_order_number("250-0000001-9999999 のご注文"),
            Some("250-0000001-9999999".to_string())
        );
        assert_eq!(extract_order_number("注文なし"), None);
    }
}
