//! コトブキヤオンラインショップ 注文確認メール用パーサー
//!
//! 件名：`ご注文確認のお知らせ［コトブキヤオンラインショップ］`
//! 送信元：`onlineshop@kotobukiya-ec.com`
//!
//! プレーンテキスト形式。
//! 注文日はメール本文に含まれるため `apply_internal_date` は不要。

use once_cell::sync::Lazy;
use regex::Regex;

use crate::parsers::{EmailParser, OrderInfo, OrderItem};

pub struct KotobukiyaConfirmParser;

// ─── 正規表現 ────────────────────────────────────────────────────────────────

/// `【オーダーID】0434429495`
static ORDER_NUMBER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"【オーダーID】(\d+)").expect("ORDER_NUMBER_RE"));

/// `【ご注文日】2026年04月15日`
static ORDER_DATE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"【ご注文日】(\d{4})年(\d{2})月(\d{2})日").expect("ORDER_DATE_RE"));

/// `　1.商品名 （商品名）` — 全角スペース + 番号 + ドット + 商品名
static ITEM_LINE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^　\d+\.(.+)$").expect("ITEM_LINE_RE"));

/// `　　価格：￥6,930 x 数量：1 = 合計：￥6,930`
static PRICE_LINE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"価格：￥([\d,]+)\s*x\s*数量：(\d+)\s*=\s*合計：￥([\d,]+)").expect("PRICE_LINE_RE")
});

/// `　　数量：1` — 特典など価格なし商品
static QUANTITY_ONLY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^　　数量：(\d+)").expect("QUANTITY_ONLY_RE"));

/// `　商品金額合計：￥6,930`
static SUBTOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"商品金額合計：￥([\d,]+)").expect("SUBTOTAL_RE"));

/// `　送料：￥660`
static SHIPPING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"　送料：￥([\d,]+)").expect("SHIPPING_RE"));

/// `　注文金額合計：￥7,590`
static TOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"注文金額合計：￥([\d,]+)").expect("TOTAL_RE"));

// ─── ヘルパー ─────────────────────────────────────────────────────────────────

fn parse_amount(s: &str) -> i64 {
    s.replace(',', "").parse().unwrap_or(0)
}

/// 商品名末尾の全角括弧 `（...）` を除去する
///
/// 例: `PUNI☆MOFU ロン （PUNI☆MOFU ロン）` → `PUNI☆MOFU ロン`
fn strip_name_suffix(name: &str) -> String {
    if let Some(pos) = name.rfind('（') {
        let candidate = name[..pos].trim_end();
        if !candidate.is_empty() {
            return candidate.to_string();
        }
    }
    name.trim().to_string()
}

// ─── 各フィールドの抽出 ───────────────────────────────────────────────────────

fn extract_order_number(body: &str) -> Option<String> {
    ORDER_NUMBER_RE.captures(body).map(|c| c[1].to_string())
}

fn extract_order_date(body: &str) -> Option<String> {
    ORDER_DATE_RE
        .captures(body)
        .map(|c| format!("{}-{}-{} 00:00:00", &c[1], &c[2], &c[3]))
}

/// `【ご注文明細】` セクションから商品リストを抽出する
///
/// 各商品ブロック（通常商品）：
/// ```text
/// 　N.商品名 （商品名）
/// 　　価格：￥X,XXX x 数量：N = 合計：￥X,XXX
/// ```
///
/// 特典など価格なし商品：
/// ```text
/// 　N.商品名
/// 　　数量：N
/// ```
///
/// `【お買上金額】` でセクション終了。
fn extract_items(body: &str) -> Vec<OrderItem> {
    let mut items: Vec<OrderItem> = Vec::new();
    let mut in_section = false;
    let mut current_name: Option<String> = None;
    let mut current_quantity: i64 = 1;
    let mut current_unit_price: i64 = 0;
    let mut current_subtotal: i64 = 0;

    for line in body.lines() {
        if line.contains("【ご注文明細】") {
            in_section = true;
            continue;
        }
        if !in_section {
            continue;
        }
        if line.contains("【お買上金額】") {
            if let Some(name) = current_name.take() {
                items.push(OrderItem {
                    name,
                    manufacturer: None,
                    model_number: None,
                    unit_price: current_unit_price,
                    quantity: current_quantity,
                    subtotal: current_subtotal,
                    image_url: None,
                });
            }
            break;
        }
        // お届け先行はスキップ
        if line.contains("・お届け先：") {
            continue;
        }
        // 商品行（全角スペース + 番号 + ドット）
        if let Some(caps) = ITEM_LINE_RE.captures(line) {
            if let Some(name) = current_name.take() {
                items.push(OrderItem {
                    name,
                    manufacturer: None,
                    model_number: None,
                    unit_price: current_unit_price,
                    quantity: current_quantity,
                    subtotal: current_subtotal,
                    image_url: None,
                });
            }
            current_name = Some(strip_name_suffix(caps[1].trim()));
            current_quantity = 1;
            current_unit_price = 0;
            current_subtotal = 0;
            continue;
        }
        // 価格行
        if let Some(caps) = PRICE_LINE_RE.captures(line) {
            current_unit_price = parse_amount(&caps[1]);
            current_quantity = caps[2].parse().unwrap_or(1);
            current_subtotal = parse_amount(&caps[3]);
            continue;
        }
        // 数量のみ行（特典など）
        if let Some(caps) = QUANTITY_ONLY_RE.captures(line) {
            if current_name.is_some() {
                current_quantity = caps[1].parse().unwrap_or(1);
            }
        }
    }

    // セクション末尾が 【お買上金額】 で終わらなかった場合の保険
    if let Some(name) = current_name.take() {
        items.push(OrderItem {
            name,
            manufacturer: None,
            model_number: None,
            unit_price: current_unit_price,
            quantity: current_quantity,
            subtotal: current_subtotal,
            image_url: None,
        });
    }

    items
}

fn extract_subtotal(body: &str) -> Option<i64> {
    SUBTOTAL_RE.captures(body).map(|c| parse_amount(&c[1]))
}

fn extract_shipping_fee(body: &str) -> Option<i64> {
    SHIPPING_RE.captures(body).map(|c| parse_amount(&c[1]))
}

fn extract_total_amount(body: &str) -> Option<i64> {
    TOTAL_RE.captures(body).map(|c| parse_amount(&c[1]))
}

// ─── EmailParser ─────────────────────────────────────────────────────────────

impl EmailParser for KotobukiyaConfirmParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let order_number = extract_order_number(email_body)
            .ok_or_else(|| "注文番号（オーダーID）が見つかりません".to_string())?;

        let items = extract_items(email_body);
        if items.is_empty() {
            return Err("商品情報が見つかりません".to_string());
        }

        let order_date = extract_order_date(email_body);
        let subtotal = extract_subtotal(email_body);
        let shipping_fee = extract_shipping_fee(email_body);
        let total_amount = extract_total_amount(email_body);

        Ok(OrderInfo {
            order_number,
            order_date,
            delivery_address: None,
            delivery_info: None,
            items,
            subtotal,
            shipping_fee,
            total_amount,
        })
    }
}

// ─── テスト ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_confirm() -> &'static str {
        // id=2479 の実メール本文をベースにしたサンプル（個人情報を匿名化）
        "山田　太郎　様\r\nコトブキヤオンラインショップをご利用いただき誠にありがとうございます。\r\n下記の内容にて、お客様からのご注文を確認いたしましたのでお知らせいたします。\r\n--------------------------------------------------\r\n【オーダーID】0434429495\r\n【ご注文日】2026年04月15日\r\n【ご注文者】山田　太郎　様\r\n【ご注文明細】\r\n・お届け先：山田　太郎　様\r\n\u{3000}1.PUNI☆MOFU ロン （PUNI☆MOFU ロン）\r\n\u{3000}\u{3000}価格：￥6,930 x 数量：1 = 合計：￥6,930\r\n\u{3000}2.【特典】特別カラー髪パーツ\r\n\u{3000}\u{3000}数量：1\r\n\r\n【お買上金額】\r\n\u{3000}商品金額合計：￥6,930\r\n\u{3000}送料：￥660\r\n\u{3000}手数料：￥0\r\n\u{3000}ポイント値引き：-0ポイント\r\n\u{3000}注文金額合計：￥7,590\r\n【お支払方法】クレジットカード\r\n"
    }

    #[test]
    fn test_parse_order_number() {
        let order = KotobukiyaConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.order_number, "0434429495");
    }

    #[test]
    fn test_parse_order_date() {
        let order = KotobukiyaConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.order_date, Some("2026-04-15 00:00:00".to_string()));
    }

    #[test]
    fn test_parse_item_count() {
        let order = KotobukiyaConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_item_names() {
        let order = KotobukiyaConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].name, "PUNI☆MOFU ロン");
        assert_eq!(order.items[1].name, "【特典】特別カラー髪パーツ");
    }

    #[test]
    fn test_parse_item_quantities() {
        let order = KotobukiyaConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[1].quantity, 1);
    }

    #[test]
    fn test_parse_item_prices() {
        let order = KotobukiyaConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].unit_price, 6930);
        assert_eq!(order.items[0].subtotal, 6930);
        // 特典は価格なし
        assert_eq!(order.items[1].unit_price, 0);
        assert_eq!(order.items[1].subtotal, 0);
    }

    #[test]
    fn test_parse_subtotal() {
        let order = KotobukiyaConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.subtotal, Some(6930));
    }

    #[test]
    fn test_parse_shipping_fee() {
        let order = KotobukiyaConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.shipping_fee, Some(660));
    }

    #[test]
    fn test_parse_total_amount() {
        let order = KotobukiyaConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.total_amount, Some(7590));
    }

    #[test]
    fn test_parse_no_order_number_returns_error() {
        let body = "【ご注文日】2026年04月15日\r\n【ご注文明細】\r\n\u{3000}1.テスト商品\r\n\u{3000}\u{3000}価格：￥1,000 x 数量：1 = 合計：￥1,000\r\n【お買上金額】\r\n";
        assert!(KotobukiyaConfirmParser.parse(body).is_err());
    }

    #[test]
    fn test_parse_no_items_returns_error() {
        let body = "【オーダーID】0434429495\r\n【ご注文明細】\r\n【お買上金額】\r\n";
        assert!(KotobukiyaConfirmParser.parse(body).is_err());
    }

    #[test]
    fn test_strip_name_suffix_with_bracket() {
        assert_eq!(
            strip_name_suffix("PUNI☆MOFU ロン （PUNI☆MOFU ロン）"),
            "PUNI☆MOFU ロン"
        );
    }

    #[test]
    fn test_strip_name_suffix_without_bracket() {
        assert_eq!(
            strip_name_suffix("【特典】特別カラー髪パーツ"),
            "【特典】特別カラー髪パーツ"
        );
    }
}
