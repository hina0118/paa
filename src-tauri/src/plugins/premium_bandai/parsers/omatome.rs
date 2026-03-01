//! プレミアムバンダイ ご注文おまとめ完了メール用パーサー
//!
//! 件名：`ご注文おまとめ完了のお知らせ`
//! 送信元：`evidence_bc@p-bandai.jp`
//!
//! フォーマット: text/plain のみ
//! 1通のメールに複数商品（複数注文を束ねたもの）が含まれる。
//! 結果は 1 つの `OrderInfo` に全商品をまとめて返す。
//!
//! ディスパッチ時に `apply_change_items_in_tx` で元注文の商品を商品名マッチングで削除する。

use super::{
    body_to_lines, extract_order_date, extract_order_number, extract_payment_fee,
    extract_shipping_fee, extract_total_amount, normalize_product_name, parse_price,
};
use crate::parsers::{EmailParser, OrderInfo, OrderItem};
use once_cell::sync::Lazy;
use regex::Regex;

/// `￥5,000（税込）×1 ￥5,000` のような価格 × 数量 行（`¥` 前置き形式）
static PRICE_QUANTITY_LINE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[¥￥]([\d,]+)(?:（税込）)?\s*×\s*(\d+)\s+[¥￥][\d,]+")
        .expect("Invalid PRICE_QUANTITY_LINE_RE")
});

/// `3,630円×1＝3,630円` / `1,980円&times;1＝1,980円` のような価格 × 数量 行（`円` 後置き形式）
///
/// - 価格: `3,630円`（`¥` なし、`円` 後置き）
/// - 数量: `×N`（Unicode 乗算記号 U+00D7）または `&times;N`（HTML エンティティ）
/// - 小計: `＝3,630円`（全角等号 U+FF1D または半角 `=`）
static PRICE_QUANTITY_YEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([\d,]+)円(?:×|&times;)(\d+)[＝=][\d,]+円$")
        .expect("Invalid PRICE_QUANTITY_YEN_RE")
});

/// プレミアムバンダイ おまとめ完了メール用パーサー
pub struct PremiumBandaiOmatomeParser;

impl EmailParser for PremiumBandaiOmatomeParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let body_lines = body_to_lines(email_body);
        let lines: Vec<&str> = body_lines.iter().map(|s| s.as_str()).collect();

        let order_number =
            extract_order_number(&lines).ok_or_else(|| "Order number not found".to_string())?;

        let order_date = extract_order_date(&lines);

        let items = extract_omatome_items(&lines);
        if items.is_empty() {
            return Err("No items found".to_string());
        }

        let subtotal: i64 = items.iter().map(|i| i.subtotal).sum();
        let subtotal = if subtotal > 0 { Some(subtotal) } else { None };

        let shipping_fee = extract_shipping_fee(&lines);
        let payment_fee = extract_payment_fee(&lines);
        let combined_fee = match (shipping_fee, payment_fee) {
            (Some(s), Some(p)) => Some(s + p),
            (Some(s), None) => Some(s),
            (None, Some(p)) => Some(p),
            (None, None) => None,
        };
        let total_amount = extract_total_amount(&lines);

        Ok(OrderInfo {
            order_number,
            order_date,
            delivery_address: None,
            delivery_info: None,
            items,
            subtotal,
            shipping_fee: combined_fee,
            total_amount,
        })
    }
}

/// おまとめメールの商品リストを抽出する
///
/// 以下の2フォーマットに対応する:
///
/// **フォーマット A（`¥` 前置き形式）:**
/// - セクション開始: `ご注文内容` / `注文内容`
/// - 価格行: `￥5,000（税込）×1 ￥5,000`
/// - セクション終了: `送料` / `合計` 等
///
/// **フォーマット B（`円` 後置き形式、実際のおまとめメール）:**
/// - セクション開始: `ご注文明細`
/// - 価格行: `3,630円×1＝3,630円`
/// - セクション終了: `【お買上金額】` 等
fn extract_omatome_items(lines: &[&str]) -> Vec<OrderItem> {
    let mut items: Vec<OrderItem> = Vec::new();
    let mut pending_name: Option<String> = None;
    let mut in_item_section = false;

    for line in lines {
        let trimmed = line.trim();

        // 注文内容セクション開始
        // フォーマット A: `ご注文内容` / `注文内容`
        // フォーマット B: `ご注文明細`（おまとめメール）
        // フォーマット C: `【注文明細】`（HTML テーブル形式、`ご` なし）
        // 複数回出現する場合（HTML テーブル形式）も都度リセットする
        if trimmed.contains("注文内容") || trimmed.contains("注文明細") {
            in_item_section = true;
            pending_name = None; // 再入時に未確定の商品名をクリア
            continue;
        }

        if !in_item_section {
            continue;
        }

        // 金額セクション開始で商品セクション終了
        if trimmed.contains("お買上金額")
            || trimmed.contains("ご購入金額")
            || trimmed.contains("支払金額") // 【支払金額】（HTML テーブル形式）
            || trimmed.starts_with("送料")
            || trimmed.starts_with("合計")
            || trimmed.starts_with("支払手数料")
            || trimmed.starts_with("代引手数料")
        {
            break;
        }

        // 区切り行・空行はスキップ
        if trimmed.is_empty()
            || trimmed.starts_with("---")
            || trimmed.starts_with("===")
            || trimmed.starts_with("－－")
        {
            continue;
        }

        // 価格×数量行 フォーマット B: `3,630円×1＝3,630円`
        if let Some(caps) = PRICE_QUANTITY_YEN_RE.captures(trimmed) {
            if let Some(name) = pending_name.take() {
                let unit_price: i64 = caps[1].replace(',', "").parse().unwrap_or(0);
                let quantity: i64 = caps[2].parse().unwrap_or(1);
                let subtotal = unit_price * quantity;
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
            continue;
        }

        // 価格×数量行 フォーマット A: `￥5,000（税込）×1 ￥5,000`
        if let Some(caps) = PRICE_QUANTITY_LINE_RE.captures(trimmed) {
            if let Some(name) = pending_name.take() {
                let unit_price: i64 = caps[1].replace(',', "").parse().unwrap_or(0);
                let quantity: i64 = caps[2].parse().unwrap_or(1);
                let subtotal = unit_price * quantity;
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
            continue;
        }

        // 注文番号・日付などのヘッダー行はスキップ
        if trimmed.starts_with("ご注文番号")
            || trimmed.starts_with("注文番号")
            || trimmed.starts_with("ご注文日")
            || trimmed.starts_with("注文日")
            || trimmed.starts_with("お支払")
            || trimmed.starts_with("【会員番号")
            || trimmed.starts_with("【お支払")
        {
            continue;
        }

        // 価格のみの行（`￥5,000（税込）`）→ pending_name と組み合わせて商品確定（数量1）
        if trimmed.starts_with('¥') || trimmed.starts_with('￥') {
            if let Some(name) = pending_name.take() {
                let unit_price = parse_price(trimmed).unwrap_or(0);
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
            continue;
        }

        // 上記いずれにも該当しない行 → 商品名候補
        if !trimmed.is_empty() {
            pending_name = Some(normalize_product_name(trimmed));
        }
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_omatome_inline_price() -> &'static str {
        r#"ご注文おまとめが完了しました。

■ご注文番号：12345
■ご注文日：2025年2月1日
■お支払方法：クレジットカード

■ご注文内容
--------------------------------------------
商品A【2025年4月発送】
￥5,000（税込）×1 ￥5,000
商品B【再販】
￥3,000（税込）×2 ￥6,000
--------------------------------------------
送料：￥0
支払手数料：￥0
合計：￥11,000
"#
    }

    fn sample_omatome_separate_price() -> &'static str {
        r#"■ご注文番号：12345
■ご注文日：2025年2月1日

■ご注文内容
商品C
￥8,000（税込）
商品D
￥4,000（税込）
送料：￥0
支払手数料：￥330
合計：￥12,330
"#
    }

    // ─── inline 価格形式（￥N×M ￥N）───

    #[test]
    fn test_parse_omatome_order_number() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_inline_price())
            .unwrap();
        assert_eq!(order.order_number, "12345");
    }

    #[test]
    fn test_parse_omatome_order_date() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_inline_price())
            .unwrap();
        assert_eq!(order.order_date, Some("2025-02-01".to_string()));
    }

    #[test]
    fn test_parse_omatome_item_count() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_inline_price())
            .unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_omatome_item_names_normalized() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_inline_price())
            .unwrap();
        assert_eq!(order.items[0].name, "商品A");
        assert_eq!(order.items[1].name, "商品B");
    }

    #[test]
    fn test_parse_omatome_item_prices_and_quantities() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_inline_price())
            .unwrap();
        assert_eq!(order.items[0].unit_price, 5000);
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[0].subtotal, 5000);
        assert_eq!(order.items[1].unit_price, 3000);
        assert_eq!(order.items[1].quantity, 2);
        assert_eq!(order.items[1].subtotal, 6000);
    }

    #[test]
    fn test_parse_omatome_amounts() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_inline_price())
            .unwrap();
        assert_eq!(order.subtotal, Some(11000));
        assert_eq!(order.shipping_fee, Some(0));
        assert_eq!(order.total_amount, Some(11000));
    }

    // ─── separate 価格形式（商品名の次行に価格）───

    #[test]
    fn test_parse_omatome_separate_price_item_count() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_separate_price())
            .unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_omatome_separate_price_names() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_separate_price())
            .unwrap();
        assert_eq!(order.items[0].name, "商品C");
        assert_eq!(order.items[1].name, "商品D");
    }

    #[test]
    fn test_parse_omatome_combined_fee() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_separate_price())
            .unwrap();
        // 送料0 + 支払手数料330 = 330
        assert_eq!(order.shipping_fee, Some(330));
    }

    /// 実際のプレミアムバンダイ おまとめメール（email 651 相当）
    ///
    /// - 注文番号: `【ご注文No.】` 形式
    /// - 注文日: ISO 形式 `2025-05-14`
    /// - 商品セクション: `【ご注文明細】`
    /// - 価格: `3,630円×1＝3,630円`（`円` 後置き）
    /// - 全角数字を含む商品名サフィックス: `【３次：２０２５年８月発送】`
    /// - 送料: `円` 後置き
    /// - 合計: `お支払い合計金額：　X円`
    fn sample_omatome_actual() -> &'static str {
        "＜！このメールには返信できません。ご注意ください！＞\n\
         このたびは「プレミアムバンダイ」をご利用いただき、まことにありがとうございます。\n\
         以下、お客様のご注文おまとめ内容の確認をお願い申しあげます。\n\
         ---------------------------------------------------------------------\n\
         【ご注文No.】\u{3000} 00130\n\
         【ご注文日】\u{3000}\u{3000}2025-05-14 12:04:59\n\
         【会員番号】\u{3000}\u{3000}5148038526\n\
         【お支払方法】\u{3000}クレジットカード(出荷時決済)\n\
         【ご注文明細】\n\
         ＨＧ 1/144 ジーライン・ライトアーマー【３次：２０２５年８月発送】\n\
         3,630円×1＝3,630円\n\
         ＨＧ 1/144 ガンダムプルトーネブラック【２次：２０２５年８月発送】\n\
         2,640円×1＝2,640円\n\
         【お買上金額】\n\
         商品金額：\u{3000}6,270円\n\
         送料：\u{3000}660円\n\
         決済手数料：\u{3000}0円\n\
         －－－－－－－－－－－－－－－－\n\
         お支払い合計金額：\u{3000}6,930円\n"
    }

    // ─── 実際のおまとめメール形式テスト ───

    #[test]
    fn test_parse_omatome_actual_order_number() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_actual())
            .unwrap();
        assert_eq!(order.order_number, "00130");
    }

    #[test]
    fn test_parse_omatome_actual_order_date() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_actual())
            .unwrap();
        assert_eq!(order.order_date, Some("2025-05-14".to_string()));
    }

    #[test]
    fn test_parse_omatome_actual_item_count() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_actual())
            .unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_omatome_actual_item_names_normalized() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_actual())
            .unwrap();
        // 全角数字サフィックス【３次：２０２５年８月発送】が除去される
        assert_eq!(order.items[0].name, "ＨＧ 1/144 ジーライン・ライトアーマー");
        assert_eq!(order.items[1].name, "ＨＧ 1/144 ガンダムプルトーネブラック");
    }

    #[test]
    fn test_parse_omatome_actual_item_prices() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_actual())
            .unwrap();
        assert_eq!(order.items[0].unit_price, 3630);
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[0].subtotal, 3630);
        assert_eq!(order.items[1].unit_price, 2640);
        assert_eq!(order.items[1].quantity, 1);
        assert_eq!(order.items[1].subtotal, 2640);
    }

    #[test]
    fn test_parse_omatome_actual_amounts() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_actual())
            .unwrap();
        assert_eq!(order.subtotal, Some(6270)); // items の合計
        assert_eq!(order.shipping_fee, Some(660)); // 送料（決済手数料 0 + 送料 660）
        assert_eq!(order.total_amount, Some(6930));
    }

    /// HTML テーブル形式のおまとめメール（email 650 相当）
    ///
    /// - 注文番号: `【注文No.】`（`ご` なし）→ 次行に `00129`
    /// - 注文日: `【注文日】`（`ご` なし）→ 次行に `2025-05-14 12:02:14`
    /// - 商品セクション: `【注文明細】`（`ご` なし、複数回出現）
    /// - 価格: `1,980円&times;1＝1,980円`（HTML エンティティ `&times;`）
    /// - 送料・手数料: ラベルのみ → 次行に金額
    /// - 合計: `支払合計金額：　4,290円`（`お` なし）
    fn sample_omatome_html_table() -> &'static str {
        "【注文No.】\n\
         00129\n\
         【注文日】\n\
         2025-05-14 12:02:14\n\
         【注文明細】\n\
         LBXオーディーンMk-2\n\
         1,980円&times;1＝1,980円\n\
         【お支払方法】\n\
         クレジットカード(出荷時決済)\n\
         【注文明細】\n\
         LBXアキレスD9\n\
         1,650円&times;1＝1,650円\n\
         【支払金額】\n\
         商品金額：\n\
         3,630円\n\
         支払合計金額：\u{3000}4,290円\n\
         送料：\n\
         660円\n\
         決済手数料：\n\
         0円\n"
    }

    // ─── HTML テーブル形式テスト ───

    #[test]
    fn test_parse_omatome_html_order_number() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_html_table())
            .unwrap();
        assert_eq!(order.order_number, "00129");
    }

    #[test]
    fn test_parse_omatome_html_order_date() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_html_table())
            .unwrap();
        assert_eq!(order.order_date, Some("2025-05-14".to_string()));
    }

    #[test]
    fn test_parse_omatome_html_item_count() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_html_table())
            .unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_omatome_html_item_names() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_html_table())
            .unwrap();
        assert_eq!(order.items[0].name, "LBXオーディーンMk-2");
        assert_eq!(order.items[1].name, "LBXアキレスD9");
    }

    #[test]
    fn test_parse_omatome_html_item_prices() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_html_table())
            .unwrap();
        assert_eq!(order.items[0].unit_price, 1980);
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[0].subtotal, 1980);
        assert_eq!(order.items[1].unit_price, 1650);
        assert_eq!(order.items[1].quantity, 1);
        assert_eq!(order.items[1].subtotal, 1650);
    }

    #[test]
    fn test_parse_omatome_html_amounts() {
        let order = PremiumBandaiOmatomeParser
            .parse(sample_omatome_html_table())
            .unwrap();
        assert_eq!(order.subtotal, Some(3630));
        assert_eq!(order.shipping_fee, Some(660)); // 送料 660 + 決済手数料 0
        assert_eq!(order.total_amount, Some(4290));
    }

    // ─── エラーケース ───

    #[test]
    fn test_parse_omatome_no_order_number_returns_error() {
        let result = PremiumBandaiOmatomeParser
            .parse("■ご注文内容\n商品A\n￥5,000（税込）×1 ￥5,000\n合計：￥5,000");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_omatome_no_items_returns_error() {
        let result =
            PremiumBandaiOmatomeParser.parse("■ご注文番号：12345\n■ご注文日：2025年1月15日");
        assert!(result.is_err());
    }
}
