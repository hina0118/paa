use crate::parsers::OrderItem;
use once_cell::sync::Lazy;
use regex::Regex;

pub mod confirm;
pub mod send;

// ─────────────────────────────────────────────────────────────────────────────
// 正規表現
// ─────────────────────────────────────────────────────────────────────────────

/// 取引番号: `取引番号:S2204166697` または `取引番号：S2204166697`（confirm / send 共通）
static ORDER_NUMBER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"取引番号[：:]\s*(S\d{10})").expect("Invalid ORDER_NUMBER_RE"));

/// confirm 商品行: `1-1 \1,656 商品名 (603103980001)`
/// 行番号・価格・商品名+末尾コードの3グループを取得する
static ITEM_LINE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(\d+-\d+)\s+\\([\d,]+)\s+(.+)$").expect("Invalid ITEM_LINE_RE"));

/// 商品名末尾のコード除去: `商品名 [5055732] (603103980001)` → `商品名`
/// `[\w+]` 形式の任意ラベルと `(\d+)` 形式のコードを末尾から除去する
static TRAILING_CODE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\s*(?:\[[^\]]+\]\s*)?\(\d+\)\s*$").expect("Invalid TRAILING_CODE_RE")
});

/// send 追跡番号: `お問い合わせ番号　　：764336939516`
///
/// 複数口の場合は `お問い合わせ番号1　 ：764337098000` / `お問い合わせ番号2　 ：764337098033` の形式になる。
/// 末尾の数字を省略可能にして先頭の追跡番号を返す。
static TRACKING_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"お問い合わせ番号\d*[\s　]*[：:]\s*(\d{12})").expect("Invalid TRACKING_RE")
});

/// send 配送会社: `お届け方法　　　　　：ゆうパック（日本郵便)`
static CARRIER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"お届け方法[\s　]*[：:]\s*(.+)").expect("Invalid CARRIER_RE"));

/// send 出荷日: `出荷日　　　　　　　：2022/04/27`
static SHIP_DATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"出荷日[\s　]*[：:]\s*(\d{4}/\d{2}/\d{2})").expect("Invalid SHIP_DATE_RE")
});

/// send 商品合計: `商品合計　　　　　　：\9,108`
static SUBTOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^商品合計[\s　]*[：:]\s*\\([\d,]+)").expect("Invalid SUBTOTAL_RE"));

/// send 送料: `送料　　　　　　　　：\0`
///
/// ゆうメールでは `送料・通信販売手数料：\0` の形式になるため、
/// `送料` と `：` の間は任意の文字を許容する。
/// 「代引手数料」と区別するため行頭アンカーを使用する。
static SHIPPING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^送料[^：:]*[：:]\s*\\([\d,]+)").expect("Invalid SHIPPING_RE"));

/// send 支払合計金額: `支払合計金額　　　　：\9,108`
static TOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^支払合計金額[\s　]*[：:]\s*\\([\d,]+)").expect("Invalid TOTAL_RE"));

// ─────────────────────────────────────────────────────────────────────────────
// 共通ヘルパー
// ─────────────────────────────────────────────────────────────────────────────

/// メール本文をトリム済み行リストに変換する
pub fn body_to_lines(body: &str) -> Vec<String> {
    body.lines().map(|l| l.trim().to_string()).collect()
}

/// `取引番号:S2204166697` 形式の取引番号を抽出する（confirm / send 共通）
pub fn extract_order_number(lines: &[&str]) -> Option<String> {
    lines
        .iter()
        .find_map(|line| ORDER_NUMBER_RE.captures(line).map(|c| c[1].to_string()))
}

/// confirm 商品行リストを抽出する
///
/// フォーマット:
/// ```text
/// 1-1 \1,656 中古プラモデル 1/144 HG グレイズアイン ... (603103980001)
/// 1-8 \1,288 ... 「機動戦士ガンダム00」 [5055732] (603101318001)
/// ```
/// - 価格はバックスラッシュ（JIS 円記号）の直後
/// - 末尾の `(\d+)` は商品コード → 除去する
/// - 末尾に `[\w+]` 形式のラベルがある場合も除去する
/// - 数量は常に 1
pub fn extract_items(lines: &[&str]) -> Vec<OrderItem> {
    let mut items = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        let Some(caps) = ITEM_LINE_RE.captures(trimmed) else {
            continue;
        };

        let unit_price: i64 = caps[2].replace(',', "").parse().unwrap_or(0);
        if unit_price <= 0 {
            continue;
        }

        let raw_name = &caps[3];
        let name = TRAILING_CODE_RE.replace(raw_name, "").trim().to_string();
        if name.is_empty() {
            continue;
        }

        items.push(OrderItem {
            name,
            manufacturer: None,
            model_number: None,
            unit_price,
            quantity: 1,
            subtotal: unit_price,
            image_url: None,
        });
    }

    items
}

/// send 追跡番号を抽出する: `お問い合わせ番号　　：764336939516`（12桁）
pub fn extract_tracking_number(lines: &[&str]) -> Option<String> {
    lines
        .iter()
        .find_map(|line| TRACKING_RE.captures(line).map(|c| c[1].to_string()))
}

/// send 配送会社を抽出して正規化する: `お届け方法　　　　　：ゆうパック（日本郵便)`
///
/// `（日本郵便)` の半角閉じ括弧を全角に正規化する（ゆうパック・ゆうメール 共通）。
pub fn extract_carrier(lines: &[&str]) -> Option<String> {
    lines.iter().find_map(|line| {
        CARRIER_RE.captures(line).map(|c| {
            // 半角閉じ括弧 `)` を全角 `）` に統一する
            c[1].trim().replace("（日本郵便)", "（日本郵便）")
        })
    })
}

/// send 出荷日を抽出して DB 保存形式に変換する: `2022/04/27` → `2022-04-27 00:00:00`
pub fn extract_ship_date(lines: &[&str]) -> Option<String> {
    lines.iter().find_map(|line| {
        SHIP_DATE_RE.captures(line).map(|c| {
            let date_slash = &c[1]; // "2022/04/27"
            format!("{} 00:00:00", date_slash.replace('/', "-"))
        })
    })
}

/// send 商品合計を抽出する: `商品合計　　　　　　：\9,108`
pub fn extract_subtotal(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        SUBTOTAL_RE
            .captures(line.trim())
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// send 送料を抽出する: `送料　　　　　　　　：\0`
pub fn extract_shipping_fee(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        SHIPPING_RE
            .captures(line.trim())
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// send 支払合計金額を抽出する: `支払合計金額　　　　：\9,108`
pub fn extract_total_amount(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        TOTAL_RE
            .captures(line.trim())
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// テスト
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── extract_order_number ───

    #[test]
    fn test_extract_order_number_confirm_format() {
        let lines = vec!["取引番号:S2204166697"];
        assert_eq!(
            extract_order_number(&lines),
            Some("S2204166697".to_string())
        );
    }

    #[test]
    fn test_extract_order_number_send_format() {
        // send: `（取引番号：S2204166697）`
        let lines = vec!["山田太郎様 （取引番号：S2204166697）"];
        assert_eq!(
            extract_order_number(&lines),
            Some("S2204166697".to_string())
        );
    }

    #[test]
    fn test_extract_order_number_not_found() {
        let lines = vec!["商品名：テスト商品"];
        assert_eq!(extract_order_number(&lines), None);
    }

    // ─── extract_items ───

    #[test]
    fn test_extract_items_basic() {
        let lines = vec!["1-1 \\1,656 中古プラモデル 1/144 HG グレイズアイン (603103980001)"];
        let items = extract_items(&lines);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].unit_price, 1656);
        assert_eq!(items[0].quantity, 1);
        assert_eq!(items[0].subtotal, 1656);
        assert_eq!(items[0].name, "中古プラモデル 1/144 HG グレイズアイン");
    }

    #[test]
    fn test_extract_items_with_bracket_label() {
        // `[5055732]` が商品名末尾に含まれるケース (item 1-8)
        let lines = vec!["1-8 \\1,288 中古プラモデル オーガンダム [5055732] (603101318001)"];
        let items = extract_items(&lines);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "中古プラモデル オーガンダム");
        assert_eq!(items[0].unit_price, 1288);
    }

    #[test]
    fn test_extract_items_with_paren_in_name() {
        // 商品名内に括弧が含まれるケース: `オーガンダム(実戦配備型)`
        let lines = vec![
            "1-4 \\368 長距離狙撃用オプションアーマー(アルト用/ダークグレー) 「30 MINUTES MISSIONS」 (603100256001)",
        ];
        let items = extract_items(&lines);
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].name,
            "長距離狙撃用オプションアーマー(アルト用/ダークグレー) 「30 MINUTES MISSIONS」"
        );
        assert_eq!(items[0].unit_price, 368);
    }

    #[test]
    fn test_extract_items_multiple() {
        let lines = vec![
            "1-1 \\1,656 商品A (600000000001)",
            "1-2 \\828 商品B (600000000002)",
        ];
        let items = extract_items(&lines);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "商品A");
        assert_eq!(items[1].name, "商品B");
    }

    #[test]
    fn test_extract_items_non_item_line_ignored() {
        // セパレーター・ヘッダー行は無視される
        let lines = vec!["──────────────────────────────────", "取引番号:S2204166697"];
        assert!(extract_items(&lines).is_empty());
    }

    // ─── extract_tracking_number ───

    #[test]
    fn test_extract_tracking_number() {
        let lines = vec!["お問い合わせ番号　　：764336939516"];
        assert_eq!(
            extract_tracking_number(&lines),
            Some("764336939516".to_string())
        );
    }

    #[test]
    fn test_extract_tracking_number_multi_parcel() {
        // 複数口の場合: `お問い合わせ番号1` / `お問い合わせ番号2` 形式
        let lines = vec![
            "お問い合わせ番号1　 ：764337098000",
            "お問い合わせ番号2　 ：764337098033",
        ];
        // 先頭の追跡番号を返す
        assert_eq!(
            extract_tracking_number(&lines),
            Some("764337098000".to_string())
        );
    }

    #[test]
    fn test_extract_tracking_number_not_found() {
        let lines = vec!["取引番号：S2204166697"];
        assert_eq!(extract_tracking_number(&lines), None);
    }

    // ─── extract_carrier ───

    #[test]
    fn test_extract_carrier_yupack() {
        // 半角閉じ括弧が含まれる実際のメール形式
        let lines = vec!["お届け方法　　　　　：ゆうパック（日本郵便)"];
        assert_eq!(
            extract_carrier(&lines),
            Some("ゆうパック（日本郵便）".to_string())
        );
    }

    #[test]
    fn test_extract_carrier_yumail() {
        // ゆうメール（追跡番号なし）の配送会社
        let lines = vec!["お届け方法　　　　　：ゆうメール（日本郵便)"];
        assert_eq!(
            extract_carrier(&lines),
            Some("ゆうメール（日本郵便）".to_string())
        );
    }

    #[test]
    fn test_extract_carrier_not_found() {
        let lines = vec!["取引番号：S2204166697"];
        assert_eq!(extract_carrier(&lines), None);
    }

    // ─── extract_ship_date ───

    #[test]
    fn test_extract_ship_date() {
        let lines = vec!["出荷日　　　　　　　：2022/04/27"];
        assert_eq!(
            extract_ship_date(&lines),
            Some("2022-04-27 00:00:00".to_string())
        );
    }

    #[test]
    fn test_extract_ship_date_not_found() {
        let lines = vec!["取引番号：S2204166697"];
        assert_eq!(extract_ship_date(&lines), None);
    }

    // ─── extract_subtotal / shipping / total ───

    #[test]
    fn test_extract_subtotal() {
        let lines = vec!["商品合計　　　　　　：\\9,108"];
        assert_eq!(extract_subtotal(&lines), Some(9108));
    }

    #[test]
    fn test_extract_shipping_fee_zero() {
        let lines = vec!["送料　　　　　　　　：\\0"];
        assert_eq!(extract_shipping_fee(&lines), Some(0));
    }

    #[test]
    fn test_extract_shipping_fee_yumail_format() {
        // ゆうメール形式: `送料・通信販売手数料：\0`
        let lines = vec!["送料・通信販売手数料：\\0"];
        assert_eq!(extract_shipping_fee(&lines), Some(0));
    }

    #[test]
    fn test_extract_shipping_fee_ignores_codash() {
        // 「代引手数料」は「送料」とマッチしないこと
        let lines = vec!["代引手数料　　　　　：\\0"];
        assert_eq!(extract_shipping_fee(&lines), None);
    }

    #[test]
    fn test_extract_total_amount() {
        let lines = vec!["支払合計金額　　　　：\\9,108"];
        assert_eq!(extract_total_amount(&lines), Some(9108));
    }

    // ─── body_to_lines ───

    #[test]
    fn test_body_to_lines_trims() {
        let body = "  取引番号:S2204166697  \n  出荷日：2022/04/27  \n";
        let lines = body_to_lines(body);
        assert_eq!(lines[0], "取引番号:S2204166697");
        assert_eq!(lines[1], "出荷日：2022/04/27");
    }
}
