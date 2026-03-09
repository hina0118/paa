use crate::parsers::OrderItem;
use once_cell::sync::Lazy;
use regex::Regex;

pub mod cancel;
pub mod confirm;
pub mod rakuten_confirm;
pub mod rakuten_send;
pub mod send;

// ─────────────────────────────────────────────────────────────────────────────
// 正規表現
// ─────────────────────────────────────────────────────────────────────────────

/// 楽天・直販 send 共通: `受注番号：739419973` パターン（9桁）
static ORDER_NUMBER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"受注番号[：:]\s*(\d{9})").expect("Invalid ORDER_NUMBER_RE"));

/// 直販 confirm: `受注番号 "219908570"` パターン（引用符付き9桁）
static ORDER_NUMBER_QUOTED_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"受注番号\s+"(\d{9})""#).expect("Invalid ORDER_NUMBER_QUOTED_RE"));

/// 楽天テーブル商品行: `商品名 | 1,411円 | 1 | 1,411円`
/// ヘッダー行（`|` 始まり）は除外し、商品行のみ抽出する。
static TABLE_ITEM_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(.+?)\s*\|\s*([\d,]+)円\s*\|\s*(\d+)\s*\|\s*([\d,]+)円\s*$")
        .expect("Invalid TABLE_ITEM_RE")
});

/// 楽天: `送料 : 500円` パターン
static SHIPPING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"送料\s*:\s*([\d,]+)円").expect("Invalid SHIPPING_RE"));

/// 楽天: `合計金額 : 2,961円` パターン
static TOTAL_RAKUTEN_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"合計金額\s*:\s*([\d,]+)円").expect("Invalid TOTAL_RAKUTEN_RE"));

/// 楽天・直販 send 共通: `荷物お問合せ番号：397404561713` パターン（12桁）
/// 「荷物受付番号」「荷物お問合せ番号」など表記ゆれに対応するため柔軟なパターンを使用。
static TRACKING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"荷物[^：:]*[：:]\s*(\d{12})").expect("Invalid TRACKING_RE"));

/// 直販 confirm 商品名: `商品名：xxx`
static DIRECT_ITEM_NAME_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^商品名[：:](.+)").expect("Invalid DIRECT_ITEM_NAME_RE"));

/// 直販 confirm 単価: `単価：\1,690`
static DIRECT_UNIT_PRICE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^単価[：:][\\¥]?([\d,]+)").expect("Invalid DIRECT_UNIT_PRICE_RE"));

/// 直販 confirm 個数: `個数：1`
static DIRECT_QUANTITY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^個数[：:](\d+)").expect("Invalid DIRECT_QUANTITY_RE"));

/// 直販 confirm 商品小計: `小計：\1,690`
static DIRECT_ITEM_SUBTOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^小計[：:][\\¥]?([\d,]+)").expect("Invalid DIRECT_ITEM_SUBTOTAL_RE"));

/// 直販 confirm 合計小計: `●小計　　　：\7,480`（複数スペースや全角スペースを含む）
/// プレフィックスは `■`/`◆`/`●` のいずれかを使用するメール形式がある。
static DIRECT_SUBTOTAL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[■◆●]?小計[\s　]*[：:]\s*[\\¥]?([\d,]+)").expect("Invalid DIRECT_SUBTOTAL_RE")
});

/// 直販 confirm 送料: `●送料　　　：\500`
static DIRECT_SHIPPING_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[■◆●]?送料[\s　]*[：:]\s*[\\¥]?([\d,]+)").expect("Invalid DIRECT_SHIPPING_RE")
});

/// 直販 confirm 合計: `●合計　　　：7,980円`
static DIRECT_TOTAL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[■◆●]?合計[\s　]*[：:]\s*[\\¥]?([\d,]+)").expect("Invalid DIRECT_TOTAL_RE")
});

// ─────────────────────────────────────────────────────────────────────────────
// 共通ヘルパー
// ─────────────────────────────────────────────────────────────────────────────

/// メール本文をトリム済み行リストに変換する
pub fn body_to_lines(body: &str) -> Vec<String> {
    body.lines().map(|l| l.trim().to_string()).collect()
}

/// `受注番号：739419973` 形式の注文番号を抽出する（楽天・直販 send 共通）
pub fn extract_order_number(lines: &[&str]) -> Option<String> {
    lines
        .iter()
        .find_map(|line| ORDER_NUMBER_RE.captures(line).map(|c| c[1].to_string()))
}

/// `受注番号 "219908570"` 形式の注文番号を抽出する（直販 confirm 専用）
pub fn extract_order_number_quoted(lines: &[&str]) -> Option<String> {
    lines.iter().find_map(|line| {
        ORDER_NUMBER_QUOTED_RE
            .captures(line)
            .map(|c| c[1].to_string())
    })
}

/// `荷物受付番号：397404561713` から12桁の送り状番号を抽出する
pub fn extract_tracking_number(lines: &[&str]) -> Option<String> {
    lines
        .iter()
        .find_map(|line| TRACKING_RE.captures(line).map(|c| c[1].to_string()))
}

/// 楽天テーブル形式の商品行を抽出する
///
/// フォーマット:
/// ```text
/// ＜ 商品名 | 単価 | 数量 | 金額 ＞  ← ヘッダー（`＜` 始まり、スキップ）
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━   ← セパレーター（U+2501）
/// 商品名A | 1,411円 | 1 | 1,411円   ← 商品行
/// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━   ← セパレーター（商品セクション終了）
/// ```
///
/// 1回目のセパレーター後から商品収集を開始し、2回目のセパレーターで終了する。
pub fn extract_rakuten_items(lines: &[&str]) -> Vec<OrderItem> {
    let mut items = Vec::new();
    let mut separator_count = 0u32;

    for line in lines {
        let trimmed = line.trim();

        if is_separator(trimmed) {
            separator_count += 1;
            // 2回目のセパレーターで商品セクション終了
            if separator_count >= 2 {
                break;
            }
            continue;
        }

        // 1回目のセパレーター通過後のみ商品行を収集
        if separator_count < 1 {
            continue;
        }

        // ヘッダー行（`|` 始まりまたは `＜` 始まり）はスキップ
        if trimmed.starts_with('|') || trimmed.starts_with('＜') {
            continue;
        }

        if let Some(caps) = TABLE_ITEM_RE.captures(trimmed) {
            let name = caps[1].trim().to_string();
            let unit_price: i64 = caps[2].replace(',', "").parse().unwrap_or(0);
            let quantity: i64 = caps[3].parse().unwrap_or(1);
            let subtotal: i64 = caps[4].replace(',', "").parse().unwrap_or(0);
            items.push(OrderItem {
                name,
                manufacturer: None,
                model_number: None,
                unit_price,
                quantity,
                subtotal,
                image_url: None,
            });
        }
    }

    items
}

/// 楽天形式の送料を抽出する: `送料 : 500円`
pub fn extract_shipping_fee(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        SHIPPING_RE
            .captures(line)
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// 楽天形式の合計金額を抽出する: `合計金額 : 2,961円`
pub fn extract_total_amount(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        TOTAL_RAKUTEN_RE
            .captures(line)
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// 直販 confirm の商品ブロックを抽出する
///
/// フォーマット（空行区切りのブロック）:
/// ```text
/// 商品名：商品A
/// 単価：\1,690
/// 個数：1
/// 小計：\1,690
///
/// 商品名：商品B
/// ...
/// ```
pub fn extract_direct_items(lines: &[&str]) -> Vec<OrderItem> {
    let mut items = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_unit_price: i64 = 0;
    let mut current_quantity: i64 = 1;

    for line in lines {
        let trimmed = line.trim();

        if let Some(caps) = DIRECT_ITEM_NAME_RE.captures(trimmed) {
            // 前のブロックが未確定なら破棄（不完全ブロック）
            current_name = Some(caps[1].trim().to_string());
            current_unit_price = 0;
            current_quantity = 1;
            continue;
        }

        if let Some(caps) = DIRECT_UNIT_PRICE_RE.captures(trimmed) {
            current_unit_price = caps[1].replace(',', "").parse().unwrap_or(0);
            continue;
        }

        if let Some(caps) = DIRECT_QUANTITY_RE.captures(trimmed) {
            current_quantity = caps[1].parse().unwrap_or(1);
            continue;
        }

        if let Some(caps) = DIRECT_ITEM_SUBTOTAL_RE.captures(trimmed) {
            if let Some(name) = current_name.take() {
                let subtotal: i64 = caps[1].replace(',', "").parse().unwrap_or(0);
                items.push(OrderItem {
                    name,
                    manufacturer: None,
                    model_number: None,
                    unit_price: current_unit_price,
                    quantity: current_quantity,
                    subtotal,
                    image_url: None,
                });
            }
            current_unit_price = 0;
            current_quantity = 1;
            continue;
        }
    }

    items
}

/// 直販 confirm の注文小計を抽出する: `●小計　　　：\7,480`
///
/// 商品ブロック内の `小計：\N` と区別するため、`■`/`◆`/`●` プレフィックスを持つ行のみマッチする。
pub fn extract_direct_subtotal(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        let trimmed = line.trim();
        // ■/◆/● プレフィックスがある行のみ (商品小計行との区別)
        if !trimmed.starts_with('■') && !trimmed.starts_with('◆') && !trimmed.starts_with('●')
        {
            return None;
        }
        DIRECT_SUBTOTAL_RE
            .captures(trimmed)
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// 直販 confirm の送料を抽出する: `●送料　　　：\500`
pub fn extract_direct_shipping_fee(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        let trimmed = line.trim();
        if !trimmed.starts_with('■') && !trimmed.starts_with('◆') && !trimmed.starts_with('●')
        {
            return None;
        }
        DIRECT_SHIPPING_RE
            .captures(trimmed)
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// 直販 confirm の合計金額を抽出する: `●合計　　　：7,980円`
pub fn extract_direct_total(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        let trimmed = line.trim();
        if !trimmed.starts_with('■') && !trimmed.starts_with('◆') && !trimmed.starts_with('●')
        {
            return None;
        }
        DIRECT_TOTAL_RE
            .captures(trimmed)
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// メール本文中の追跡 URL から配送会社を判定する
///
/// - `kuronekoyamato.co.jp` → ヤマト運輸
/// - `sagawa-exp.co.jp` → 佐川急便
pub fn detect_carrier(body: &str) -> Option<String> {
    if body.contains("kuronekoyamato.co.jp") {
        Some("ヤマト運輸".to_string())
    } else if body.contains("sagawa-exp.co.jp") {
        Some("佐川急便".to_string())
    } else {
        None
    }
}

/// 楽天テーブルのセパレーター行かどうかを判定する
///
/// 実際のメール本文で確認: `━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━` 形式
/// （U+2501 BOX DRAWINGS HEAVY HORIZONTAL の繰り返し）
fn is_separator(line: &str) -> bool {
    let trimmed = line.trim();
    // U+2501 (━) のみで構成された行をセパレーターと見なす（最低4文字以上）
    if trimmed.chars().count() < 4 {
        return false;
    }
    trimmed.chars().all(|c| c == '━')
}

// ─────────────────────────────────────────────────────────────────────────────
// テスト
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── extract_order_number ───

    #[test]
    fn test_extract_order_number_colon_full() {
        let lines = vec!["受注番号：739419973"];
        assert_eq!(extract_order_number(&lines), Some("739419973".to_string()));
    }

    #[test]
    fn test_extract_order_number_colon_half() {
        let lines = vec!["受注番号:739419973"];
        assert_eq!(extract_order_number(&lines), Some("739419973".to_string()));
    }

    #[test]
    fn test_extract_order_number_with_label_prefix() {
        // 本文中の `受注番号：739419973 でお承りしました` のような行
        let lines = vec!["お客様のご注文は受注番号：739419973 でお承りしました。"];
        assert_eq!(extract_order_number(&lines), Some("739419973".to_string()));
    }

    #[test]
    fn test_extract_order_number_not_found() {
        let lines = vec!["商品名：テスト商品"];
        assert_eq!(extract_order_number(&lines), None);
    }

    // ─── extract_order_number_quoted ───

    #[test]
    fn test_extract_order_number_quoted() {
        let lines = vec![r#"お客様のご注文は受注番号 "219908570"にて承りました。"#];
        assert_eq!(
            extract_order_number_quoted(&lines),
            Some("219908570".to_string())
        );
    }

    #[test]
    fn test_extract_order_number_quoted_not_found() {
        let lines = vec!["受注番号：219908570"];
        assert_eq!(extract_order_number_quoted(&lines), None);
    }

    // ─── extract_tracking_number ───

    #[test]
    fn test_extract_tracking_number() {
        // 実際のメール形式: 荷物お問合せ番号
        let lines = vec!["荷物お問合せ番号：397404561713"];
        assert_eq!(
            extract_tracking_number(&lines),
            Some("397404561713".to_string())
        );
    }

    #[test]
    fn test_extract_tracking_number_variant_label() {
        // 表記ゆれにも対応できること
        let lines = vec!["荷物受付番号：397404561713"];
        assert_eq!(
            extract_tracking_number(&lines),
            Some("397404561713".to_string())
        );
    }

    #[test]
    fn test_extract_tracking_number_not_found() {
        let lines = vec!["受注番号：739419973"];
        assert_eq!(extract_tracking_number(&lines), None);
    }

    // ─── extract_rakuten_items ───

    #[test]
    fn test_extract_rakuten_items_single() {
        let sep = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
        let lines = vec![
            "＜                     商品名              | 単価 | 数量 | 金額 ＞",
            sep,
            "30MM 1/144 eEXM-21 ラビオット [ネイビー] プラモデル | 1,411円 | 1 | 1,411円",
            sep,
        ];
        let items = extract_rakuten_items(&lines);
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].name,
            "30MM 1/144 eEXM-21 ラビオット [ネイビー] プラモデル"
        );
        assert_eq!(items[0].unit_price, 1411);
        assert_eq!(items[0].quantity, 1);
        assert_eq!(items[0].subtotal, 1411);
    }

    #[test]
    fn test_extract_rakuten_items_multiple() {
        let sep = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
        let lines = vec![
            "＜                     商品名              | 単価 | 数量 | 金額 ＞",
            sep,
            "商品A | 1,411円 | 1 | 1,411円",
            "商品B | 1,050円 | 2 | 2,100円",
            sep,
        ];
        let items = extract_rakuten_items(&lines);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "商品A");
        assert_eq!(items[1].name, "商品B");
        assert_eq!(items[1].unit_price, 1050);
        assert_eq!(items[1].quantity, 2);
        assert_eq!(items[1].subtotal, 2100);
    }

    #[test]
    fn test_extract_rakuten_items_header_skipped() {
        // ヘッダー行（＜ 始まり）は商品として抽出されない
        let sep = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
        let lines = vec![
            sep,
            "＜                     商品名              | 単価 | 数量 | 金額 ＞",
            "商品A | 500円 | 1 | 500円",
            sep,
        ];
        let items = extract_rakuten_items(&lines);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "商品A");
    }

    // ─── extract_shipping_fee ───

    #[test]
    fn test_extract_shipping_fee() {
        let lines = vec!["送料 : 500円"];
        assert_eq!(extract_shipping_fee(&lines), Some(500));
    }

    #[test]
    fn test_extract_shipping_fee_no_space() {
        let lines = vec!["送料:500円"];
        assert_eq!(extract_shipping_fee(&lines), Some(500));
    }

    // ─── extract_total_amount ───

    #[test]
    fn test_extract_total_amount() {
        let lines = vec!["合計金額 : 2,961円"];
        assert_eq!(extract_total_amount(&lines), Some(2961));
    }

    // ─── extract_direct_items ───

    #[test]
    fn test_extract_direct_items_single() {
        let lines = vec![
            "商品名：テスト商品A",
            "単価：\\1,690",
            "個数：1",
            "小計：\\1,690",
        ];
        let items = extract_direct_items(&lines);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "テスト商品A");
        assert_eq!(items[0].unit_price, 1690);
        assert_eq!(items[0].quantity, 1);
        assert_eq!(items[0].subtotal, 1690);
    }

    #[test]
    fn test_extract_direct_items_multiple() {
        let lines = vec![
            "商品名：商品A",
            "単価：\\980",
            "個数：2",
            "小計：\\1,960",
            "",
            "商品名：商品B",
            "単価：\\3,630",
            "個数：1",
            "小計：\\3,630",
        ];
        let items = extract_direct_items(&lines);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "商品A");
        assert_eq!(items[0].unit_price, 980);
        assert_eq!(items[0].quantity, 2);
        assert_eq!(items[0].subtotal, 1960);
        assert_eq!(items[1].name, "商品B");
        assert_eq!(items[1].unit_price, 3630);
    }

    // ─── extract_direct_subtotal / shipping / total ───

    #[test]
    fn test_extract_direct_subtotal_with_bullet_kuro() {
        let lines = vec!["■小計　　　　　：\\7,480"];
        assert_eq!(extract_direct_subtotal(&lines), Some(7480));
    }

    #[test]
    fn test_extract_direct_subtotal_with_bullet_maru() {
        // 実際のメール形式: ● プレフィックス
        let lines = vec!["●小計　　　　　：\\7,480"];
        assert_eq!(extract_direct_subtotal(&lines), Some(7480));
    }

    #[test]
    fn test_extract_direct_subtotal_ignores_item_subtotal() {
        // 商品ブロック内の `小計：` はプレフィックスなしなので無視される
        let lines = vec!["小計：\\1,690"];
        assert_eq!(extract_direct_subtotal(&lines), None);
    }

    #[test]
    fn test_extract_direct_shipping_fee_kuro() {
        let lines = vec!["■送料　　　　　：\\500"];
        assert_eq!(extract_direct_shipping_fee(&lines), Some(500));
    }

    #[test]
    fn test_extract_direct_shipping_fee_maru() {
        // 実際のメール形式: ● プレフィックス
        let lines = vec!["●送料　　　　　：\\500"];
        assert_eq!(extract_direct_shipping_fee(&lines), Some(500));
    }

    #[test]
    fn test_extract_direct_total_kuro() {
        let lines = vec!["■合計　　　　　：7,980円"];
        assert_eq!(extract_direct_total(&lines), Some(7980));
    }

    #[test]
    fn test_extract_direct_total_maru() {
        // 実際のメール形式: ● プレフィックス
        let lines = vec!["●合計　　　　　：7,980円"];
        assert_eq!(extract_direct_total(&lines), Some(7980));
    }

    // ─── detect_carrier ───

    #[test]
    fn test_detect_carrier_yamato() {
        let body = "荷物受付番号：397404561713\nhttp://www.kuronekoyamato.co.jp/top.html";
        assert_eq!(detect_carrier(body), Some("ヤマト運輸".to_string()));
    }

    #[test]
    fn test_detect_carrier_sagawa() {
        let body = "荷物受付番号：515596488142\nhttps://www.sagawa-exp.co.jp/";
        assert_eq!(detect_carrier(body), Some("佐川急便".to_string()));
    }

    #[test]
    fn test_detect_carrier_unknown() {
        let body = "荷物受付番号：123456789012\nhttp://example.com/";
        assert_eq!(detect_carrier(body), None);
    }

    // ─── is_separator ───

    #[test]
    fn test_is_separator_heavy_horizontal() {
        // 実際のメール形式: U+2501 (━) の繰り返し
        assert!(is_separator("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    }

    #[test]
    fn test_is_separator_short_not_separator() {
        assert!(!is_separator("━━"));
    }

    #[test]
    fn test_is_separator_product_line_not_separator() {
        assert!(!is_separator("商品A | 1,411円 | 1 | 1,411円"));
    }

    // ─── body_to_lines ───

    #[test]
    fn test_body_to_lines_trims() {
        let body = "  受注番号：739419973  \n  送料 : 500円  \n";
        let lines = body_to_lines(body);
        assert_eq!(lines[0], "受注番号：739419973");
        assert_eq!(lines[1], "送料 : 500円");
    }
}
