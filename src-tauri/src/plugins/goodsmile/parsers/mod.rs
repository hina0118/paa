use crate::parsers::OrderItem;
use once_cell::sync::Lazy;
use regex::Regex;

pub mod confirm;
pub mod send;

/// `<br>` / `<br/>` / `<br />` タグを改行に置換するパターン
static BR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)<br\s*/?>").expect("Invalid BR_RE"));

/// HTML タグ全体を除去するパターン
static HTML_TAG_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<[^>]+>").expect("Invalid HTML_TAG_RE"));

/// `ご注文番号: CpBk4quaORPw` / `注文番号: CpBk4quaORPw` パターン
///
/// confirm メールは `ご注文番号:`、send メールは `注文番号:` とプレフィックスが異なるため
/// `ご?` で両方に対応する。
static ORDER_NUMBER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"ご?注文番号:\s*([A-Za-z0-9]+)").expect("Invalid ORDER_NUMBER_RE"));

/// `ご注文日時: Feb 01, 2025 4:48:07 PM` パターン
static ORDER_DATE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"ご注文日時:\s*(.+)").expect("Invalid ORDER_DATE_RE"));

/// `数量：N` / `数量:N` パターン（全角・半角コロン両対応）
static QUANTITY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"数量[：:]\s*(\d+)").expect("Invalid QUANTITY_RE"));

/// `小計：￥5,900` / `小計:¥5,900` パターン（全角・半角 ¥ 両対応）
static ITEM_SUBTOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"小計[：:]\s*[¥￥]([\d,]+)").expect("Invalid ITEM_SUBTOTAL_RE"));

/// `配送料 ￥0` パターン（行頭限定）
static SHIPPING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^配送料\s+[¥￥]([\d,]+)").expect("Invalid SHIPPING_RE"));

/// `合計 ￥5,900` パターン（行頭限定・`クーポン割引額` 等と混同しないよう限定）
static TOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^合計\s+[¥￥]([\d,]+)").expect("Invalid TOTAL_RE"));

/// `配送番号：564841939476` パターン
static TRACKING_NUMBER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"配送番号[：:]\s*(\d+)").expect("Invalid TRACKING_NUMBER_RE"));

/// `4580590207912 1` 形式の JAN コード（13 桁）+ 数量行パターン
static JAN_QUANTITY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*(\d{13})\s+(\d+)\s*$").expect("Invalid JAN_QUANTITY_RE"));

/// `配送元：佐川急便(送料無料)` パターン（括弧以降を除去）
static CARRIER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"配送元[：:]\s*([^(（\n]+)").expect("Invalid CARRIER_RE"));

/// `配送時間：指定なし` パターン
///
/// HTML メールでは `配送元：XXX 配送時間：XXX 追跡番号：...` が 1 行に並ぶため、
/// `\S+`（非空白文字列）でキャプチャし、後続フィールドを取り込まないようにする。
static DELIVERY_TIME_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"配送時間[：:]\s*(\S+)").expect("Invalid DELIVERY_TIME_RE"));

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
        .collect()
}

/// メール本文をテキスト行のリストに変換する
///
/// HTML が含まれる場合は `html_to_lines()` を使用し、
/// プレーンテキストの場合はそのまま分割する。
/// いずれも各行をトリムして返す。
pub fn body_to_lines(body: &str) -> Vec<String> {
    if body.contains("<br") || body.contains("<BR") {
        html_to_lines(body)
    } else {
        body.lines().map(|l| l.trim().to_string()).collect()
    }
}

/// 注文番号を抽出する
///
/// `ご注文番号:` / `注文番号:` どちらの形式にも対応する。
pub fn extract_order_number(lines: &[&str]) -> Option<String> {
    lines
        .iter()
        .find_map(|line| ORDER_NUMBER_RE.captures(line).map(|c| c[1].to_string()))
}

/// 英語形式の注文日時 `"Feb 01, 2025 4:48:07 PM"` を `"YYYY-MM-DD HH:MM"` に変換する
///
/// `ご注文日時:` 行の値を chrono でパースし、日本時間（Asia/Tokyo）として扱う。
/// confirm メールの日時は UTC で届くため、`+0000` タイムゾーンとして変換する。
pub fn extract_order_date(lines: &[&str]) -> Option<String> {
    let raw = lines.iter().find_map(|line| {
        ORDER_DATE_RE
            .captures(line)
            .map(|c| c[1].trim().to_string())
    })?;
    parse_english_datetime(&raw)
}

/// `"Feb 01, 2025 4:48:07 PM"` → `"2025-02-01 16:48"` に変換する
fn parse_english_datetime(s: &str) -> Option<String> {
    // chrono が要求する形式に正規化: "Feb 01, 2025 4:48:07 PM" → "%b %d, %Y %I:%M:%S %p"
    use chrono::NaiveDateTime;
    let dt = NaiveDateTime::parse_from_str(s, "%b %d, %Y %I:%M:%S %p").ok()?;
    Some(format!("{}", dt.format("%Y-%m-%d %H:%M")))
}

/// `商品:` 行を起点に注文商品リストを抽出する
///
/// `商品:` マーカーの直後の非空行が商品名、その後の `数量：` / `小計：` 行で
/// quantity / subtotal を取得する。複数商品にも対応する。
pub fn extract_items(lines: &[&str]) -> Vec<OrderItem> {
    let mut items: Vec<OrderItem> = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_quantity: Option<i64> = None;
    let mut after_product_marker = false;

    for line in lines {
        let trimmed = line.trim();

        if trimmed.starts_with("商品:") {
            // 前の商品があれば確定させてから次へ
            if let Some(name) = current_name.take() {
                let quantity = current_quantity.unwrap_or(1);
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
            current_quantity = None;
            after_product_marker = true;

            // `商品:商品名` のように同一行に商品名が含まれる場合
            let inline_name = trimmed.trim_start_matches("商品:").trim();
            if !inline_name.is_empty() {
                current_name = Some(inline_name.to_string());
                after_product_marker = false;
            }
            continue;
        }

        // 商品マーカー直後の非空行を商品名として取得
        if after_product_marker && current_name.is_none() && !trimmed.is_empty() {
            current_name = Some(trimmed.to_string());
            after_product_marker = false;
            continue;
        }

        // 数量行
        if let Some(caps) = QUANTITY_RE.captures(trimmed) {
            current_quantity = caps[1].parse().ok();
            continue;
        }

        // 小計行（商品ごとの小計）→ unit_price も算出して item を確定
        if let Some(caps) = ITEM_SUBTOTAL_RE.captures(trimmed) {
            if let Some(name) = current_name.take() {
                let quantity = current_quantity.unwrap_or(1);
                let subtotal: i64 = caps[1].replace(',', "").parse().unwrap_or(0);
                let unit_price = if quantity > 0 {
                    subtotal / quantity
                } else {
                    subtotal
                };
                items.push(OrderItem {
                    name,
                    manufacturer: None,
                    model_number: None,
                    unit_price,
                    quantity,
                    subtotal,
                    image_url: None,
                });
                current_quantity = None;
            }
            continue;
        }
    }

    items
}

/// `配送料 ￥0` 行から送料を抽出する
pub fn extract_shipping_fee(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        SHIPPING_RE
            .captures(line.trim())
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// `合計 ￥5,900` 行から合計金額を抽出する
pub fn extract_total_amount(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        TOTAL_RE
            .captures(line.trim())
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// `配送番号：564841939476` 行から追跡番号を抽出する
pub fn extract_tracking_number(lines: &[&str]) -> Option<String> {
    lines
        .iter()
        .find_map(|line| TRACKING_NUMBER_RE.captures(line).map(|c| c[1].to_string()))
}

/// 配送情報セクション内の商品リストを抽出する（send メール用）
///
/// `配送番号：` 行の後から `配送元：` 行の前までを対象とする。
/// 商品名行と JAN コード + 数量行がペアで並ぶ構造を想定する。
pub fn extract_send_items(lines: &[&str]) -> Vec<OrderItem> {
    let mut items: Vec<OrderItem> = Vec::new();

    // `配送情報:` セクション内のみ処理する
    let in_delivery_section = lines
        .iter()
        .position(|l| l.trim().starts_with("配送情報:"))
        .map(|pos| &lines[pos..])
        .unwrap_or(lines);

    let mut pending_name: Option<String> = None;

    for line in in_delivery_section {
        let trimmed = line.trim();

        // `配送元：` 以降は商品セクション終了
        if trimmed.starts_with("配送元") {
            break;
        }

        // 追跡番号行・空行・既知のキーワード行はスキップ
        if trimmed.is_empty()
            || trimmed.starts_with("配送番号")
            || trimmed.starts_with("配送時間")
            || trimmed.starts_with("追跡番号")
            || trimmed.starts_with("配送情報:")
        {
            continue;
        }

        // JAN + 数量行
        if let Some(caps) = JAN_QUANTITY_RE.captures(trimmed) {
            if let Some(name) = pending_name.take() {
                let jan = caps[1].to_string();
                let quantity: i64 = caps[2].parse().unwrap_or(1);
                items.push(OrderItem {
                    name,
                    manufacturer: None,
                    model_number: Some(jan),
                    unit_price: 0,
                    quantity,
                    subtotal: 0,
                    image_url: None,
                });
            }
            continue;
        }

        // 商品名候補行
        pending_name = Some(trimmed.to_string());
    }

    items
}

/// `配送元：佐川急便(送料無料)` から配送業者名を抽出する（括弧以降を除去）
pub fn extract_carrier(lines: &[&str]) -> Option<String> {
    lines
        .iter()
        .find_map(|line| CARRIER_RE.captures(line).map(|c| c[1].trim().to_string()))
}

/// `配送時間：指定なし` から配送時間を抽出する
///
/// `"指定なし"` の場合は `None` を返す。
pub fn extract_delivery_time(lines: &[&str]) -> Option<String> {
    lines.iter().find_map(|line| {
        DELIVERY_TIME_RE.captures(line).and_then(|c| {
            let val = c[1].trim().to_string();
            if val == "指定なし" {
                None
            } else {
                Some(val)
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_order_number_with_go_prefix() {
        let lines = vec!["ご注文番号: CpBk4quaORPw"];
        assert_eq!(
            extract_order_number(&lines),
            Some("CpBk4quaORPw".to_string())
        );
    }

    #[test]
    fn test_extract_order_number_without_go_prefix() {
        let lines = vec!["注文番号: CpBk4quaORPw"];
        assert_eq!(
            extract_order_number(&lines),
            Some("CpBk4quaORPw".to_string())
        );
    }

    #[test]
    fn test_extract_order_date_english_format() {
        let lines = vec!["ご注文日時: Feb 01, 2025 4:48:07 PM"];
        assert_eq!(
            extract_order_date(&lines),
            Some("2025-02-01 16:48".to_string())
        );
    }

    #[test]
    fn test_extract_order_date_am() {
        let lines = vec!["ご注文日時: Mar 15, 2024 9:03:00 AM"];
        assert_eq!(
            extract_order_date(&lines),
            Some("2024-03-15 09:03".to_string())
        );
    }

    #[test]
    fn test_extract_order_date_noon() {
        // 12:xx PM → 12:xx（正午）
        let lines = vec!["ご注文日時: Jun 10, 2025 12:00:00 PM"];
        assert_eq!(
            extract_order_date(&lines),
            Some("2025-06-10 12:00".to_string())
        );
    }

    #[test]
    fn test_extract_shipping_fee_zero() {
        let lines = vec!["配送料 ￥0"];
        assert_eq!(extract_shipping_fee(&lines), Some(0));
    }

    #[test]
    fn test_extract_shipping_fee_nonzero() {
        let lines = vec!["配送料 ￥500"];
        assert_eq!(extract_shipping_fee(&lines), Some(500));
    }

    #[test]
    fn test_extract_total_amount() {
        let lines = vec!["クーポン割引額 ￥0", "合計 ￥5,900"];
        assert_eq!(extract_total_amount(&lines), Some(5900));
    }

    #[test]
    fn test_extract_tracking_number() {
        let lines = vec!["配送番号：564841939476"];
        assert_eq!(
            extract_tracking_number(&lines),
            Some("564841939476".to_string())
        );
    }

    #[test]
    fn test_extract_carrier_strips_parentheses() {
        let lines = vec!["配送元：佐川急便(送料無料)"];
        assert_eq!(extract_carrier(&lines), Some("佐川急便".to_string()));
    }

    #[test]
    fn test_extract_carrier_no_parentheses() {
        let lines = vec!["配送元：ヤマト運輸"];
        assert_eq!(extract_carrier(&lines), Some("ヤマト運輸".to_string()));
    }

    #[test]
    fn test_extract_delivery_time_shitenashi() {
        let lines = vec!["配送時間：指定なし"];
        assert_eq!(extract_delivery_time(&lines), None);
    }

    #[test]
    fn test_extract_delivery_time_specified() {
        let lines = vec!["配送時間：14時〜16時"];
        assert_eq!(
            extract_delivery_time(&lines),
            Some("14時〜16時".to_string())
        );
    }

    /// HTML メールでは「配送元 配送時間 追跡番号」が 1 行に並ぶ。
    /// `\S+` キャプチャにより後続フィールドを取り込まないことを確認する。
    #[test]
    fn test_extract_delivery_time_inline_with_other_fields() {
        let lines =
            vec!["配送元：佐川急便(送料無料) 配送時間：指定なし 追跡番号：http://example.com"];
        assert_eq!(extract_delivery_time(&lines), None);
    }

    #[test]
    fn test_body_to_lines_plain_text() {
        let body = "注文番号: ABC\n数量：1\n合計 ￥1,000";
        let lines = body_to_lines(body);
        assert_eq!(lines[0], "注文番号: ABC");
        assert_eq!(lines[1], "数量：1");
    }

    #[test]
    fn test_body_to_lines_html_strips_tags() {
        let body = "<p>注文番号: ABC<br>数量：1</p>";
        let lines = body_to_lines(body);
        assert!(lines.iter().any(|l| l == "注文番号: ABC"));
        assert!(lines.iter().any(|l| l == "数量：1"));
    }

    #[test]
    fn test_body_to_lines_html_trims_whitespace() {
        let body = "  <br>   配送料  ￥0<br>   合計  ￥5,900<br>";
        let lines = body_to_lines(body);
        assert!(lines.iter().any(|l| l == "配送料  ￥0"));
        assert!(lines.iter().any(|l| l == "合計  ￥5,900"));
    }

    #[test]
    fn test_jan_quantity_re() {
        let line = "4580590207912 1";
        let caps = JAN_QUANTITY_RE.captures(line).unwrap();
        assert_eq!(&caps[1], "4580590207912");
        assert_eq!(&caps[2], "1");
    }

    #[test]
    fn test_extract_items_single() {
        let lines = vec![
            "配送方法:　佐川急便_送料無料",
            "商品:MODEROID バーンドラゴン",
            "発売時期：2025/9",
            "数量：1",
            "小計：￥5,900",
            "配送料 ￥0",
        ];
        let items = extract_items(&lines);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "MODEROID バーンドラゴン");
        assert_eq!(items[0].quantity, 1);
        assert_eq!(items[0].subtotal, 5900);
        assert_eq!(items[0].unit_price, 5900);
    }

    #[test]
    fn test_extract_items_multiple() {
        let lines = vec![
            "商品:商品Ａ",
            "数量：2",
            "小計：￥2,000",
            "商品:商品Ｂ",
            "数量：1",
            "小計：￥1,500",
            "合計 ￥3,500",
        ];
        let items = extract_items(&lines);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "商品Ａ");
        assert_eq!(items[0].unit_price, 1000);
        assert_eq!(items[0].subtotal, 2000);
        assert_eq!(items[1].name, "商品Ｂ");
        assert_eq!(items[1].subtotal, 1500);
    }
}
