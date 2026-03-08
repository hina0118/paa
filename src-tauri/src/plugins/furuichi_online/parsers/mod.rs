use crate::parsers::OrderItem;
use once_cell::sync::Lazy;
use regex::Regex;

pub mod confirm;
pub mod send;

/// `ご注文番号：100409780` パターン
static ORDER_NUMBER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^ご注文番号：(\d+)").expect("Invalid ORDER_NUMBER_RE"));

/// `ご注文日：2026-03-03 22:25:08` パターン
static ORDER_DATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^ご注文日：(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})").expect("Invalid ORDER_DATE_RE")
});

/// `商品名:1個` パターン（商品行を商品名と数量に分割）
///
/// ふるいちオンラインの商品行は `商品名全体:数量個` の 1 行形式。
/// 商品名にも `:` が含まれる可能性があるため、末尾の `:N個` のみを数量として扱う。
static ITEM_LINE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(.+):(\d+)個$").expect("Invalid ITEM_LINE_RE"));

/// `商品小計（税込）「6,158」円` パターン
static SUBTOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^商品小計（税込）「([\d,]+)」円").expect("Invalid SUBTOTAL_RE"));

/// `送料(税込)「0」円` パターン
static SHIPPING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^送料\(税込\)「([\d,]+)」円").expect("Invalid SHIPPING_RE"));

/// `ご注文金額合計（税込）「6,158」円` パターン
static TOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^ご注文金額合計（税込）「([\d,]+)」円").expect("Invalid TOTAL_RE"));

/// `配送会社：ゆうパケット` パターン
static CARRIER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^配送会社：(.+)").expect("Invalid CARRIER_RE"));

/// `伝票番号：680156937342` パターン
static TRACKING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^伝票番号：(\d+)").expect("Invalid TRACKING_RE"));

/// メール本文をテキスト行のリストに変換する（プレーンテキスト専用）
///
/// ふるいちオンラインのメールはプレーンテキスト形式のため、HTML 変換は不要。
/// 各行をトリムして返す。
pub fn body_to_lines(body: &str) -> Vec<String> {
    body.lines().map(|l| l.trim().to_string()).collect()
}

/// `ご注文番号：100409780` から注文番号を抽出する
pub fn extract_order_number(lines: &[&str]) -> Option<String> {
    lines.iter().find_map(|line| {
        ORDER_NUMBER_RE
            .captures(line.trim())
            .map(|c| c[1].to_string())
    })
}

/// `ご注文日：2026-03-03 22:25:08` から注文日を抽出する
pub fn extract_order_date(lines: &[&str]) -> Option<String> {
    lines.iter().find_map(|line| {
        ORDER_DATE_RE
            .captures(line.trim())
            .map(|c| c[1].to_string())
    })
}

/// `ご注文商品：` セクション以降の商品行を抽出する
///
/// 商品行フォーマット: `商品名:数量個`
/// セクション開始: `ご注文商品：`
/// セクション終了: `-----...` の区切り行（次のセクション）
///
/// 単価・商品小計はメールに含まれないため `unit_price` / `subtotal` を 0 とする。
pub fn extract_items(lines: &[&str]) -> Vec<OrderItem> {
    let mut items: Vec<OrderItem> = Vec::new();
    let mut in_items_section = false;

    for line in lines {
        let trimmed = line.trim();

        if trimmed == "ご注文商品：" {
            in_items_section = true;
            continue;
        }

        if !in_items_section {
            continue;
        }

        // `-----` 区切り行でセクション終了
        if trimmed.starts_with("-----") {
            break;
        }

        // 空行はスキップ
        if trimmed.is_empty() {
            continue;
        }

        if let Some(caps) = ITEM_LINE_RE.captures(trimmed) {
            let name = caps[1].trim().to_string();
            let quantity: i64 = caps[2].parse().unwrap_or(1);
            items.push(OrderItem {
                name,
                manufacturer: None,
                model_number: None,
                unit_price: 0,
                quantity,
                subtotal: 0,
                image_url: None,
            });
        }
    }

    items
}

/// `商品小計（税込）「6,158」円` から商品小計を抽出する
pub fn extract_subtotal(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        SUBTOTAL_RE
            .captures(line.trim())
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// `送料(税込)「0」円` から送料を抽出する
pub fn extract_shipping_fee(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        SHIPPING_RE
            .captures(line.trim())
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// `ご注文金額合計（税込）「6,158」円` から合計金額を抽出する
pub fn extract_total_amount(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        TOTAL_RE
            .captures(line.trim())
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// `配送会社：ゆうパケット` から配送会社を抽出する
pub fn extract_carrier(lines: &[&str]) -> Option<String> {
    lines.iter().find_map(|line| {
        CARRIER_RE
            .captures(line.trim())
            .map(|c| c[1].trim().to_string())
    })
}

/// `伝票番号：680156937342` から伝票番号を抽出する
pub fn extract_tracking_number(lines: &[&str]) -> Option<String> {
    lines
        .iter()
        .find_map(|line| TRACKING_RE.captures(line.trim()).map(|c| c[1].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── extract_order_number ───

    #[test]
    fn test_extract_order_number() {
        let lines = vec!["ご注文番号：100409780"];
        assert_eq!(extract_order_number(&lines), Some("100409780".to_string()));
    }

    #[test]
    fn test_extract_order_number_not_found() {
        let lines = vec!["注文内容をご確認ください"];
        assert_eq!(extract_order_number(&lines), None);
    }

    // ─── extract_order_date ───

    #[test]
    fn test_extract_order_date() {
        let lines = vec!["ご注文日：2026-03-03 22:25:08"];
        assert_eq!(
            extract_order_date(&lines),
            Some("2026-03-03 22:25:08".to_string())
        );
    }

    #[test]
    fn test_extract_order_date_not_found() {
        let lines = vec!["ご注文番号：100409780"];
        assert_eq!(extract_order_date(&lines), None);
    }

    // ─── extract_items ───

    #[test]
    fn test_extract_items_single() {
        let lines = vec![
            "ご注文商品：",
            "テスト商品:1個",
            "---------------------------------------------",
        ];
        let items = extract_items(&lines);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "テスト商品");
        assert_eq!(items[0].quantity, 1);
        assert_eq!(items[0].unit_price, 0);
        assert_eq!(items[0].subtotal, 0);
    }

    #[test]
    fn test_extract_items_multiple() {
        let lines = vec![
            "ご注文商品：",
            "03ゼウスⅠ　カルノージャート:1個",
            "030カルノージャート　エクサ:1個",
            "---------------------------------------------",
        ];
        let items = extract_items(&lines);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "03ゼウスⅠ　カルノージャート");
        assert_eq!(items[0].quantity, 1);
        assert_eq!(items[1].name, "030カルノージャート　エクサ");
        assert_eq!(items[1].quantity, 1);
    }

    #[test]
    fn test_extract_items_quantity_greater_than_one() {
        let lines = vec![
            "ご注文商品：",
            "商品名テスト:3個",
            "---------------------------------------------",
        ];
        let items = extract_items(&lines);
        assert_eq!(items[0].quantity, 3);
    }

    #[test]
    fn test_extract_items_skips_empty_lines() {
        let lines = vec![
            "ご注文商品：",
            "",
            "商品A:1個",
            "",
            "商品B:2個",
            "---------------------------------------------",
        ];
        let items = extract_items(&lines);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_extract_items_not_started() {
        // ご注文商品：セクションがない場合は空
        let lines = vec!["商品A:1個", "商品B:2個"];
        let items = extract_items(&lines);
        assert!(items.is_empty());
    }

    // ─── extract_subtotal ───

    #[test]
    fn test_extract_subtotal() {
        let lines = vec!["商品小計（税込）「6,158」円"];
        assert_eq!(extract_subtotal(&lines), Some(6158));
    }

    #[test]
    fn test_extract_subtotal_zero() {
        let lines = vec!["商品小計（税込）「0」円"];
        assert_eq!(extract_subtotal(&lines), Some(0));
    }

    // ─── extract_shipping_fee ───

    #[test]
    fn test_extract_shipping_fee() {
        let lines = vec!["送料(税込)「500」円"];
        assert_eq!(extract_shipping_fee(&lines), Some(500));
    }

    #[test]
    fn test_extract_shipping_fee_zero() {
        let lines = vec!["送料(税込)「0」円"];
        assert_eq!(extract_shipping_fee(&lines), Some(0));
    }

    // ─── extract_total_amount ───

    #[test]
    fn test_extract_total_amount() {
        let lines = vec!["ご注文金額合計（税込）「6,158」円"];
        assert_eq!(extract_total_amount(&lines), Some(6158));
    }

    // ─── extract_carrier ───

    #[test]
    fn test_extract_carrier() {
        let lines = vec!["配送会社：ゆうパケット"];
        assert_eq!(extract_carrier(&lines), Some("ゆうパケット".to_string()));
    }

    #[test]
    fn test_extract_carrier_not_found() {
        let lines = vec!["伝票番号：680156937342"];
        assert_eq!(extract_carrier(&lines), None);
    }

    // ─── extract_tracking_number ───

    #[test]
    fn test_extract_tracking_number() {
        let lines = vec!["伝票番号：680156937342"];
        assert_eq!(
            extract_tracking_number(&lines),
            Some("680156937342".to_string())
        );
    }

    #[test]
    fn test_extract_tracking_number_not_found() {
        let lines = vec!["配送会社：ゆうパケット"];
        assert_eq!(extract_tracking_number(&lines), None);
    }

    // ─── body_to_lines ───

    #[test]
    fn test_body_to_lines_trims_whitespace() {
        let body = "  ご注文番号：100409780  \n  ご注文日：2026-03-03 22:25:08  \n";
        let lines = body_to_lines(body);
        assert_eq!(lines[0], "ご注文番号：100409780");
        assert_eq!(lines[1], "ご注文日：2026-03-03 22:25:08");
    }
}
