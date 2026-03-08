use crate::parsers::OrderItem;
use once_cell::sync::Lazy;
use regex::Regex;

pub mod confirm;
pub mod send;

/// `商品名: 商品名称` パターン
static ITEM_NAME_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^商品名:\s*(.+)").expect("Invalid ITEM_NAME_RE"));

/// `数量:1 個` パターン
static QUANTITY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^数量:(\d+)\s*個").expect("Invalid QUANTITY_RE"));

/// `単価:3,000円(税込)` パターン
static UNIT_PRICE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^単価:([\d,]+)円").expect("Invalid UNIT_PRICE_RE"));

/// `商品合計額:3,000円(税込)` パターン（各商品の小計）
static ITEM_SUBTOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^商品合計額:([\d,]+)円").expect("Invalid ITEM_SUBTOTAL_RE"));

/// `商品合計:8,000円(税込)` パターン（●合計セクションの合計）
///
/// `商品合計額:` とは末尾の「額」有無で区別できる。`^商品合計:` は `商品合計額:` にはマッチしない。
static SUBTOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^商品合計:([\d,]+)円").expect("Invalid SUBTOTAL_RE"));

/// `送料:594円(税込)` パターン
static SHIPPING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^送料:([\d,]+)円").expect("Invalid SHIPPING_RE"));

/// `合計額:8,594円(税込)` パターン
static TOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^合計額:([\d,]+)円").expect("Invalid TOTAL_RE"));

/// メール本文をテキスト行のリストに変換する（プレーンテキスト専用）
///
/// アニメイト通販のメールはプレーンテキスト形式のため、HTML 変換は不要。
/// 各行をトリムして返す。
pub fn body_to_lines(body: &str) -> Vec<String> {
    body.lines().map(|l| l.trim().to_string()).collect()
}

/// 注文番号を抽出する
///
/// ラベル行（`注文番号` で終わる行: `●ご注文番号` / `ご注文番号` / `注文番号` 等）の
/// 次行に 8 桁の数字が続く形式を想定する。
/// `ends_with("注文番号")` による判定で `●` 有無や `ご` 有無の表記揺れに対応する。
pub fn extract_order_number(lines: &[&str]) -> Option<String> {
    for (i, line) in lines.iter().enumerate() {
        if line.trim().ends_with("注文番号") {
            if let Some(&next) = lines.get(i + 1) {
                let n = next.trim();
                if n.len() == 8 && n.chars().all(|c| c.is_ascii_digit()) {
                    return Some(n.to_string());
                }
            }
        }
    }
    None
}

/// 送り状番号を抽出する
///
/// ラベル行（`送り状番号` で終わる行: `●送り状番号` / `送り状番号` 等）の
/// 次行に 12 桁の数字が続く形式を想定する。
/// `ends_with("送り状番号")` による判定で `●` 有無の表記揺れに対応する。
pub fn extract_tracking_number(lines: &[&str]) -> Option<String> {
    for (i, line) in lines.iter().enumerate() {
        if line.trim().ends_with("送り状番号") {
            if let Some(&next) = lines.get(i + 1) {
                let n = next.trim();
                if n.len() == 12 && n.chars().all(|c| c.is_ascii_digit()) {
                    return Some(n.to_string());
                }
            }
        }
    }
    None
}

/// `●ご注文内容` セクションから商品リストを抽出する
///
/// `商品名:` 行で商品ブロック開始、`商品合計額:` 行で商品を確定する。
/// `=============` はブロック区切り、`支払方法：` または `●合計` でセクション終了。
pub fn extract_items(lines: &[&str]) -> Vec<OrderItem> {
    let mut items: Vec<OrderItem> = Vec::new();
    let mut in_content_section = false;
    let mut current_name: Option<String> = None;
    let mut current_quantity: i64 = 1;
    let mut current_unit_price: i64 = 0;

    for line in lines {
        let trimmed = line.trim();

        if trimmed == "●ご注文内容" {
            in_content_section = true;
            continue;
        }

        if !in_content_section {
            continue;
        }

        // セクション終了
        if trimmed.starts_with("●合計") || trimmed.starts_with("支払方法") {
            // 未確定の商品があれば確定（単価のみあって商品合計額がないケース対応）
            if let Some(name) = current_name.take() {
                items.push(OrderItem {
                    name,
                    manufacturer: None,
                    model_number: None,
                    unit_price: current_unit_price,
                    quantity: current_quantity,
                    subtotal: current_unit_price * current_quantity,
                    image_url: None,
                });
            }
            break;
        }

        if let Some(caps) = ITEM_NAME_RE.captures(trimmed) {
            // 前の商品が商品合計額なしで確定していない場合は破棄（不完全ブロック）
            current_name = Some(caps[1].trim().to_string());
            current_quantity = 1;
            current_unit_price = 0;
            continue;
        }

        if let Some(caps) = QUANTITY_RE.captures(trimmed) {
            current_quantity = caps[1].parse().unwrap_or(1);
            continue;
        }

        if let Some(caps) = UNIT_PRICE_RE.captures(trimmed) {
            current_unit_price = caps[1].replace(',', "").parse().unwrap_or(0);
            continue;
        }

        if let Some(caps) = ITEM_SUBTOTAL_RE.captures(trimmed) {
            if let Some(name) = current_name.take() {
                let subtotal: i64 = caps[1].replace(',', "").parse().unwrap_or(0);
                let unit_price = if current_unit_price > 0 {
                    current_unit_price
                } else if current_quantity > 0 {
                    subtotal / current_quantity
                } else {
                    subtotal
                };
                items.push(OrderItem {
                    name,
                    manufacturer: None,
                    model_number: None,
                    unit_price,
                    quantity: current_quantity,
                    subtotal,
                    image_url: None,
                });
                current_quantity = 1;
                current_unit_price = 0;
            }
            continue;
        }

        // `=============` はブロック区切り（スキップ）
        if trimmed.starts_with("=====") {
            continue;
        }
    }

    items
}

/// `商品合計:8,000円(税込)` から合計商品金額を抽出する（●合計セクション）
pub fn extract_subtotal(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        SUBTOTAL_RE
            .captures(line.trim())
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// `送料:594円(税込)` から送料を抽出する
pub fn extract_shipping_fee(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        SHIPPING_RE
            .captures(line.trim())
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// `合計額:8,594円(税込)` から合計金額を抽出する
pub fn extract_total_amount(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        TOTAL_RE
            .captures(line.trim())
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── extract_order_number ───

    #[test]
    fn test_extract_order_number_with_bullet() {
        let lines = vec!["●ご注文番号", "28928446", "ご注文番号は大切に保管してください。"];
        assert_eq!(extract_order_number(&lines), Some("28928446".to_string()));
    }

    #[test]
    fn test_extract_order_number_without_bullet() {
        // 「●」なしの表記揺れに対応
        let lines = vec!["ご注文番号", "28928446"];
        assert_eq!(extract_order_number(&lines), Some("28928446".to_string()));
    }

    #[test]
    fn test_extract_order_number_without_go_prefix() {
        // 「ご」なしの表記揺れに対応
        let lines = vec!["注文番号", "28928446"];
        assert_eq!(extract_order_number(&lines), Some("28928446".to_string()));
    }

    #[test]
    fn test_extract_order_number_wrong_digit_count_rejected() {
        // 8桁未満の数字は注文番号として採用しない
        let lines = vec!["●ご注文番号", "1234567"];
        assert_eq!(extract_order_number(&lines), None);
    }

    #[test]
    fn test_extract_order_number_nine_digits_rejected() {
        // 9桁は採用しない
        let lines = vec!["●ご注文番号", "123456789"];
        assert_eq!(extract_order_number(&lines), None);
    }

    #[test]
    fn test_extract_order_number_sentence_line_not_label() {
        // 「注文番号は大切に〜」のような文章行はラベルとして扱わない
        let lines = vec!["ご注文番号は大切に保管してください。", "28928446"];
        assert_eq!(extract_order_number(&lines), None);
    }

    #[test]
    fn test_extract_order_number_not_found() {
        let lines = vec!["注文確認です", "28928446"];
        assert_eq!(extract_order_number(&lines), None);
    }

    // ─── extract_tracking_number ───

    #[test]
    fn test_extract_tracking_number_with_bullet() {
        let lines = vec!["●送り状番号", "217565803081"];
        assert_eq!(
            extract_tracking_number(&lines),
            Some("217565803081".to_string())
        );
    }

    #[test]
    fn test_extract_tracking_number_without_bullet() {
        // 「●」なしの表記揺れに対応
        let lines = vec!["送り状番号", "217565803081"];
        assert_eq!(
            extract_tracking_number(&lines),
            Some("217565803081".to_string())
        );
    }

    #[test]
    fn test_extract_tracking_number_wrong_digit_count_rejected() {
        // 12桁未満は採用しない
        let lines = vec!["●送り状番号", "12345678901"];
        assert_eq!(extract_tracking_number(&lines), None);
    }

    #[test]
    fn test_extract_tracking_number_thirteen_digits_rejected() {
        // 13桁は採用しない
        let lines = vec!["●送り状番号", "2175658030812"];
        assert_eq!(extract_tracking_number(&lines), None);
    }

    #[test]
    fn test_extract_tracking_number_not_found() {
        let lines = vec!["●送り状番号", "abc"];
        assert_eq!(extract_tracking_number(&lines), None);
    }

    #[test]
    fn test_extract_items_single() {
        let lines = vec![
            "●ご注文内容",
            "商品名: テスト商品A",
            "数量:1 個",
            "単価:3,000円(税込)",
            "発売日:2022年02月 中 発売予定",
            "商品合計額:3,000円(税込)",
            "支払方法：クレジット",
        ];
        let items = extract_items(&lines);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "テスト商品A");
        assert_eq!(items[0].quantity, 1);
        assert_eq!(items[0].unit_price, 3000);
        assert_eq!(items[0].subtotal, 3000);
    }

    #[test]
    fn test_extract_items_multiple() {
        let lines = vec![
            "●ご注文内容",
            "商品名: 商品A",
            "数量:1 個",
            "単価:3,000円(税込)",
            "商品合計額:3,000円(税込)",
            "=============",
            "商品名: 商品B",
            "数量:2 個",
            "単価:5,000円(税込)",
            "商品合計額:10,000円(税込)",
            "支払方法：クレジット",
        ];
        let items = extract_items(&lines);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "商品A");
        assert_eq!(items[0].subtotal, 3000);
        assert_eq!(items[1].name, "商品B");
        assert_eq!(items[1].quantity, 2);
        assert_eq!(items[1].subtotal, 10000);
    }

    #[test]
    fn test_extract_items_name_trimmed() {
        // 商品名末尾のスペースはトリムされる
        let lines = vec![
            "●ご注文内容",
            "商品名: 【フィギュア】テスト商品  ",
            "数量:1 個",
            "単価:3,000円(税込)",
            "商品合計額:3,000円(税込)",
            "●合計",
        ];
        let items = extract_items(&lines);
        assert_eq!(items[0].name, "【フィギュア】テスト商品");
    }

    #[test]
    fn test_extract_subtotal() {
        let lines = vec!["商品合計:8,000円(税込)"];
        assert_eq!(extract_subtotal(&lines), Some(8000));
    }

    #[test]
    fn test_extract_subtotal_does_not_match_item_subtotal() {
        // 商品合計額: は商品合計: にマッチしない
        let lines = vec!["商品合計額:3,000円(税込)"];
        assert_eq!(extract_subtotal(&lines), None);
    }

    #[test]
    fn test_extract_shipping_fee() {
        let lines = vec!["送料:594円(税込)"];
        assert_eq!(extract_shipping_fee(&lines), Some(594));
    }

    #[test]
    fn test_extract_shipping_fee_zero() {
        let lines = vec!["送料:0円(税込)"];
        assert_eq!(extract_shipping_fee(&lines), Some(0));
    }

    #[test]
    fn test_extract_total_amount() {
        let lines = vec!["合計額:8,594円(税込)"];
        assert_eq!(extract_total_amount(&lines), Some(8594));
    }

    #[test]
    fn test_body_to_lines_trims_whitespace() {
        let body = "  ●ご注文番号  \n  28928446  \n";
        let lines = body_to_lines(body);
        assert_eq!(lines[0], "●ご注文番号");
        assert_eq!(lines[1], "28928446");
    }
}
