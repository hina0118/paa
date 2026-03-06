use crate::parsers::OrderItem;
use once_cell::sync::Lazy;
use regex::Regex;

pub mod confirm;
pub mod send;

/// `[商品名]：商品名[SKU]       単価 円 x  個数 個       合計 円` 形式の行パターン
///
/// コロンは全角（`：` U+FF1A）・半角（`:` U+003A）両方に対応する。
/// ISO-2022-JP 由来のメールは全角コロンで届く場合がある。
///
/// `.+` をグリーディにすることで、商品名に `[カラーC]` 等のブラケットが含まれる場合でも
/// SKU（末尾の最後のブラケット）を正しく抽出できる。
static ITEM_LINE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\[商品名\][：:](.+)\[([^\]]+)\]\s+([\d,]+)\s*円\s+x\s+(\d+)\s*個\s+([\d,]+)\s*円")
        .expect("Invalid ITEM_LINE_RE")
});

/// `YYYY年M月D日 HH:MM` 形式の受注日時パターン
static ORDER_DATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\d{4})年(\d+)月(\d+)日\s+(\d{1,2}:\d{2})").expect("Invalid ORDER_DATE_RE")
});

/// 金額抽出用パターン（`N,NNN 円` 形式）
static AMOUNT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([\d,]+)\s*円").expect("Invalid AMOUNT_RE"));

/// 発送方法行パターン（`発送方法  : ヤマト宅急便` 等）
static CARRIER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"発送方法\s*[：:]\s*(.+)").expect("Invalid CARRIER_RE")
});

/// `[商品名]:...` 行から OrderItem を抽出する
///
/// 商品名に `[xxx]` 形式のブラケットが含まれる場合も対応（末尾ブラケットを SKU とみなす）。
pub fn parse_item_line(line: &str) -> Option<OrderItem> {
    let caps = ITEM_LINE_RE.captures(line)?;
    let name = caps[1].trim().to_string();
    let model_number = Some(caps[2].to_string());
    let unit_price = caps[3].replace(',', "").parse::<i64>().ok()?;
    let quantity = caps[4].parse::<i64>().ok()?;
    let subtotal = caps[5].replace(',', "").parse::<i64>().ok()?;

    Some(OrderItem {
        name,
        manufacturer: None,
        model_number,
        unit_price,
        quantity,
        subtotal,
        image_url: None,
    })
}

/// 商品小計・送料・合計を抽出する
///
/// `商品合計` / `送料合計` は集計行であるため除外する。
pub fn extract_amounts(lines: &[&str]) -> (Option<i64>, Option<i64>, Option<i64>) {
    let mut subtotal = None;
    let mut shipping_fee = None;
    let mut total_amount = None;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("商品小計") {
            subtotal = parse_amount(trimmed);
        } else if trimmed.starts_with("送料") && !trimmed.starts_with("送料合計") {
            shipping_fee = parse_amount(trimmed);
        } else if trimmed.starts_with("合計") {
            total_amount = parse_amount(trimmed);
        }
    }

    (subtotal, shipping_fee, total_amount)
}

/// 受注日時を `"YYYY-MM-DD HH:MM"` 形式で抽出する
///
/// 本文中の `YYYY年M月D日 HH:MM` パターンを探して返す。
pub fn extract_order_date(lines: &[&str]) -> Option<String> {
    for line in lines {
        if let Some(caps) = ORDER_DATE_RE.captures(line) {
            let year = &caps[1];
            let month: u32 = caps[2].parse().ok()?;
            let day: u32 = caps[3].parse().ok()?;
            let time = &caps[4];
            return Some(format!("{}-{:02}-{:02} {}", year, month, day, time));
        }
    }
    None
}

/// `発送方法  : ヤマト宅急便` のような行から配送業者名を抽出する
///
/// 行が見つからない場合は `None` を返す。
pub fn extract_carrier(lines: &[&str]) -> Option<String> {
    for line in lines {
        if let Some(caps) = CARRIER_RE.captures(line) {
            let carrier = caps[1].trim().to_string();
            if !carrier.is_empty() {
                return Some(carrier);
            }
        }
    }
    None
}

fn parse_amount(line: &str) -> Option<i64> {
    AMOUNT_RE
        .captures(line)
        .and_then(|caps| caps[1].replace(',', "").parse::<i64>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_item_line_fullwidth_colon() {
        // 実際のメールは全角コロン `：`（U+FF1A）
        let line = "[商品名]：バンダイ ノンスケール ＳＤ ＥＸ-スタンダードシリーズ No.004 ウィングガンダムゼロ ＥＷ[bd-sdex-004]       594 円 x  1 個       594 円";
        let item = parse_item_line(line).unwrap();
        assert!(item.name.contains("ウィングガンダムゼロ"));
        assert_eq!(item.model_number, Some("bd-sdex-004".to_string()));
        assert_eq!(item.unit_price, 594);
        assert_eq!(item.quantity, 1);
        assert_eq!(item.subtotal, 594);
    }

    #[test]
    fn test_parse_item_line_halfwidth_colon() {
        // 半角コロン `:` にも対応（後方互換）
        let line = "[商品名]:バンダイ ノンスケール SD EX ウィングガンダムゼロ EW[bd-sdex-004]       594 円 x  1 個       594 円";
        let item = parse_item_line(line).unwrap();
        assert_eq!(item.model_number, Some("bd-sdex-004".to_string()));
        assert_eq!(item.unit_price, 594);
    }

    #[test]
    fn test_parse_item_line_brackets_in_name() {
        // 商品名に [カラーC] を含むケース（全角コロン）
        let line = "[商品名]：バンダイ 30MS OB-11 アームパーツ&レッグパーツ[カラーC][bd-30ms-ob11]       880 円 x  1 個       880 円";
        let item = parse_item_line(line).unwrap();
        assert!(item.name.contains("[カラーC]"));
        assert_eq!(item.model_number, Some("bd-30ms-ob11".to_string()));
        assert_eq!(item.unit_price, 880);
    }

    #[test]
    fn test_parse_item_line_fullwidth_brackets_in_name() {
        // 全角ブラケット ［ホワイト/ブラック］ を含む商品名（全角コロン）
        let line = "[商品名]：バンダイ 30MS OB-12 アームパーツ＆レッグパーツ［ホワイト/ブラック］[bd-30ms-ob012]       990 円 x  1 個       990 円";
        let item = parse_item_line(line).unwrap();
        assert!(item.name.contains("［ホワイト/ブラック］"));
        assert_eq!(item.model_number, Some("bd-30ms-ob012".to_string()));
        assert_eq!(item.unit_price, 990);
    }

    #[test]
    fn test_parse_item_line_with_comma_price() {
        let line = "[商品名]：テスト商品[test-001]       1,980 円 x  2 個       3,960 円";
        let item = parse_item_line(line).unwrap();
        assert_eq!(item.unit_price, 1980);
        assert_eq!(item.quantity, 2);
        assert_eq!(item.subtotal, 3960);
    }

    #[test]
    fn test_parse_item_line_not_matching() {
        assert!(parse_item_line("普通のテキスト行").is_none());
        assert!(parse_item_line("  商品小計  4,268 円").is_none());
    }

    #[test]
    fn test_extract_amounts_all() {
        let lines = vec![
            "  商品小計             4,268 円",
            "  送料                   1,200 円",
            "  商品合計             4,268 円",
            "  送料合計             1,200 円",
            "  合計                   5,468 円",
        ];
        let (subtotal, shipping, total) = extract_amounts(&lines);
        assert_eq!(subtotal, Some(4268));
        assert_eq!(shipping, Some(1200));
        assert_eq!(total, Some(5468));
    }

    #[test]
    fn test_extract_amounts_excludes_aggregate_lines() {
        // 商品合計・送料合計は subtotal / shipping に入らない
        let lines = vec![
            "  商品合計             4,268 円",
            "  送料合計             1,200 円",
        ];
        let (subtotal, shipping, total) = extract_amounts(&lines);
        assert_eq!(subtotal, None);
        assert_eq!(shipping, None);
        assert_eq!(total, None);
    }

    #[test]
    fn test_extract_order_date() {
        let lines = vec!["★ 受注日時", "  2023年6月15日 02:17"];
        assert_eq!(
            extract_order_date(&lines),
            Some("2023-06-15 02:17".to_string())
        );
    }

    #[test]
    fn test_extract_order_date_inline() {
        // セクションヘッダと同じ行にある場合
        let lines = vec!["★ 受注日時  2023年6月15日 02:17"];
        assert_eq!(
            extract_order_date(&lines),
            Some("2023-06-15 02:17".to_string())
        );
    }

    #[test]
    fn test_extract_order_date_single_digit_month() {
        let lines = vec!["  2023年1月5日 09:03"];
        assert_eq!(
            extract_order_date(&lines),
            Some("2023-01-05 09:03".to_string())
        );
    }

    #[test]
    fn test_extract_order_date_not_found() {
        let lines = vec!["注文確認メール", "キッズドラゴンです"];
        assert_eq!(extract_order_date(&lines), None);
    }
}
