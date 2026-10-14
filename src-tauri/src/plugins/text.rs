//! プラグイン共通のメール本文テキスト処理ヘルパー

use once_cell::sync::Lazy;
use regex::Regex;

/// `<br>` / `<br/>` / `<br />`（大文字小文字無視）
static BR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)<br\s*/?>").expect("Invalid BR_RE"));

/// HTML タグ全体を除去するパターン
static HTML_TAG_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<[^>]+>").expect("Invalid HTML_TAG_RE"));

/// プレーンテキスト本文を行に分割し、各行をトリムして返す
pub fn plain_to_lines(body: &str) -> Vec<String> {
    body.lines().map(|l| l.trim().to_string()).collect()
}

/// HTML ボディをテキスト行のリストに変換する
///
/// 1. `<br>` / `<br/>` / `<br />` を改行に置換
/// 2. HTML タグを除去
/// 3. 改行で分割し、各行をトリム
pub fn html_to_lines(html: &str) -> Vec<String> {
    let with_newlines = BR_RE.replace_all(html, "\n");
    let without_tags = HTML_TAG_RE.replace_all(&with_newlines, "");
    plain_to_lines(&without_tags)
}

/// `1,234` 形式の金額文字列を数値に変換する（カンマ除去・前後空白トリム）
pub fn parse_yen(s: &str) -> Option<i64> {
    s.replace(',', "").trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_to_lines_trims_each_line() {
        assert_eq!(plain_to_lines("  a \n\tb\n"), vec!["a", "b"]);
    }

    #[test]
    fn test_html_to_lines_converts_br_and_strips_tags() {
        let lines = html_to_lines("<p>注文番号：1</p><BR/> <b>合計</b><br >100円");
        assert_eq!(lines, vec!["注文番号：1", "合計", "100円"]);
    }

    #[test]
    fn test_parse_yen() {
        assert_eq!(parse_yen(" 12,345 "), Some(12345));
        assert_eq!(parse_yen("abc"), None);
    }
}
