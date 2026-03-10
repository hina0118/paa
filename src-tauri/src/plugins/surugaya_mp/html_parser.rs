//! 駿河屋マーケットプレイス マイページHTMLパーサー
//!
//! URL: `https://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=M...`
//!
//! # 抽出対象
//! - 取引番号
//! - 注文日（`2026.03.03` → `2026-03-03 00:00:00`）
//! - 商品合計・送料・総合計
//! - 発送日・発送会社名・お問い合わせ番号（追跡番号）
//! - 商品一覧（商品タイトル・単価・数量・金額）

use scraper::{Html, Selector};

use crate::parsers::{DeliveryInfo, OrderInfo, OrderItem};

// ─────────────────────────────────────────────────────────────────────────────
// 出力型
// ─────────────────────────────────────────────────────────────────────────────

/// マイページHTMLから抽出した注文情報
#[derive(Debug)]
pub struct MypageOrderInfo {
    pub trade_code: String,
    pub order_info: OrderInfo,
}

// ─────────────────────────────────────────────────────────────────────────────
// パース関数
// ─────────────────────────────────────────────────────────────────────────────

/// マイページHTMLをパースして注文情報を返す
pub fn parse_mypage_html(html: &str) -> Result<MypageOrderInfo, String> {
    let document = Html::parse_document(html);

    let trade_code =
        extract_trade_code(&document).ok_or_else(|| "取引番号が見つかりません".to_string())?;

    let order_date = extract_th_value(&document, "注文日").and_then(|s| parse_date_jp(&s));
    let subtotal = extract_th_value(&document, "商品合計").and_then(|s| parse_yen(&s));
    let shipping_fee =
        extract_th_value(&document, "送料・出荷手数料・離島追加送料").and_then(|s| parse_yen(&s));
    let total_amount = extract_th_value(&document, "総合計").and_then(|s| parse_yen(&s));

    let ship_date = extract_th_value(&document, "発送日").and_then(|s| parse_date_jp(&s));
    let carrier = extract_th_value(&document, "発送会社名");
    let tracking_number = extract_th_value(&document, "お問い合わせ番号");

    let delivery_info = build_delivery_info(carrier, tracking_number, ship_date);

    let items = extract_items(&document);

    Ok(MypageOrderInfo {
        trade_code: trade_code.clone(),
        order_info: OrderInfo {
            order_number: trade_code,
            order_date,
            delivery_address: None,
            delivery_info,
            items,
            subtotal,
            shipping_fee,
            total_amount,
        },
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// 内部ヘルパー
// ─────────────────────────────────────────────────────────────────────────────

/// `<input id="trade_code" value="M...">` から取引番号を取得する
fn extract_trade_code(document: &Html) -> Option<String> {
    let sel = Selector::parse("input#trade_code").ok()?;
    document
        .select(&sel)
        .next()
        .and_then(|el| el.value().attr("value"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// `<th>label</th>` の直後の `<td>` のテキストを返す
///
/// 同一 `<tr>` 内で `<th>` → `<td>` のペアを探す。
/// `<th>` が複数ある行でも、対象ラベルの直後の `<td>` を返す。
fn extract_th_value(document: &Html, label: &str) -> Option<String> {
    let tr_sel = Selector::parse("table.paddTbl tr").ok()?;
    let th_sel = Selector::parse("th").ok()?;
    let td_sel = Selector::parse("td").ok()?;

    for tr in document.select(&tr_sel) {
        let ths: Vec<_> = tr.select(&th_sel).collect();
        let tds: Vec<_> = tr.select(&td_sel).collect();

        for (i, th) in ths.iter().enumerate() {
            let th_text = th.text().collect::<String>();
            if th_text.trim() == label {
                // 同じ行内の i 番目の <td> を返す
                if let Some(td) = tds.get(i) {
                    let val = td.text().collect::<String>();
                    let val = val.trim().to_string();
                    if !val.is_empty() {
                        return Some(val);
                    }
                }
            }
        }
    }
    None
}

/// `2026.03.03` → `2026-03-03 00:00:00`
fn parse_date_jp(s: &str) -> Option<String> {
    let s = s.trim();
    // "2026.03.03" 形式
    if s.len() == 10 && s.chars().nth(4) == Some('.') && s.chars().nth(7) == Some('.') {
        let normalized = s.replace('.', "-");
        return Some(format!("{normalized} 00:00:00"));
    }
    None
}

/// `￥2,170` または `¥2,170` → `2170`
fn parse_yen(s: &str) -> Option<i64> {
    let digits: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .collect();
    digits.replace(',', "").parse().ok()
}

/// 配送情報を構築する
///
/// - 追跡番号あり: `shipped` ステータスで登録
/// - 追跡番号なし: `delivered` とみなす（追跡不可配送）
fn build_delivery_info(
    carrier: Option<String>,
    tracking_number: Option<String>,
    delivery_date: Option<String>,
) -> Option<DeliveryInfo> {
    let carrier = carrier?;
    let carrier_url = tracking_url_for_carrier(&carrier);

    match tracking_number {
        Some(tracking) if !tracking.is_empty() => Some(DeliveryInfo {
            carrier,
            tracking_number: tracking,
            delivery_date,
            delivery_time: None,
            carrier_url,
            delivery_status: None, // "shipped" がデフォルト
        }),
        _ => Some(DeliveryInfo {
            carrier,
            tracking_number: String::new(),
            delivery_date,
            delivery_time: None,
            carrier_url: None,
            delivery_status: Some("delivered".to_string()),
        }),
    }
}

/// 配送会社名から追跡URLを返す
fn tracking_url_for_carrier(carrier: &str) -> Option<String> {
    if carrier.contains("ヤマト") || carrier.contains("クロネコ") {
        // ヤマト運輸は追跡番号を直接URLに含める形式
        Some("https://jizen.kuronekoyamato.co.jp/jizen/servlet/crjz.b.NQ0010".to_string())
    } else if carrier.contains("日本郵便")
        || carrier.contains("ゆうパック")
        || carrier.contains("ゆうパケット")
    {
        Some(crate::plugins::JAPANPOST_TRACKING_URL.to_string())
    } else if carrier.contains("佐川") {
        Some("https://k2k.sagawa-exp.co.jp/p/web/okurijosearch.do".to_string())
    } else {
        None
    }
}

/// 商品テーブル (`mgnT15 paddTbl`) から商品行を抽出する
///
/// テーブルの構造:
/// ```html
/// <tr><th>品番</th><th>状態</th><th>枝番</th><th>商品タイトル</th><th>単価</th><th>数量</th><th>値引額</th><th>金額</th><th>備考</th></tr>
/// <tr><td>...</td><td>...</td><td>...</td><td class="left">商品名</td><td class="right">￥1,600</td><td>1</td><td>...</td><td>￥1,600</td><td>...</td></tr>
/// ```
fn extract_items(document: &Html) -> Vec<OrderItem> {
    let table_sel = match Selector::parse("table.mgnT15.paddTbl") {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let tr_sel = match Selector::parse("tr") {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let td_sel = match Selector::parse("td") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let mut items = Vec::new();

    if let Some(table) = document.select(&table_sel).next() {
        for tr in table.select(&tr_sel) {
            let tds: Vec<_> = tr.select(&td_sel).collect();
            // 商品行は td が 9 列（品番, 状態, 枝番, 商品タイトル, 単価, 数量, 値引額, 金額, 備考）
            if tds.len() < 8 {
                continue;
            }

            let name = tds[3].text().collect::<String>();
            let name = name.trim().to_string();
            if name.is_empty() {
                continue;
            }

            let unit_price_str = tds[4].text().collect::<String>();
            let unit_price = match parse_yen(&unit_price_str) {
                Some(p) if p > 0 => p,
                _ => continue,
            };

            let quantity_str = tds[5].text().collect::<String>();
            let quantity: i64 = quantity_str.trim().parse().unwrap_or(1);
            if quantity <= 0 {
                continue;
            }

            let subtotal_str = tds[7].text().collect::<String>();
            let subtotal = parse_yen(&subtotal_str).unwrap_or(unit_price * quantity);

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

// ─────────────────────────────────────────────────────────────────────────────
// テスト
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_html() -> String {
        // 実際のマイページ HTML を模したミニマルサンプル
        r#"<!DOCTYPE html>
<html>
<head><title>駿河屋 マイページ</title></head>
<body>
<input id="trade_code" type="hidden" value="M2603039345">
<table class="paddTbl">
  <tr><th>注文日</th><td>2026.03.03</td></tr>
  <tr><th>商品合計</th><td>￥2,170</td></tr>
  <tr><th>送料・出荷手数料・離島追加送料</th><td>￥500</td></tr>
  <tr><th>総合計</th><td>￥2,670</td></tr>
  <tr><th>発送日</th><td>2026.03.04</td></tr>
  <tr><th>発送会社名</th><td>ヤマト運輸（クロネコ）</td></tr>
  <tr><th>お問い合わせ番号</th><td>489888356635</td></tr>
</table>
<table class="mgnT15 paddTbl">
  <tr><th>品番</th><th>状態</th><th>枝番</th><th>商品タイトル</th><th>単価</th><th>数量</th><th>値引額</th><th>金額</th><th>備考</th></tr>
  <tr><td>PSP001</td><td>中古</td><td>-</td><td class="left">中古PSPソフト Carnage Heart PORTABLE</td><td class="right">￥1,600</td><td>1</td><td>￥0</td><td>￥1,600</td><td></td></tr>
  <tr><td>PS001</td><td>中古</td><td>-</td><td class="left">中古PSソフト Carnage Heart EZ (SLG)</td><td class="right">￥570</td><td>1</td><td>￥0</td><td>￥570</td><td></td></tr>
</table>
</body>
</html>"#
        .to_string()
    }

    #[test]
    fn test_parse_trade_code() {
        let result = parse_mypage_html(&sample_html()).unwrap();
        assert_eq!(result.trade_code, "M2603039345");
    }

    #[test]
    fn test_parse_order_date() {
        let result = parse_mypage_html(&sample_html()).unwrap();
        assert_eq!(
            result.order_info.order_date.as_deref(),
            Some("2026-03-03 00:00:00")
        );
    }

    #[test]
    fn test_parse_subtotal() {
        let result = parse_mypage_html(&sample_html()).unwrap();
        assert_eq!(result.order_info.subtotal, Some(2170));
    }

    #[test]
    fn test_parse_shipping_fee() {
        let result = parse_mypage_html(&sample_html()).unwrap();
        assert_eq!(result.order_info.shipping_fee, Some(500));
    }

    #[test]
    fn test_parse_total_amount() {
        let result = parse_mypage_html(&sample_html()).unwrap();
        assert_eq!(result.order_info.total_amount, Some(2670));
    }

    #[test]
    fn test_parse_items_count() {
        let result = parse_mypage_html(&sample_html()).unwrap();
        assert_eq!(result.order_info.items.len(), 2);
    }

    #[test]
    fn test_parse_item_names() {
        let result = parse_mypage_html(&sample_html()).unwrap();
        let names: Vec<&str> = result
            .order_info
            .items
            .iter()
            .map(|i| i.name.as_str())
            .collect();
        assert!(names.contains(&"中古PSPソフト Carnage Heart PORTABLE"));
        assert!(names.contains(&"中古PSソフト Carnage Heart EZ (SLG)"));
    }

    #[test]
    fn test_parse_item_prices() {
        let result = parse_mypage_html(&sample_html()).unwrap();
        let portable = result
            .order_info
            .items
            .iter()
            .find(|i| i.name.contains("PORTABLE"))
            .unwrap();
        assert_eq!(portable.unit_price, 1600);
        assert_eq!(portable.quantity, 1);
        assert_eq!(portable.subtotal, 1600);
    }

    #[test]
    fn test_parse_tracking_number() {
        let result = parse_mypage_html(&sample_html()).unwrap();
        let di = result.order_info.delivery_info.as_ref().unwrap();
        assert_eq!(di.tracking_number, "489888356635");
    }

    #[test]
    fn test_parse_carrier() {
        let result = parse_mypage_html(&sample_html()).unwrap();
        let di = result.order_info.delivery_info.as_ref().unwrap();
        assert_eq!(di.carrier, "ヤマト運輸（クロネコ）");
    }

    #[test]
    fn test_parse_carrier_url_yamato() {
        let result = parse_mypage_html(&sample_html()).unwrap();
        let di = result.order_info.delivery_info.as_ref().unwrap();
        assert!(di.carrier_url.is_some());
        assert!(di
            .carrier_url
            .as_deref()
            .unwrap()
            .contains("kuronekoyamato"));
    }

    #[test]
    fn test_parse_ship_date() {
        let result = parse_mypage_html(&sample_html()).unwrap();
        let di = result.order_info.delivery_info.as_ref().unwrap();
        assert_eq!(di.delivery_date.as_deref(), Some("2026-03-04 00:00:00"));
    }

    #[test]
    fn test_parse_date_jp_valid() {
        assert_eq!(
            parse_date_jp("2026.03.03"),
            Some("2026-03-03 00:00:00".to_string())
        );
    }

    #[test]
    fn test_parse_date_jp_invalid() {
        assert_eq!(parse_date_jp(""), None);
        assert_eq!(parse_date_jp("2026/03/03"), None);
    }

    #[test]
    fn test_parse_yen_fullwidth() {
        assert_eq!(parse_yen("￥2,170"), Some(2170));
    }

    #[test]
    fn test_parse_yen_halfwidth() {
        assert_eq!(parse_yen("¥500"), Some(500));
    }

    #[test]
    fn test_parse_yen_zero() {
        assert_eq!(parse_yen("￥0"), Some(0));
    }

    #[test]
    fn test_parse_no_trade_code_returns_error() {
        let html = "<html><body><p>取引番号なし</p></body></html>";
        assert!(parse_mypage_html(html).is_err());
    }
}
