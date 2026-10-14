pub mod cancel;
pub mod confirm;
pub mod send;

/// 金額文字列を数値に変換する（パースできない場合は 0）
fn parse_amount(s: &str) -> i64 {
    crate::plugins::text::parse_yen(s).unwrap_or(0)
}

/// `2019年06月12日` → `"2019-06-12 00:00:00"`
fn parse_yodobashi_date(year: &str, month: &str, day: &str) -> String {
    format!("{}-{}-{} 00:00:00", year, month, day)
}
