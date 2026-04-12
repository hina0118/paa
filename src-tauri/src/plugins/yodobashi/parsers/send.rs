//! ヨドバシ・ドット・コム 発送通知メール用パーサー
//!
//! 件名：`ヨドバシ・ドット・コム：ご注文商品出荷のお知らせ`
//! 送信元：`otodoke@yodobashi.com`
//!
//! 対応配送業者：ヤマト運輸・日本郵便 ゆうパック・ヨドバシエクストリームサービス便・当社専用便

use once_cell::sync::Lazy;
use regex::Regex;

use crate::parsers::{DeliveryInfo, EmailParser, OrderInfo, OrderItem};

pub struct YodobashiSendParser;

// ─── 正規表現 ────────────────────────────────────────────────────────────────

/// `【ご注文番号】 7224945594`
static ORDER_NUMBER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"【ご注文番号】\s*(\d+)").expect("ORDER_NUMBER_RE"));

/// `・ご注文日　　　　　　　　　　　2019年06月12日`
static ORDER_DATE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"ご注文日\s*(\d{4})年(\d{2})月(\d{2})日").expect("ORDER_DATE_RE"));

/// `【ご注文金額】今回出荷のお買い物合計金額　　　　2,527 円`
static TOTAL_AMOUNT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"【ご注文金額】[^\d]+([\d,]+)\s*円").expect("TOTAL_AMOUNT_RE"));

/// 出荷商品セクションの数量・価格行
/// `　 　 1 点　1,900 円` → trimmed: `1 点　1,900 円`
static ITEM_QTY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(\d+)\s*点\s+([\d,]+)\s*円").expect("ITEM_QTY_RE"));

/// `・配達料金：　　0 円`
static SHIPPING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"・配達料金：\s*([\d,]+)\s*円").expect("SHIPPING_RE"));

/// `【配達について】今回の配達：ヤマト運輸 宅急便`
/// `【配達について】今回の配達担当：ヨドバシエクストリームサービス便`
static CARRIER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"【配達について】今回の配達(?:担当)?：(.+)").expect("CARRIER_RE"));

/// `配達受付番号（伝票番号）：335065497546`
static TRACKING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"配達受付番号（伝票番号）：(\d+)").expect("TRACKING_RE"));

// ─── ヘルパー ─────────────────────────────────────────────────────────────────

fn parse_amount(s: &str) -> i64 {
    s.replace(',', "").trim().parse().unwrap_or(0)
}

fn parse_yodobashi_date(year: &str, month: &str, day: &str) -> String {
    format!("{}-{}-{} 00:00:00", year, month, day)
}

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
    TOTAL_AMOUNT_RE.captures(body).map(|c| parse_amount(&c[1]))
}

fn extract_shipping_fee(body: &str) -> Option<i64> {
    SHIPPING_RE.captures(body).map(|c| parse_amount(&c[1]))
}

fn extract_carrier(body: &str) -> Option<String> {
    CARRIER_RE.captures(body).map(|c| c[1].trim().to_string())
}

fn extract_tracking_number(body: &str) -> Option<String> {
    TRACKING_RE.captures(body).map(|c| c[1].trim().to_string())
}

/// `▼ヤマト運輸ホームページ` または `▼日本郵便ホームページ` の次行にある追跡 URL を抽出する
fn extract_carrier_url(body: &str) -> Option<String> {
    let mut next_is_url = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if next_is_url {
            if trimmed.starts_with("http") {
                return Some(trimmed.to_string());
            }
            // 空行はスキップ、それ以外はリセット
            if !trimmed.is_empty() {
                next_is_url = false;
            }
        }
        if trimmed.starts_with('▼') && trimmed.ends_with("ホームページ") {
            next_is_url = true;
        }
    }
    None
}

/// `【今回出荷の商品】` セクションから商品リストを抽出する
///
/// 数量行は confirm の `合計 N 点` と異なり `N 点　X,XXX 円` 形式。
/// 商品名の折り返しは confirm・cancel と同形式。
/// `・配達料金：` または `【` で始まる次セクションでセクション終了。
fn extract_shipped_items(body: &str) -> Vec<OrderItem> {
    let mut items: Vec<OrderItem> = Vec::new();
    let mut in_section = false;
    let mut current_name: Option<String> = None;
    let mut collecting_name = false;

    for line in body.lines() {
        let trimmed = line.trim();

        if trimmed == "【今回出荷の商品】" {
            in_section = true;
            continue;
        }

        if !in_section {
            continue;
        }

        // セクション終了
        if trimmed.starts_with("・配達料金")
            || (trimmed.starts_with('【') && trimmed != "【今回出荷の商品】")
        {
            break;
        }

        // 商品名の折り返し行を収集中
        if collecting_name {
            if let Some(ref mut name) = current_name {
                if let Some(rest) = trimmed.strip_suffix('」') {
                    name.push_str(rest);
                    collecting_name = false;
                } else {
                    name.push_str(trimmed);
                }
            }
            continue;
        }

        // 商品名行
        if let Some(after) = trimmed.strip_prefix("・「") {
            if let Some(name) = after.strip_suffix('」') {
                current_name = Some(name.trim().to_string());
            } else {
                current_name = Some(after.to_string());
                collecting_name = true;
            }
            continue;
        }

        // 数量・価格行（`N 点　X,XXX 円`）
        if let Some(caps) = ITEM_QTY_RE.captures(trimmed) {
            if let Some(name) = current_name.take() {
                let quantity: i64 = caps[1].parse().unwrap_or(1);
                let subtotal = parse_amount(&caps[2]);
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
            }
            continue;
        }
    }

    items
}

/// 配送情報を抽出する
///
/// 追跡番号がない業者（ヨドバシエクストリームサービス便・当社専用便）では
/// `tracking_number` を空文字列とする。
fn extract_delivery_info(body: &str) -> Option<DeliveryInfo> {
    let carrier = extract_carrier(body)?;
    let tracking_number = extract_tracking_number(body).unwrap_or_default();
    let carrier_url = extract_carrier_url(body);

    Some(DeliveryInfo {
        carrier,
        tracking_number,
        delivery_date: None,
        delivery_time: None,
        carrier_url,
        delivery_status: None,
    })
}

// ─── EmailParser ─────────────────────────────────────────────────────────────

impl EmailParser for YodobashiSendParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let order_number = extract_order_number(email_body)
            .ok_or_else(|| "注文番号が見つかりません".to_string())?;

        let items = extract_shipped_items(email_body);
        if items.is_empty() {
            return Err("出荷商品が見つかりません".to_string());
        }

        let subtotal: i64 = items.iter().map(|i| i.subtotal).sum();

        Ok(OrderInfo {
            order_number,
            order_date: extract_order_date(email_body),
            delivery_address: None,
            delivery_info: extract_delivery_info(email_body),
            items,
            subtotal: Some(subtotal),
            shipping_fee: extract_shipping_fee(email_body),
            total_amount: extract_total_amount(email_body),
        })
    }
}

// ─── テスト ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_yamato() -> &'static str {
        r#"【ご注文番号】 7224945594
・ご注文日　　　　　　　　　　　2019年06月12日

【ご注文金額】今回出荷のお買い物合計金額　　　　2,527 円

【今回出荷の商品】
---------------------------------------------------------------
・「SS-07 [シンプルフォン]」
　 　 1 点　1,900 円
・「ペペ オーガニック 360ml」
　 　 1 点　627 円
・配達料金：　　0 円

【配達について】今回の配達：ヤマト運輸 宅急便
---------------------------------------------------------------
配達受付番号（伝票番号）：335065497546

　▼ヤマト運輸ホームページ
　　http://toi.kuronekoyamato.co.jp/cgi-bin/tneko?type=1&no01=335065497546
"#
    }

    fn sample_japanpost() -> &'static str {
        r#"【ご注文番号】 7466000284
・ご注文日　　　　　　　　　　　2022年05月17日

【ご注文金額】今回出荷のお買い物合計金額　　　　594 円

【今回出荷の商品】
---------------------------------------------------------------
・「30 MINUTES MISSIONS 1/144 オプションパーツセット7 （カスタマイズ
　　ヘッドB） [プラモデル用パーツ]」
　 　 1 点　594 円
・配達料金：　　0 円

【配達について】今回の配達：日本郵便 ゆうパック
---------------------------------------------------------------
配達受付番号（伝票番号）：263823775762

　▼日本郵便ホームページ
　　http://tracking.post.japanpost.jp/service/singleSearch.do?searchKind=S002&reqCodeNo1=263823775762
"#
    }

    fn sample_extreme() -> &'static str {
        r#"【ご注文番号】 7234682517
・ご注文日　　　　　　　　　　　2019年10月13日

【ご注文金額】今回出荷のお買い物合計金額　　　　4,136 円

【今回出荷の商品】
---------------------------------------------------------------
・「LB-L ピュアホワイト [ランドリーバスケット Lサイズ ピュアホワイト
　　]」
　 　 2 点　1,298 円
・「81336596 [AC/DC・2WAYパワーブロー 4mロングDCコード/0.51PSI]」
　 　 1 点　2,838 円
・配達料金：　　0 円

【配達会社指定サービス】
---------------------------------------------------------------
・ヨドバシエクストリームサービス便

【配達について】今回の配達担当：ヨドバシエクストリームサービス便
---------------------------------------------------------------
"#
    }

    // ── ヤマト運輸 ────────────────────────────────────────────────────────────

    #[test]
    fn test_yamato_order_number() {
        let order = YodobashiSendParser.parse(sample_yamato()).unwrap();
        assert_eq!(order.order_number, "7224945594");
    }

    #[test]
    fn test_yamato_order_date() {
        let order = YodobashiSendParser.parse(sample_yamato()).unwrap();
        assert_eq!(order.order_date, Some("2019-06-12 00:00:00".to_string()));
    }

    #[test]
    fn test_yamato_item_count() {
        let order = YodobashiSendParser.parse(sample_yamato()).unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_yamato_item_names() {
        let order = YodobashiSendParser.parse(sample_yamato()).unwrap();
        assert_eq!(order.items[0].name, "SS-07 [シンプルフォン]");
        assert_eq!(order.items[1].name, "ペペ オーガニック 360ml");
    }

    #[test]
    fn test_yamato_item_prices() {
        let order = YodobashiSendParser.parse(sample_yamato()).unwrap();
        assert_eq!(order.items[0].unit_price, 1900);
        assert_eq!(order.items[1].unit_price, 627);
    }

    #[test]
    fn test_yamato_total_amount() {
        let order = YodobashiSendParser.parse(sample_yamato()).unwrap();
        assert_eq!(order.total_amount, Some(2527));
    }

    #[test]
    fn test_yamato_shipping_fee() {
        let order = YodobashiSendParser.parse(sample_yamato()).unwrap();
        assert_eq!(order.shipping_fee, Some(0));
    }

    #[test]
    fn test_yamato_carrier() {
        let order = YodobashiSendParser.parse(sample_yamato()).unwrap();
        let di = order.delivery_info.unwrap();
        assert_eq!(di.carrier, "ヤマト運輸 宅急便");
        assert_eq!(di.tracking_number, "335065497546");
        assert!(di
            .carrier_url
            .as_deref()
            .unwrap()
            .contains("kuronekoyamato"));
    }

    // ── 日本郵便 ──────────────────────────────────────────────────────────────

    #[test]
    fn test_japanpost_carrier() {
        let order = YodobashiSendParser.parse(sample_japanpost()).unwrap();
        let di = order.delivery_info.unwrap();
        assert_eq!(di.carrier, "日本郵便 ゆうパック");
        assert_eq!(di.tracking_number, "263823775762");
        assert!(di.carrier_url.as_deref().unwrap().contains("japanpost"));
    }

    #[test]
    fn test_japanpost_wrapped_item_name() {
        let order = YodobashiSendParser.parse(sample_japanpost()).unwrap();
        assert!(order.items[0].name.contains("30 MINUTES MISSIONS"));
        assert!(order.items[0].name.contains("ヘッドB"));
    }

    // ── ヨドバシエクストリームサービス便 ──────────────────────────────────────

    #[test]
    fn test_extreme_carrier() {
        let order = YodobashiSendParser.parse(sample_extreme()).unwrap();
        let di = order.delivery_info.unwrap();
        assert_eq!(di.carrier, "ヨドバシエクストリームサービス便");
        assert_eq!(di.tracking_number, ""); // 追跡番号なし
        assert!(di.carrier_url.is_none());
    }

    #[test]
    fn test_extreme_item_count() {
        let order = YodobashiSendParser.parse(sample_extreme()).unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_extreme_item_quantity() {
        let order = YodobashiSendParser.parse(sample_extreme()).unwrap();
        assert_eq!(order.items[0].quantity, 2);
        assert_eq!(order.items[0].subtotal, 1298);
        assert_eq!(order.items[0].unit_price, 649);
    }

    // ── エラーケース ──────────────────────────────────────────────────────────

    #[test]
    fn test_no_order_number_returns_error() {
        let result = YodobashiSendParser.parse(
            "【今回出荷の商品】\n・「テスト商品」\n　 　 1 点　1,000 円\n・配達料金：　　0 円",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_no_items_returns_error() {
        let result = YodobashiSendParser
            .parse("【ご注文番号】 1234567890\n【今回出荷の商品】\n・配達料金：　　0 円");
        assert!(result.is_err());
    }
}
