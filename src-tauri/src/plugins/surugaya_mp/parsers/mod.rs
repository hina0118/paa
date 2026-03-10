use once_cell::sync::Lazy;
use regex::Regex;

pub mod confirm;
pub mod send;

// ─────────────────────────────────────────────────────────────────────────────
// 正規表現
// ─────────────────────────────────────────────────────────────────────────────

/// `<br>` / `<br/>` / `<br />` タグを改行に置換するパターン
static BR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)<br\s*/?>").expect("Invalid BR_RE"));

/// HTML タグ全体を除去するパターン
static HTML_TAG_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<[^>]+>").expect("Invalid HTML_TAG_RE"));

/// confirm 取引番号: `お客様のご注文番号 [ M2502021943 ] になります。`
/// 桁数は 9〜12 桁に対応（実績: 10桁・11桁）
static CONFIRM_ORDER_NUMBER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[\s*(M\d{9,12})\s*\]").expect("Invalid CONFIRM_ORDER_NUMBER_RE")
});

/// send 取引番号: `取引番号：M2603039345` (全角・半角コロン両対応)
/// 桁数は 9〜12 桁に対応（実績: 10桁・11桁）
static SEND_ORDER_NUMBER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"取引番号[：:]\s*(M\d{9,12})").expect("Invalid SEND_ORDER_NUMBER_RE")
});

/// マイページURL: `https://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=M2603039345`
/// href 属性内またはプレーンテキストの両方に対応する。桁数は 9〜12 桁に対応。
static MYPAGE_URL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"https://www\.suruga-ya\.jp/pcmypage/action_sell_search/detail\?trade_code=(M\d{9,12})",
    )
    .expect("Invalid MYPAGE_URL_RE")
});

// ─────────────────────────────────────────────────────────────────────────────
// 共通ヘルパー
// ─────────────────────────────────────────────────────────────────────────────

/// HTML ボディをテキスト行のリストに変換する
///
/// 1. `<br>` / `<br/>` / `<br />` を改行に置換
/// 2. HTML タグを除去
/// 3. 改行で分割し、各行をトリム
fn html_to_lines(html: &str) -> Vec<String> {
    let with_newlines = BR_RE.replace_all(html, "\n");
    let without_tags = HTML_TAG_RE.replace_all(&with_newlines, "");
    without_tags
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// メール本文をテキスト行のリストに変換する
///
/// HTML が含まれる場合は `html_to_lines()` を使用し、
/// プレーンテキストの場合はそのまま分割する。
pub fn body_to_lines(body: &str) -> Vec<String> {
    if body.contains("<br") || body.contains("<BR") || body.contains("<p") || body.contains("<P") {
        html_to_lines(body)
    } else {
        body.lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    }
}

/// confirm メール用取引番号抽出: `[ M2502021943 ]` 形式
pub fn extract_confirm_order_number(lines: &[&str]) -> Option<String> {
    lines.iter().find_map(|line| {
        CONFIRM_ORDER_NUMBER_RE
            .captures(line)
            .map(|c| c[1].to_string())
    })
}

/// send メール用取引番号抽出: `取引番号：M2603039345` 形式
pub fn extract_send_order_number(lines: &[&str]) -> Option<String> {
    lines.iter().find_map(|line| {
        SEND_ORDER_NUMBER_RE
            .captures(line)
            .map(|c| c[1].to_string())
    })
}

/// マイページURL を抽出する（HTML の href 属性またはプレーンテキスト）
pub fn extract_mypage_url(body: &str) -> Option<String> {
    MYPAGE_URL_RE.find(body).map(|m| m.as_str().to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// テスト
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── body_to_lines ───

    #[test]
    fn test_body_to_lines_html_strips_tags() {
        let html = "<p>取引番号：M2603039345</p><br/>テスト";
        let lines = body_to_lines(html);
        assert!(lines.contains(&"取引番号：M2603039345".to_string()));
        assert!(lines.contains(&"テスト".to_string()));
    }

    #[test]
    fn test_body_to_lines_plain_passthrough() {
        let plain = "line1\nline2\nline3";
        let lines = body_to_lines(plain);
        assert_eq!(lines, vec!["line1", "line2", "line3"]);
    }

    #[test]
    fn test_body_to_lines_filters_empty() {
        let html = "<p>テスト</p>\n\n<br/>\n<p>   </p>";
        let lines = body_to_lines(html);
        assert_eq!(lines, vec!["テスト"]);
    }

    // ─── extract_confirm_order_number ───

    #[test]
    fn test_extract_confirm_order_number() {
        let lines = vec!["お客様のご注文番号 [ M2502021943 ] になります。"];
        assert_eq!(
            extract_confirm_order_number(&lines),
            Some("M2502021943".to_string())
        );
    }

    #[test]
    fn test_extract_confirm_order_number_with_spaces() {
        // スペース数が異なるケースも対応
        let lines = vec!["[ M2603039345 ]"];
        assert_eq!(
            extract_confirm_order_number(&lines),
            Some("M2603039345".to_string())
        );
    }

    #[test]
    fn test_extract_confirm_order_number_11_digits() {
        // 実績: M25110817482 (11桁) - email id=1207
        let lines = vec!["お客様のお取引番号は [ M25110817482 ] になります。"];
        assert_eq!(
            extract_confirm_order_number(&lines),
            Some("M25110817482".to_string())
        );
    }

    #[test]
    fn test_extract_confirm_order_number_not_found() {
        let lines = vec!["取引番号：M2603039345"]; // send 形式は confirm では取得しない
        assert_eq!(extract_confirm_order_number(&lines), None);
    }

    // ─── extract_send_order_number ───

    #[test]
    fn test_extract_send_order_number_zenkaku() {
        let lines = vec!["取引番号：M2603039345"];
        assert_eq!(
            extract_send_order_number(&lines),
            Some("M2603039345".to_string())
        );
    }

    #[test]
    fn test_extract_send_order_number_hankaku() {
        let lines = vec!["取引番号:M2603039345"];
        assert_eq!(
            extract_send_order_number(&lines),
            Some("M2603039345".to_string())
        );
    }

    #[test]
    fn test_extract_send_order_number_not_found() {
        let lines = vec!["[ M2603039345 ]"]; // confirm 形式は send では取得しない
        assert_eq!(extract_send_order_number(&lines), None);
    }

    // ─── extract_mypage_url ───

    #[test]
    fn test_extract_mypage_url_from_href() {
        let body = r#"<a href="https://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=M2603039345">リンク</a>"#;
        assert_eq!(
            extract_mypage_url(body),
            Some("https://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=M2603039345".to_string())
        );
    }

    #[test]
    fn test_extract_mypage_url_plaintext() {
        let body = "こちらより確認をお願い致します。\nhttps://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=M2502021943\n";
        assert_eq!(
            extract_mypage_url(body),
            Some("https://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=M2502021943".to_string())
        );
    }

    #[test]
    fn test_extract_mypage_url_11_digit_trade_code() {
        // 実績: M25110817482 (11桁)
        let body = "https://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=M25110817482";
        assert_eq!(
            extract_mypage_url(body),
            Some("https://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=M25110817482".to_string())
        );
    }

    #[test]
    fn test_extract_mypage_url_not_found() {
        let body = "ご注文ありがとうございます。";
        assert_eq!(extract_mypage_url(body), None);
    }
}
