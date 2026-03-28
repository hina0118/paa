use crate::parsers::OrderItem;
use once_cell::sync::Lazy;
use regex::Regex;

pub mod confirm;

/// `【オーダーID】HJ20260321_051302_88` パターン
static ORDER_NUMBER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"【オーダーID】(.+)").expect("Invalid ORDER_NUMBER_RE"));

/// `【ご注文日】2026年03月21日` パターン
static ORDER_DATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"【ご注文日】(\d{4})年(\d{2})月(\d{2})日").expect("Invalid ORDER_DATE_RE")
});

/// 商品名行: `　1.GALHolic深青`（行頭に全角/半角スペース + 番号ドット商品名）
static ITEM_NAME_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*\d+\.(.+)$").expect("Invalid ITEM_NAME_RE"));

/// 価格行: `　　価格：￥7,880 x 数量：1 = 合計：￥7,880`
static ITEM_PRICE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"価格：[￥¥]([\d,]+) x 数量：(\d+) = 合計：[￥¥]([\d,]+)")
        .expect("Invalid ITEM_PRICE_RE")
});

/// `　商品金額合計：￥7,880` パターン
static SUBTOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"商品金額合計：[￥¥]([\d,]+)").expect("Invalid SUBTOTAL_RE"));

/// `　送料：￥700` パターン
static SHIPPING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"送料：[￥¥]([\d,]+)").expect("Invalid SHIPPING_RE"));

/// `　注文金額合計：￥8,580` パターン
static TOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"注文金額合計：[￥¥]([\d,]+)").expect("Invalid TOTAL_RE"));

/// メール本文をテキスト行のリストに変換する
pub fn body_to_lines(body: &str) -> Vec<String> {
    body.lines().map(|l| l.to_string()).collect()
}

/// `【オーダーID】HJ20260321_051302_88` から注文番号を抽出する
pub fn extract_order_number(lines: &[&str]) -> Option<String> {
    lines.iter().find_map(|line| {
        ORDER_NUMBER_RE
            .captures(line)
            .map(|c| c[1].trim().to_string())
    })
}

/// `【ご注文日】2026年03月21日` から注文日を抽出し `2026-03-21` 形式で返す
pub fn extract_order_date(lines: &[&str]) -> Option<String> {
    lines.iter().find_map(|line| {
        ORDER_DATE_RE
            .captures(line)
            .map(|c| format!("{}-{}-{}", &c[1], &c[2], &c[3]))
    })
}

/// `【ご注文品】` セクション以降の商品行を抽出する
///
/// - 商品名行: `　N.商品名`
/// - 価格行（商品名行の直後）: `　　価格：￥unit_price x 数量：qty = 合計：￥subtotal`
/// - セクション終了: `【` で始まる次のセクション
pub fn extract_items(lines: &[&str]) -> Vec<OrderItem> {
    let mut items: Vec<OrderItem> = Vec::new();
    let mut in_section = false;
    let mut pending_name: Option<String> = None;

    for line in lines {
        if line.contains("【ご注文品】") {
            in_section = true;
            continue;
        }

        if !in_section {
            continue;
        }

        // 次のセクション（【...】）でセクション終了
        let trimmed = line.trim();
        if trimmed.starts_with('【') && trimmed.ends_with('】') {
            break;
        }
        // 区切り行でもセクション終了
        if trimmed.starts_with("--") {
            break;
        }

        // 価格行を先にチェック（pending_name がある場合）
        if let Some(ref name) = pending_name.clone() {
            if let Some(caps) = ITEM_PRICE_RE.captures(line) {
                let unit_price: i64 = caps[1].replace(',', "").parse().unwrap_or(0);
                let quantity: i64 = caps[2].parse().unwrap_or(1);
                let subtotal: i64 = caps[3].replace(',', "").parse().unwrap_or(0);
                items.push(OrderItem {
                    name: name.clone(),
                    manufacturer: None,
                    model_number: None,
                    unit_price,
                    quantity,
                    subtotal,
                    image_url: None,
                });
                pending_name = None;
                continue;
            }
        }

        // 商品名行
        if let Some(caps) = ITEM_NAME_RE.captures(line) {
            pending_name = Some(caps[1].trim().to_string());
        }
    }

    items
}

/// `　商品金額合計：￥7,880` から小計を抽出する
pub fn extract_subtotal(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        SUBTOTAL_RE
            .captures(line)
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// `　送料：￥700` から送料を抽出する
pub fn extract_shipping_fee(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        SHIPPING_RE
            .captures(line)
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// `　注文金額合計：￥8,580` から合計金額を抽出する
pub fn extract_total_amount(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        TOTAL_RE
            .captures(line)
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_lines() -> Vec<&'static str> {
        vec![
            "【オーダーID】HJ20260321_051302_88",
            "【ご注文日】2026年03月21日",
            "【ご注文者】山田　太郎　様",
            "【ご注文品】",
            "\u{3000}1.GALHolic深青",
            "\u{3000}\u{3000}価格：\u{FFE5}7,880 x 数量：1 = 合計：\u{FFE5}7,880",
            "【お買上げ金額】",
            "\u{3000}商品金額合計：\u{FFE5}7,880",
            "\u{3000}送料：\u{FFE5}700",
            "\u{3000}手数料：\u{FFE5}0",
            "\u{3000}注文金額合計：\u{FFE5}8,580",
        ]
    }

    #[test]
    fn test_extract_order_number() {
        let lines = sample_lines();
        assert_eq!(
            extract_order_number(&lines),
            Some("HJ20260321_051302_88".to_string())
        );
    }

    #[test]
    fn test_extract_order_date() {
        let lines = sample_lines();
        assert_eq!(extract_order_date(&lines), Some("2026-03-21".to_string()));
    }

    #[test]
    fn test_extract_items() {
        let lines = sample_lines();
        let items = extract_items(&lines);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "GALHolic深青");
        assert_eq!(items[0].unit_price, 7880);
        assert_eq!(items[0].quantity, 1);
        assert_eq!(items[0].subtotal, 7880);
    }

    #[test]
    fn test_extract_subtotal() {
        let lines = sample_lines();
        assert_eq!(extract_subtotal(&lines), Some(7880));
    }

    #[test]
    fn test_extract_shipping_fee() {
        let lines = sample_lines();
        assert_eq!(extract_shipping_fee(&lines), Some(700));
    }

    #[test]
    fn test_extract_total_amount() {
        let lines = sample_lines();
        assert_eq!(extract_total_amount(&lines), Some(8580));
    }

    #[test]
    fn test_extract_order_number_not_found() {
        let lines = vec!["ご注文日：2026-03-21"];
        assert_eq!(extract_order_number(&lines), None);
    }
}
