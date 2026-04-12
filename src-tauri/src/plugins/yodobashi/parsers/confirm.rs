//! ヨドバシ・ドット・コム 注文確認メール用パーサー
//!
//! 件名：`ヨドバシ・ドット・コム：ご注文ありがとうございます`
//! 送信元：`thanks_gochuumon@yodobashi.com`
//!
//! プレーンテキスト形式。
//! 注文日はメール本文に含まれるため `apply_internal_date` は不要。

use once_cell::sync::Lazy;
use regex::Regex;

use crate::parsers::{EmailParser, OrderInfo, OrderItem};

pub struct YodobashiConfirmParser;

// ─── 正規表現 ────────────────────────────────────────────────────────────────

/// `【ご注文番号】 7224945594`
static ORDER_NUMBER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"【ご注文番号】\s*(\d+)").expect("ORDER_NUMBER_RE"));

/// `・ご注文日　　　　　　　　　　　2019年06月12日`
static ORDER_DATE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"ご注文日\s*(\d{4})年(\d{2})月(\d{2})日").expect("ORDER_DATE_RE"));

/// `【ご注文金額】今回のお買い物合計金額　　　　      2,527 円`
///
/// `[^\d]+` で数字以外を読み飛ばし、最初の数字グループを合計金額として取得する。
static TOTAL_AMOUNT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"【ご注文金額】[^\d]+([\d,]+)\s*円").expect("TOTAL_AMOUNT_RE")
});

/// `合計 1 点　   1,900 円` → (数量, 小計)
static ITEM_TOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"合計\s+(\d+)\s*点\s+([\d,]+)\s*円").expect("ITEM_TOTAL_RE"));

/// `・配達料金：　　0 円`
static SHIPPING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"・配達料金：\s*([\d,]+)\s*円").expect("SHIPPING_RE"));

// ─── ヘルパー ─────────────────────────────────────────────────────────────────

fn parse_amount(s: &str) -> i64 {
    s.replace(',', "").trim().parse().unwrap_or(0)
}

/// `2019年06月12日` → `"2019-06-12 00:00:00"`
fn parse_yodobashi_date(year: &str, month: &str, day: &str) -> String {
    format!("{}-{}-{} 00:00:00", year, month, day)
}

// ─── 各フィールドの抽出 ───────────────────────────────────────────────────────

fn extract_order_number(body: &str) -> Option<String> {
    ORDER_NUMBER_RE
        .captures(body)
        .map(|c| c[1].trim().to_string())
}

fn extract_order_date(body: &str) -> Option<String> {
    ORDER_DATE_RE
        .captures(body)
        .map(|c| parse_yodobashi_date(&c[1], &c[2], &c[3]))
}

fn extract_total_amount(body: &str) -> Option<i64> {
    TOTAL_AMOUNT_RE
        .captures(body)
        .map(|c| parse_amount(&c[1]))
}

fn extract_shipping_fee(body: &str) -> Option<i64> {
    SHIPPING_RE
        .captures(body)
        .map(|c| parse_amount(&c[1]))
}

/// `【ご注文商品】` セクションから商品リストを抽出する
///
/// 各商品ブロック：
/// ```text
/// ・「商品名（長い場合は次行に折り返される）
/// 　　折り返し部分」
///
/// 　　配達希望日：YYYY年MM月DD日
///
/// 　　合計 N 点　   X,XXX 円
/// ```
/// `・配達料金：` または `【お支払方法】` でセクション終了。
fn extract_items(body: &str) -> Vec<OrderItem> {
    let mut items: Vec<OrderItem> = Vec::new();
    let mut in_items_section = false;
    let mut current_name: Option<String> = None;
    let mut collecting_name = false; // 商品名が複数行に折り返されている途中

    for line in body.lines() {
        let trimmed = line.trim();

        if trimmed == "【ご注文商品】" {
            in_items_section = true;
            continue;
        }

        if !in_items_section {
            continue;
        }

        // セクション終了条件（名前収集中でも優先）
        if trimmed.starts_with("・配達料金") || trimmed.starts_with("【お支払方法】") {
            break;
        }

        // 商品名の折り返し行を収集中
        if collecting_name {
            if let Some(ref mut name) = current_name {
                if let Some(rest) = trimmed.strip_suffix('」') {
                    // 折り返しの最終行：名前確定
                    name.push_str(rest);
                    collecting_name = false;
                } else {
                    // さらに続く行（3行以上の折り返し）
                    name.push_str(trimmed);
                }
            }
            continue;
        }

        // 商品名行の開始（・「...」 または ・「...（折り返し）
        if let Some(after) = trimmed.strip_prefix("・「") {
            if let Some(name) = after.strip_suffix('」') {
                // 1行に収まっている
                current_name = Some(name.trim().to_string());
            } else {
                // 複数行に折り返されている
                current_name = Some(after.to_string());
                collecting_name = true;
            }
            continue;
        }

        // 合計行（数量 + 小計）
        if let Some(caps) = ITEM_TOTAL_RE.captures(trimmed) {
            if let Some(name) = current_name.take() {
                let quantity: i64 = caps[1].parse().unwrap_or(1);
                let subtotal = parse_amount(&caps[2]);
                let unit_price = if quantity > 0 { subtotal / quantity } else { subtotal };
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
    }

    items
}

// ─── EmailParser ─────────────────────────────────────────────────────────────

impl EmailParser for YodobashiConfirmParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let order_number = extract_order_number(email_body)
            .ok_or_else(|| "注文番号が見つかりません".to_string())?;

        let items = extract_items(email_body);
        if items.is_empty() {
            return Err("商品情報が見つかりません".to_string());
        }

        let order_date = extract_order_date(email_body);
        let shipping_fee = extract_shipping_fee(email_body);
        let total_amount = extract_total_amount(email_body);

        // 小計 = 各商品 subtotal の合算
        let subtotal: i64 = items.iter().map(|i| i.subtotal).sum();

        Ok(OrderInfo {
            order_number,
            order_date,
            delivery_address: None,
            delivery_info: None,
            items,
            subtotal: Some(subtotal),
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
        r#"==========================================================
■■ご注文ありがとうございます■■
　　（このメールは、配信専用のアドレスで配信されています）
==========================================================
ヨドバシ・ドット・コムをご利用いただき、ありがとうございます。
下記の内容でご注文を承りました。
（表示の価格は、すべて消費税総額表示です）

【ご注文番号】 7224945594
---------------------------------------------------------------
・ご注文主のお名前　　　　　　　テスト 太郎 様
・ご注文日　　　　　　　　　　　2019年06月12日

・お届け先のお名前　　　　　　　テスト 太郎 様
・商品のお届け先　　　　　　　　東京都テスト市テスト町1-1

【ご注文金額】今回のお買い物合計金額　　　　      2,527 円
---------------------------------------------------------------
・クレジットカードでのお支払い　　          2,527 円

【ご注文商品】
---------------------------------------------------------------
・「SS-07 [シンプルフォン]」

　　配達希望日：2019年06月13日

　　合計 1 点　   1,900 円

・「ペペ オーガニック 360ml」

　　配達希望日：2019年06月13日

　　合計 1 点　   627 円

・配達料金：　　0 円

【お支払方法】クレジットカード
---------------------------------------------------------------
"#
    }

    #[test]
    fn test_parse_order_number() {
        let order = YodobashiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.order_number, "7224945594");
    }

    #[test]
    fn test_parse_order_date() {
        let order = YodobashiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.order_date, Some("2019-06-12 00:00:00".to_string()));
    }

    #[test]
    fn test_parse_item_count() {
        let order = YodobashiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_item_names() {
        let order = YodobashiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].name, "SS-07 [シンプルフォン]");
        assert_eq!(order.items[1].name, "ペペ オーガニック 360ml");
    }

    #[test]
    fn test_parse_item_quantities() {
        let order = YodobashiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[1].quantity, 1);
    }

    #[test]
    fn test_parse_item_prices() {
        let order = YodobashiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].unit_price, 1900);
        assert_eq!(order.items[0].subtotal, 1900);
        assert_eq!(order.items[1].unit_price, 627);
        assert_eq!(order.items[1].subtotal, 627);
    }

    #[test]
    fn test_parse_shipping_fee() {
        let order = YodobashiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.shipping_fee, Some(0));
    }

    #[test]
    fn test_parse_total_amount() {
        let order = YodobashiConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.total_amount, Some(2527));
    }

    #[test]
    fn test_parse_subtotal() {
        let order = YodobashiConfirmParser.parse(sample_confirm()).unwrap();
        // 1,900 + 627 = 2,527
        assert_eq!(order.subtotal, Some(2527));
    }

    fn sample_confirm_wrapped() -> &'static str {
        // 商品名が2行に折り返されているケース（メール2234相当）
        r#"【ご注文番号】 7538892732
・ご注文日　　　　　　　　　　　2026年02月23日
【ご注文金額】今回のお買い物合計金額　　　　      4,487 円
【ご注文商品】
-------
・「データ用CD-R 700MB ひろびろワイドレーベル 10枚 エコパッケージ CD
　　R700S.SWPS.10E」

　　合計 1 点　   587 円

・「EXCERIA BASIC microSDHCカード 32GB Class10 UHS-I U1 最大読込50MB
　　/s KMUB-A032G」

　　合計 1 点　   1,070 円

・配達料金：　　0 円
"#
    }

    #[test]
    fn test_parse_wrapped_item_count() {
        let order = YodobashiConfirmParser.parse(sample_confirm_wrapped()).unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_wrapped_item_names() {
        let order = YodobashiConfirmParser.parse(sample_confirm_wrapped()).unwrap();
        // 折り返し部分が連結されていること
        assert!(order.items[0].name.contains("データ用CD-R"));
        assert!(order.items[0].name.contains("R700S.SWPS.10E"));
        assert!(order.items[1].name.contains("EXCERIA BASIC microSDHCカード"));
        assert!(order.items[1].name.contains("KMUB-A032G"));
    }

    #[test]
    fn test_parse_wrapped_item_prices() {
        let order = YodobashiConfirmParser.parse(sample_confirm_wrapped()).unwrap();
        assert_eq!(order.items[0].subtotal, 587);
        assert_eq!(order.items[1].subtotal, 1070);
    }

    #[test]
    fn test_parse_no_order_number_returns_error() {
        let result = YodobashiConfirmParser.parse("ご注文ありがとうございます。\n【ご注文商品】\n・「テスト商品」\n\n　　合計 1 点　   1,000 円\n・配達料金：　　0 円");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_no_items_returns_error() {
        let result = YodobashiConfirmParser.parse("【ご注文番号】 1234567890\n【ご注文商品】\n・配達料金：　　0 円");
        assert!(result.is_err());
    }
}
