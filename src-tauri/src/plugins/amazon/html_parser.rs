//! Amazon.co.jp 注文詳細ページ HTML パーサー
//!
//! URL: `https://www.amazon.co.jp/your-orders/order-details?orderID=XXX-XXXXXXX-XXXXXXX`
//!
//! # 抽出対象
//! - 商品一覧（商品名・単価・数量）
//! - 小計・送料・注文合計

use scraper::{ElementRef, Html, Selector};

use crate::parsers::{OrderInfo, OrderItem};

// ─────────────────────────────────────────────────────────────────────────────
// パース関数
// ─────────────────────────────────────────────────────────────────────────────

/// Amazon 注文詳細 HTML をパースして `OrderInfo` を返す
///
/// `order_number` は URL の `orderID` パラメータから呼び出し元で抽出して渡す。
pub fn parse_order_detail_html(html: &str, order_number: &str) -> Result<OrderInfo, String> {
    let document = Html::parse_document(html);

    let items = extract_items(&document);
    let subtotal = extract_subtotal(&document);
    let shipping_fee = extract_shipping_fee(&document);
    let total_amount = extract_total_amount(&document);

    Ok(OrderInfo {
        order_number: order_number.to_string(),
        order_date: None,
        delivery_address: None,
        delivery_info: None,
        items,
        subtotal,
        shipping_fee,
        total_amount,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// 商品リスト抽出
// ─────────────────────────────────────────────────────────────────────────────

fn extract_items(document: &Html) -> Vec<OrderItem> {
    // ① .yohtmlc-item ベースで試みる（現行フォーマット）
    let items = try_extract_yohtmlc_items(document);
    if !items.is_empty() {
        return items;
    }

    // ② .a-fixed-left-grid ベースで試みる（代替フォーマット）
    try_extract_grid_items(document)
}

fn try_extract_yohtmlc_items(document: &Html) -> Vec<OrderItem> {
    let Ok(item_sel) = Selector::parse(".yohtmlc-item") else {
        return vec![];
    };

    let mut items = Vec::new();

    for item_el in document.select(&item_sel) {
        let name = el_text_by_sel(item_el, ".yohtmlc-product-title")
            .or_else(|| el_text_by_sel(item_el, "a.a-link-normal"))
            .unwrap_or_default();

        if name.is_empty() {
            continue;
        }

        let quantity = el_quantity(item_el);
        let unit_price = el_price(item_el).unwrap_or(0);

        items.push(OrderItem {
            name,
            manufacturer: None,
            model_number: None,
            unit_price,
            quantity,
            subtotal: unit_price * quantity,
            image_url: None,
        });
    }

    items
}

fn try_extract_grid_items(document: &Html) -> Vec<OrderItem> {
    let Ok(grid_sel) = Selector::parse(".a-fixed-left-grid") else {
        return vec![];
    };
    let Ok(right_sel) = Selector::parse(".a-fixed-left-grid-col.a-col-right") else {
        return vec![];
    };

    let mut items = Vec::new();

    for grid_el in document.select(&grid_sel) {
        let Some(right_el) = grid_el.select(&right_sel).next() else {
            continue;
        };

        let name = el_text_by_sel(right_el, ".a-size-medium.a-color-base.a-text-normal")
            .or_else(|| el_text_by_sel(right_el, ".a-size-medium"))
            .or_else(|| el_text_by_sel(right_el, "a.a-link-normal"))
            .unwrap_or_default();

        if name.is_empty() {
            continue;
        }

        let quantity = el_quantity(right_el);
        let unit_price = el_price(right_el).unwrap_or(0);

        items.push(OrderItem {
            name,
            manufacturer: None,
            model_number: None,
            unit_price,
            quantity,
            subtotal: unit_price * quantity,
            image_url: None,
        });
    }

    items
}

// ─────────────────────────────────────────────────────────────────────────────
// 金額抽出（サマリーセクション）
// ─────────────────────────────────────────────────────────────────────────────

fn extract_subtotal(document: &Html) -> Option<i64> {
    extract_summary_amount(document, "小計")
        .or_else(|| extract_summary_amount(document, "商品の小計"))
}

fn extract_shipping_fee(document: &Html) -> Option<i64> {
    extract_summary_amount(document, "配送料・手数料")
        .or_else(|| extract_summary_amount(document, "配送料"))
}

fn extract_total_amount(document: &Html) -> Option<i64> {
    // .grand-total-price を優先
    if let Ok(sel) = Selector::parse(".grand-total-price .a-price .a-offscreen") {
        if let Some(el) = document.select(&sel).next() {
            let text = el.text().collect::<String>();
            if let Some(v) = parse_yen_amount(&text) {
                return Some(v);
            }
        }
    }

    extract_summary_amount(document, "注文合計")
        .or_else(|| extract_summary_amount(document, "合計"))
}

/// サマリー行でラベルを含む `.a-row` を探し、その行内の金額を返す
fn extract_summary_amount(document: &Html, label: &str) -> Option<i64> {
    let Ok(row_sel) = Selector::parse(".a-row") else {
        return None;
    };

    for row_el in document.select(&row_sel) {
        let text = row_el.text().collect::<String>();
        if text.contains(label) {
            if let Some(v) = find_yen_in_text(&text) {
                return Some(v);
            }
        }
    }

    None
}

// ─────────────────────────────────────────────────────────────────────────────
// ElementRef ヘルパー
// ─────────────────────────────────────────────────────────────────────────────

/// 子要素をセレクタで検索し、最初にマッチした要素のトリム済みテキストを返す
fn el_text_by_sel(parent: ElementRef<'_>, selector_str: &str) -> Option<String> {
    let sel = Selector::parse(selector_str).ok()?;
    let text = parent
        .select(&sel)
        .next()?
        .text()
        .collect::<String>()
        .trim()
        .to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// 要素内テキストから数量を抽出する（デフォルト 1）
fn el_quantity(el: ElementRef<'_>) -> i64 {
    let text = el.text().collect::<String>();
    parse_quantity_from_text(&text).unwrap_or(1)
}

/// 要素内から価格（円）を抽出する
fn el_price(el: ElementRef<'_>) -> Option<i64> {
    // .a-price .a-offscreen（スクリーンリーダー向け正確金額テキスト）
    if let Ok(sel) = Selector::parse(".a-price .a-offscreen") {
        if let Some(price_el) = el.select(&sel).next() {
            let text = price_el.text().collect::<String>();
            if let Some(v) = parse_yen_amount(&text) {
                return Some(v);
            }
        }
    }

    // .a-color-price テキスト
    if let Ok(sel) = Selector::parse(".a-color-price") {
        if let Some(price_el) = el.select(&sel).next() {
            let text = price_el.text().collect::<String>();
            if let Some(v) = parse_yen_amount(&text) {
                return Some(v);
            }
        }
    }

    // フォールバック: 要素全体から最初の ￥/¥ を探す
    let text = el.text().collect::<String>();
    find_yen_in_text(&text)
}

// ─────────────────────────────────────────────────────────────────────────────
// テキスト解析ヘルパー
// ─────────────────────────────────────────────────────────────────────────────

/// `￥1,234` / `¥5,678` 形式の文字列から金額（i64）を返す
fn parse_yen_amount(text: &str) -> Option<i64> {
    let trimmed = text.trim();
    let after_yen = trimmed
        .strip_prefix('￥')
        .or_else(|| trimmed.strip_prefix('¥'))?;

    let clean: String = after_yen
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .collect::<String>()
        .replace(',', "");

    clean.parse::<i64>().ok()
}

/// テキスト内で最初に見つかった `￥N,NNN` / `¥N,NNN` 形式の金額を返す
fn find_yen_in_text(text: &str) -> Option<i64> {
    for ch in ['￥', '¥'] {
        if let Some(pos) = text.find(ch) {
            let after = &text[pos + ch.len_utf8()..];
            let digits: String = after
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == ',')
                .collect();
            if !digits.is_empty() {
                let clean = digits.replace(',', "");
                if let Ok(n) = clean.parse::<i64>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

/// テキストから数量を抽出する
fn parse_quantity_from_text(text: &str) -> Option<i64> {
    // 「数量：N」または「数量: N」
    for prefix in ["数量：", "数量:"] {
        if let Some(pos) = text.find(prefix) {
            let after = &text[pos + prefix.len()..];
            let num_str: String = after
                .chars()
                .skip_while(|c| c.is_whitespace())
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(n) = num_str.parse::<i64>() {
                return Some(n);
            }
        }
    }

    // 「（N個）」
    if let Some(pos) = text.find('（') {
        let after = &text[pos + '（'.len_utf8()..];
        let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = num_str.parse::<i64>() {
            return Some(n);
        }
    }

    None
}

// ─────────────────────────────────────────────────────────────────────────────
// テスト
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_yen_amount_fullwidth() {
        assert_eq!(parse_yen_amount("￥1,234"), Some(1234));
    }

    #[test]
    fn test_parse_yen_amount_halfwidth() {
        assert_eq!(parse_yen_amount("¥5,678"), Some(5678));
    }

    #[test]
    fn test_parse_yen_amount_no_comma() {
        assert_eq!(parse_yen_amount("￥100"), Some(100));
    }

    #[test]
    fn test_parse_yen_amount_whitespace() {
        assert_eq!(parse_yen_amount("  ￥2,500  "), Some(2500));
    }

    #[test]
    fn test_parse_yen_amount_invalid() {
        assert_eq!(parse_yen_amount("abc"), None);
        assert_eq!(parse_yen_amount(""), None);
    }

    #[test]
    fn test_find_yen_in_text() {
        assert_eq!(find_yen_in_text("合計 ￥2,500"), Some(2500));
        assert_eq!(find_yen_in_text("送料 ¥350 税込"), Some(350));
    }

    #[test]
    fn test_parse_quantity_colon() {
        assert_eq!(parse_quantity_from_text("数量：3"), Some(3));
        assert_eq!(parse_quantity_from_text("数量: 2"), Some(2));
    }

    #[test]
    fn test_parse_quantity_kakko() {
        assert_eq!(parse_quantity_from_text("（1個）"), Some(1));
    }

    #[test]
    fn test_parse_quantity_none() {
        assert_eq!(parse_quantity_from_text("no quantity"), None);
    }

    #[test]
    fn test_parse_order_detail_html_empty() {
        let html = "<html><body></body></html>";
        let result = parse_order_detail_html(html, "123-4567890-1234567");
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.order_number, "123-4567890-1234567");
        assert!(info.items.is_empty());
        assert!(info.subtotal.is_none());
        assert!(info.total_amount.is_none());
    }

    #[test]
    fn test_parse_order_detail_html_yohtmlc_item() {
        let html = r#"
        <html><body>
          <div class="yohtmlc-item">
            <span class="yohtmlc-product-title">テスト商品A</span>
            <span>数量：2</span>
            <span class="a-price"><span class="a-offscreen">￥1,200</span></span>
          </div>
        </body></html>
        "#;
        let result = parse_order_detail_html(html, "111-2222222-3333333");
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.items.len(), 1);
        assert_eq!(info.items[0].name, "テスト商品A");
        assert_eq!(info.items[0].quantity, 2);
        assert_eq!(info.items[0].unit_price, 1200);
    }

    #[test]
    fn test_parse_order_detail_html_total() {
        let html = r#"
        <html><body>
          <div class="a-row">小計 ￥3,000</div>
          <div class="a-row">配送料 ￥500</div>
          <div class="a-row">注文合計 ￥3,500</div>
        </body></html>
        "#;
        let result = parse_order_detail_html(html, "999-8888888-7777777");
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.subtotal, Some(3000));
        assert_eq!(info.shipping_fee, Some(500));
        assert_eq!(info.total_amount, Some(3500));
    }
}
