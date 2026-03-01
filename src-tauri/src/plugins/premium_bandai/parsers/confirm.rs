//! プレミアムバンダイ 注文確認メール用パーサー
//!
//! 件名：`ご注文完了のお知らせ`
//! 送信元：`evidence_bc@p-bandai.jp`
//!
//! フォーマット: multipart/alternative（テキスト + HTML）
//! 文字コード: ISO-2022-JP（Gmail API により UTF-8 に変換済み）
//!
//! 商品画像 URL は HTML パートから取得する。
//! 「おすすめ商品」セクション以降は注文商品に含めない。

use once_cell::sync::Lazy;
use regex::Regex;

use super::{
    body_to_lines, extract_image_urls_from_html, extract_order_date, extract_order_number,
    extract_payment_fee, extract_shipping_fee, extract_total_amount, find_recommend_section_line,
    normalize_product_name, parse_item_subtotal, parse_price, parse_quantity,
};
use crate::parsers::{EmailParser, OrderInfo, OrderItem};
use scraper::{ElementRef, Html, Selector};

/// ISO 日付（`YYYY-MM-DD`）または ISO 日時（`YYYY-MM-DD HH:MM:SS`）行を検出するパターン
///
/// HTML テーブル形式メールで `ご注文日` ラベルと値が別行になる場合に、
/// 値行（例: `2022-05-25 10:43:50`）を商品名として取り込まないためのフィルター。
static ISO_DATE_OR_DATETIME_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\d{4}-\d{2}-\d{2}(?: \d{2}:\d{2}:\d{2})?$")
        .expect("Invalid ISO_DATE_OR_DATETIME_RE")
});

/// 注文内容・注文明細セクション開始行を検出するパターン（行全体がセクションヘッダーであること）
///
/// `^...$` アンカーにより「以下、お客様のご注文内容の確認をお願い申しあげます。」のような
/// 文章行には **マッチしない**。対応形式:
/// - `■ご注文内容` / `ご注文内容` / `注文内容`（プレーンテキスト形式）
/// - `【注文明細】` / `【ご注文明細】`（HTML テーブル形式）
static ITEM_SECTION_START_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:■|【)?ご?注文(?:内容|明細)(?:】\s*)?$").expect("Invalid ITEM_SECTION_START_RE")
});

/// `【...】` のみの行を検出するパターン（HTML テーブル形式のセクションヘッダー）
///
/// 商品名は `【再販】` 等の接尾辞を持つが、行全体が `【...】` のみになることはない。
/// `【お支払方法】` / `【支払金額】` 等のセクションヘッダーを検出してアイテム収集を制御する。
static BRACKET_ONLY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^【[^【】]+】$").expect("Invalid BRACKET_ONLY_RE"));

/// `N円×N＝N円` / `N円&times;N＝N円` 形式の価格行
///
/// HTML テーブル形式の注文確認メールで使用される価格フォーマット。
/// `×` は U+00D7 (MULTIPLICATION SIGN) または HTML エンティティ `&times;` のいずれか。
/// `html_to_lines()` は HTML エンティティをデコードしないため両方に対応する。
static ITEM_PRICE_QTY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([\d,]+)円(?:×|&times;)(\d+)[＝=]([\d,]+)円").expect("Invalid ITEM_PRICE_QTY_RE")
});

/// プレミアムバンダイ 注文確認メール用パーサー
pub struct PremiumBandaiConfirmParser;

impl EmailParser for PremiumBandaiConfirmParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let body_lines = body_to_lines(email_body);
        let lines: Vec<&str> = body_lines.iter().map(|s| s.as_str()).collect();

        let order_number =
            extract_order_number(&lines).ok_or_else(|| "Order number not found".to_string())?;

        let order_date = extract_order_date(&lines);

        // おすすめ商品セクション以前のみ商品解析対象とする
        let item_end = find_recommend_section_line(&lines).unwrap_or(lines.len());
        let item_lines = &lines[..item_end];

        let image_urls = extract_image_urls_from_html(email_body);
        // HTML 形式ではまず HTML パーサで商品を抽出し、見つからなければテキストベースのパーサにフォールバックする
        let items = {
            let html_items = extract_items_from_confirm_html(email_body, &image_urls);
            if !html_items.is_empty() {
                html_items
            } else {
                extract_confirm_items(item_lines, &image_urls)
            }
        };
        if items.is_empty() {
            return Err("No items found".to_string());
        }

        let subtotal: i64 = items.iter().map(|i| i.subtotal).sum();
        let subtotal = if subtotal > 0 { Some(subtotal) } else { None };

        let shipping_fee = extract_shipping_fee(&lines);
        let payment_fee = extract_payment_fee(&lines);
        let total_amount = extract_total_amount(&lines);

        // payment_fee を shipping_fee に加算して OrderInfo の shipping_fee フィールドに格納する
        // （OrderInfo に payment_fee フィールドがないため）
        let combined_fee = match (shipping_fee, payment_fee) {
            (Some(s), Some(p)) => Some(s + p),
            (Some(s), None) => Some(s),
            (None, Some(p)) => Some(p),
            (None, None) => None,
        };

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

/// `N円×N＝N円` / `N円&times;N＝N円` 形式の価格行を解析する
///
/// 戻り値: `(unit_price, quantity, subtotal)`
fn parse_price_qty_subtotal(line: &str) -> Option<(i64, i64, i64)> {
    ITEM_PRICE_QTY_RE.captures(line).and_then(|c| {
        let unit_price: i64 = c[1].replace(',', "").parse().ok()?;
        let quantity: i64 = c[2].parse().ok()?;
        let subtotal: i64 = c[3].replace(',', "").parse().ok()?;
        Some((unit_price, quantity, subtotal))
    })
}

/// HTML パートから注文確認メールの商品リストを抽出する
///
/// `<th>注文明細</th>` → 親 `<tr>` → `<td>`（商品名）→ 次の兄弟 `<tr>` → `<td>`（価格情報）
/// のパターンを使用する。HTML でない入力（`<th>` が存在しない場合）は空リストを返す。
///
/// 商品画像は `<img alt="商品名">` の alt 属性と商品名（正規化前の生テキスト）を照合して取得する。
/// alt 属性で見つからない場合は `image_urls` のインデックス順にフォールバックする。
fn extract_items_from_confirm_html(html: &str, image_urls: &[String]) -> Vec<OrderItem> {
    let document = Html::parse_document(html);
    let th_sel = Selector::parse("th").unwrap();
    let td_sel = Selector::parse("td").unwrap();
    let img_sel = Selector::parse("img").unwrap();
    let mut items = Vec::new();

    // alt 属性 → src 属性のマップを構築（商品画像マッチング用）
    // プレミアムバンダイの注文確認メールでは <img alt="商品名"> で商品画像が特定できる
    let mut alt_to_src: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for img in document.select(&img_sel) {
        let alt = img.value().attr("alt").unwrap_or("").trim().to_string();
        if let Some(src) = img.value().attr("src") {
            if !alt.is_empty() && alt != "none" {
                alt_to_src.entry(alt).or_insert_with(|| src.to_string());
            }
        }
    }

    for th in document.select(&th_sel) {
        let th_text: String = th.text().collect();
        if !th_text.contains("注文明細") {
            continue;
        }

        // 親 <tr> を取得
        let tr_node = match th.parent() {
            Some(node) => node,
            None => continue,
        };
        let tr_el = match ElementRef::wrap(tr_node) {
            Some(el) if el.value().name() == "tr" => el,
            _ => continue,
        };

        // 同一 <tr> 内の <td>（商品名）を取得
        let name_td = match tr_el.select(&td_sel).next() {
            Some(td) => td,
            None => continue,
        };
        // 生テキスト（正規化前）を alt 照合に使用し、正規化後を商品名として登録する
        let raw_name: String = name_td.text().collect();
        let raw_name = raw_name.trim().to_string();
        let name = normalize_product_name(&raw_name);
        if name.is_empty() {
            continue;
        }

        // 次の兄弟 <tr> の <td>（価格情報）を取得
        let mut price_text: Option<String> = None;
        let mut candidate = tr_node.next_sibling();
        while let Some(node) = candidate {
            if let Some(next_el) = ElementRef::wrap(node) {
                if next_el.value().name() == "tr" {
                    if let Some(price_td) = next_el.select(&td_sel).next() {
                        price_text = Some(price_td.text().collect());
                    }
                }
                break;
            }
            candidate = node.next_sibling();
        }

        let (unit_price, quantity, subtotal) = price_text
            .as_deref()
            .and_then(|t| parse_price_qty_subtotal(t.trim()))
            .unwrap_or((0, 1, 0));

        // alt 属性で商品名に対応する画像を検索し、見つからなければインデックスでフォールバック
        let image_url = alt_to_src
            .get(&raw_name)
            .cloned()
            .or_else(|| image_urls.get(items.len()).cloned());

        items.push(OrderItem {
            name,
            manufacturer: None,
            model_number: None,
            unit_price,
            quantity,
            subtotal,
            image_url,
        });
    }

    items
}

/// 注文確認メールの商品リストを抽出する
///
/// 商品名行 → `単価：￥N（税込）` → `個数：N個` → `小計：￥N` のパターンを繰り返しパースする。
/// HTML テーブル形式では `N円×N＝N円` のパターンも対応する。
/// `image_urls` が提供された場合は商品の順序に対応する URL を割り当てる。
fn extract_confirm_items(lines: &[&str], image_urls: &[String]) -> Vec<OrderItem> {
    let mut items: Vec<OrderItem> = Vec::new();

    #[derive(Default)]
    struct Pending {
        name: Option<String>,
        unit_price: Option<i64>,
        quantity: Option<i64>,
    }

    let mut pending = Pending::default();
    let mut in_item_section = false;

    let flush = |p: &mut Pending, items: &mut Vec<OrderItem>, image_urls: &[String]| {
        if let Some(name) = p.name.take() {
            let quantity = p.quantity.unwrap_or(1);
            let unit_price = p.unit_price.unwrap_or(0);
            let subtotal = unit_price * quantity;
            let image_url = image_urls.get(items.len()).cloned();
            items.push(OrderItem {
                name,
                manufacturer: None,
                model_number: None,
                unit_price,
                quantity,
                subtotal,
                image_url,
            });
        }
        p.unit_price = None;
        p.quantity = None;
    };

    for line in lines {
        let trimmed = line.trim();

        // 注文内容セクション開始マーカー
        // 行全体がセクションヘッダーの場合のみマッチ（文章中の部分一致は除外）
        if ITEM_SECTION_START_RE.is_match(trimmed) {
            in_item_section = true;
            continue;
        }

        if !in_item_section {
            continue;
        }

        // `【...】` のみの行は HTML テーブル形式のセクションヘッダー
        // お支払方法・支払金額・配送関連のセクションが来たらアイテム収集を終了する
        if BRACKET_ONLY_RE.is_match(trimmed) {
            flush(&mut pending, &mut items, image_urls);
            if trimmed.contains("支払") || trimmed.contains("お届") || trimmed.contains("配送")
            {
                break;
            }
            continue;
        }

        // `N円×N＝N円` / `N円&times;N＝N円` 形式（HTML テーブル形式の注文確認メール）
        if let Some((unit_price, quantity, subtotal)) = parse_price_qty_subtotal(trimmed) {
            if let Some(ref name) = pending.name {
                let image_url = image_urls.get(items.len()).cloned();
                items.push(OrderItem {
                    name: name.clone(),
                    manufacturer: None,
                    model_number: None,
                    unit_price,
                    quantity,
                    subtotal,
                    image_url,
                });
                pending.name = None;
                pending.unit_price = None;
                pending.quantity = None;
            }
            continue;
        }

        // 小計行で商品を確定
        if let Some(subtotal) = parse_item_subtotal(trimmed) {
            if let Some(ref name) = pending.name {
                let quantity = pending.quantity.unwrap_or(1);
                let unit_price = pending.unit_price.unwrap_or_else(|| {
                    if quantity > 0 {
                        subtotal / quantity
                    } else {
                        subtotal
                    }
                });
                let image_url = image_urls.get(items.len()).cloned();
                items.push(OrderItem {
                    name: name.clone(),
                    manufacturer: None,
                    model_number: None,
                    unit_price,
                    quantity,
                    subtotal,
                    image_url,
                });
                pending.name = None;
                pending.unit_price = None;
                pending.quantity = None;
            }
            continue;
        }

        // 個数行
        if let Some(qty) = parse_quantity(trimmed) {
            pending.quantity = Some(qty);
            continue;
        }

        // 単価行
        if trimmed.starts_with("単価") {
            pending.unit_price = parse_price(trimmed);
            continue;
        }

        // 送料・支払手数料・合計などは商品名に取り込まない
        if trimmed.starts_with("送料")
            || trimmed.starts_with("支払手数料")
            || trimmed.starts_with("代引手数料")
            || trimmed.starts_with("合計")
            || trimmed.starts_with("小計：")
            || trimmed.starts_with("---")
            || trimmed.starts_with("===")
            || trimmed.is_empty()
        {
            // 現在の商品が未確定なら（小計行なしで次の区切りに来た場合）スキップ
            if trimmed.starts_with("送料") || trimmed.starts_with("合計") {
                flush(&mut pending, &mut items, image_urls);
            }
            continue;
        }

        // 注文番号・注文日・お支払方法などのヘッダー行はスキップ
        if trimmed.starts_with("ご注文番号")
            || trimmed.starts_with("注文番号")
            || trimmed.starts_with("ご注文日")
            || trimmed.starts_with("注文日")
            || trimmed.starts_with("お支払")
            || trimmed.starts_with("発売時期")
        {
            continue;
        }

        // ISO 日付/日時行をスキップ
        // HTML テーブル形式メールでは `ご注文日` ラベルと値（例: `2022-05-25 10:43:50`）が
        // 別行に分かれるため、値行を商品名として取り込まないようフィルターする。
        if ISO_DATE_OR_DATETIME_RE.is_match(trimmed) {
            continue;
        }

        // 前の商品がある場合は確定させてから新しい商品名を記録
        if pending.name.is_some() {
            flush(&mut pending, &mut items, image_urls);
        }

        if !trimmed.is_empty() {
            pending.name = Some(normalize_product_name(trimmed));
        }
    }

    // 末尾の未確定商品を確定（小計行なしで終わった場合）
    flush(&mut pending, &mut items, image_urls);

    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_confirm_plain() -> &'static str {
        r#"ご注文が完了しました。

■ご注文番号：12345
■ご注文日：2025年1月15日
■お支払方法：クレジットカード

■ご注文内容
--------------------------------------------
figma テスト【再販】
単価：￥5,000（税込）
個数：1個
小計：￥5,000
--------------------------------------------
送料：￥0
支払手数料：￥0
合計：￥5,000
--------------------------------------------
■おすすめ商品
おすすめ商品A
"#
    }

    fn sample_confirm_multiple_items() -> &'static str {
        r#"■ご注文番号：12345
■ご注文日：2025年1月15日

■ご注文内容
--------------------------------------------
商品A【2025年4月発送】
単価：￥5,000（税込）
個数：1個
小計：￥5,000
商品B
単価：￥3,000（税込）
個数：2個
小計：￥6,000
--------------------------------------------
送料：￥0
支払手数料：￥330
合計：￥11,330
"#
    }

    fn sample_confirm_with_payment_fee() -> &'static str {
        r#"■ご注文番号：12345
■ご注文日：2025年3月5日

■ご注文内容
商品C
単価：￥8,000（税込）
個数：1個
小計：￥8,000
送料：￥660
支払手数料：￥330
合計：￥8,990
"#
    }

    // ─── 基本テスト ───

    #[test]
    fn test_parse_confirm_order_number() {
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_plain())
            .unwrap();
        assert_eq!(order.order_number, "12345");
    }

    #[test]
    fn test_parse_confirm_order_date() {
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_plain())
            .unwrap();
        assert_eq!(order.order_date, Some("2025-01-15".to_string()));
    }

    #[test]
    fn test_parse_confirm_single_item() {
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_plain())
            .unwrap();
        assert_eq!(order.items.len(), 1);
        assert_eq!(order.items[0].name, "figma テスト");
        assert_eq!(order.items[0].unit_price, 5000);
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[0].subtotal, 5000);
    }

    #[test]
    fn test_parse_confirm_amounts() {
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_plain())
            .unwrap();
        assert_eq!(order.subtotal, Some(5000));
        assert_eq!(order.shipping_fee, Some(0)); // 送料0 + 支払手数料0
        assert_eq!(order.total_amount, Some(5000));
    }

    #[test]
    fn test_parse_confirm_normalizes_product_name() {
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_plain())
            .unwrap();
        // 【再販】が除去されていること
        assert_eq!(order.items[0].name, "figma テスト");
    }

    #[test]
    fn test_parse_confirm_excludes_recommend_section() {
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_plain())
            .unwrap();
        // おすすめ商品セクションの「おすすめ商品A」が含まれないこと
        assert!(!order.items.iter().any(|i| i.name.contains("おすすめ")));
    }

    // ─── 複数商品テスト ───

    #[test]
    fn test_parse_confirm_multiple_items_count() {
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_multiple_items())
            .unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_confirm_multiple_items_names() {
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_multiple_items())
            .unwrap();
        assert_eq!(order.items[0].name, "商品A");
        assert_eq!(order.items[1].name, "商品B");
    }

    #[test]
    fn test_parse_confirm_multiple_items_quantities() {
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_multiple_items())
            .unwrap();
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[1].quantity, 2);
    }

    #[test]
    fn test_parse_confirm_multiple_items_subtotals() {
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_multiple_items())
            .unwrap();
        assert_eq!(order.items[0].subtotal, 5000);
        assert_eq!(order.items[1].subtotal, 6000);
    }

    #[test]
    fn test_parse_confirm_combined_fee() {
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_with_payment_fee())
            .unwrap();
        // 送料 660 + 支払手数料 330 = 990
        assert_eq!(order.shipping_fee, Some(990));
        assert_eq!(order.total_amount, Some(8990));
    }

    // ─── エラーケース ───

    #[test]
    fn test_parse_confirm_no_order_number_returns_error() {
        let result = PremiumBandaiConfirmParser.parse(
            "■ご注文内容\n商品A\n単価：￥5,000（税込）\n個数：1個\n小計：￥5,000\n合計：￥5,000",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_confirm_no_items_returns_error() {
        let result =
            PremiumBandaiConfirmParser.parse("■ご注文番号：12345\n■ご注文日：2025年1月15日");
        assert!(result.is_err());
    }

    // ─── HTML テーブル形式（注文明細 + N円×N＝N円 フォーマット）テスト ───

    /// `html_to_lines()` 処理後を想定したプレーンテキスト形式
    /// 【注文明細】セクションヘッダーと `N円&times;N＝N円` 価格行を含む
    fn sample_confirm_html_table_price_qty() -> &'static str {
        "【注文No.】\n00037\n【注文明細】\nＨＧ 1/144 テスト商品【４次：２０２３年９月発送】\n2,420円&times;1＝2,420円\n【お支払方法】\nペイディ\n支払合計金額：　3,080円"
    }

    #[test]
    fn test_confirm_html_table_item_section_by_chumei() {
        // 【注文明細】がセクション開始マーカーとして認識されること
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_html_table_price_qty())
            .unwrap();
        assert_eq!(
            order.items.len(),
            1,
            "商品は1件のみのはず（got: {:?}）",
            order.items
        );
    }

    #[test]
    fn test_confirm_html_table_item_name_normalized() {
        // 【４次：２０２３年９月発送】が除去されること
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_html_table_price_qty())
            .unwrap();
        assert_eq!(order.items[0].name, "ＨＧ 1/144 テスト商品");
    }

    #[test]
    fn test_confirm_html_table_item_price_qty_subtotal() {
        // `N円&times;N＝N円` 形式から unit_price / quantity / subtotal が正しく抽出されること
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_html_table_price_qty())
            .unwrap();
        assert_eq!(order.items[0].unit_price, 2420);
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[0].subtotal, 2420);
    }

    #[test]
    fn test_confirm_html_table_bracket_only_not_treated_as_item() {
        // 【お支払方法】【支払金額】等がアイテムに混入しないこと
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_html_table_price_qty())
            .unwrap();
        assert!(
            !order
                .items
                .iter()
                .any(|i| i.name.contains("お支払") || i.name.contains("支払金額")),
            "セクションヘッダーがアイテムに混入している: {:?}",
            order.items
        );
    }

    #[test]
    fn test_parse_price_qty_subtotal_html_entity() {
        // `&times;` エンティティ形式のパース
        assert_eq!(
            parse_price_qty_subtotal("2,420円&times;1＝2,420円"),
            Some((2420, 1, 2420))
        );
    }

    #[test]
    fn test_parse_price_qty_subtotal_multiplication_sign() {
        // `×` (U+00D7) 形式のパース
        assert_eq!(
            parse_price_qty_subtotal("3,300円×2＝6,600円"),
            Some((3300, 2, 6600))
        );
    }

    // ─── HTML テーブル形式（次行値）テスト ───

    /// HTML テーブル形式メールで `ご注文日` の値行（ISO 日時）が商品名に混入しないことを確認
    fn sample_confirm_html_table_next_line_date() -> &'static str {
        // html_to_lines() 後の出力を想定したプレーンテキスト
        // <th>ご注文日</th><td>2022-05-25 10:43:50</td> が別行に分かれた場合
        "■ご注文番号：12345\nご注文内容\nご注文日\n2022-05-25 10:43:50\nサンプル商品A\n単価：￥3,300（税込）\n個数：1個\n小計：￥3,300\n送料：￥0\n合計：￥3,300"
    }

    #[test]
    fn test_confirm_html_table_iso_datetime_not_treated_as_item() {
        // ISO 日時行が商品名として取り込まれないこと
        let order = PremiumBandaiConfirmParser
            .parse(sample_confirm_html_table_next_line_date())
            .unwrap();
        assert_eq!(order.items.len(), 1, "商品は1件のみのはず");
        assert_eq!(order.items[0].name, "サンプル商品A");
    }

    #[test]
    fn test_confirm_html_table_iso_date_only_not_treated_as_item() {
        // 時刻なし ISO 日付行（`2022-05-25`）も商品名に含まれないこと
        let body = "■ご注文番号：12345\nご注文内容\nご注文日\n2022-05-25\nサンプル商品B\n単価：￥1,650（税込）\n個数：2個\n小計：￥3,300\n送料：￥0\n合計：￥3,300";
        let order = PremiumBandaiConfirmParser.parse(body).unwrap();
        assert_eq!(order.items.len(), 1, "商品は1件のみのはず");
        assert_eq!(order.items[0].name, "サンプル商品B");
    }

    // ─── HTML パーサ直接テスト ───

    #[test]
    fn test_extract_items_from_confirm_html_basic() {
        // 実際の HTML 形式: <th>注文明細</th> + <td>商品名</td> + 次 <tr> の <td> に価格
        let html = r#"<table><tr><th rowspan="2">【注文明細】</th><td>ＨＧ 1/144 テスト商品【４次：２０２３年９月発送】</td></tr><tr><td>2,420円×1＝2,420円</td></tr></table>"#;
        let items = extract_items_from_confirm_html(html, &[]);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "ＨＧ 1/144 テスト商品");
        assert_eq!(items[0].unit_price, 2420);
        assert_eq!(items[0].quantity, 1);
        assert_eq!(items[0].subtotal, 2420);
    }

    #[test]
    fn test_extract_items_from_confirm_html_plain_text_returns_empty() {
        // プレーンテキストには <th> が存在しないため空リストを返す（テキストベースにフォールバック）
        let items = extract_items_from_confirm_html("商品A\n2,420円×1＝2,420円", &[]);
        assert!(items.is_empty(), "プレーンテキストは空リストのはず");
    }

    #[test]
    fn test_extract_items_from_confirm_html_image_by_alt() {
        // <img alt="商品名"> から画像 URL を正しく取得すること（ヘッダー画像より優先）
        let html = r#"
            <img alt="ヘッダー" src="https://example.com/header.png">
            <img alt="ＨＧ 1/144 テスト商品【４次：２０２３年９月発送】" src="https://example.com/product.jpg">
            <table><tr>
                <th rowspan="2">【注文明細】</th>
                <td>ＨＧ 1/144 テスト商品【４次：２０２３年９月発送】</td>
            </tr><tr>
                <td>2,420円×1＝2,420円</td>
            </tr></table>
        "#;
        let items =
            extract_items_from_confirm_html(html, &["https://example.com/header.png".to_string()]);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "ＨＧ 1/144 テスト商品");
        // ヘッダー画像ではなく alt 一致の商品画像が使われること
        assert_eq!(
            items[0].image_url.as_deref(),
            Some("https://example.com/product.jpg")
        );
    }
}
