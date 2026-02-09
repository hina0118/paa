//! DMM通販「ご注文手続き完了のお知らせ」メール用パーサー
//!
//! 送信元: info@mail.dmm.com
//! 件名: DMM通販：ご注文手続き完了のお知らせ
//!
//! HTML を優先してパースし、フォールバックでテキストをパースする。

use super::{EmailParser, OrderInfo, OrderItem};
use regex::Regex;
use scraper::{Element, Html, Selector};

/// 商品名から【○月再生産分】【再販】等のプレフィックスを除去（正規化時に月情報が混入しないように）
fn normalize_product_name(name: &str) -> String {
    let mut s = name.trim().to_string();
    // 【】で囲まれた生産・発売関連のプレフィックスを除去（繰り返し適用）
    let bracket_patterns = [
        r"【\d{1,2}月再生産分】",
        r"【\d{1,2}月再販】",
        r"【\d{1,2}月発売】",
        r"【再販】",
        r"【再生産】",
        r"【再生産分】",
        r"【初回生産分】",
    ];
    for pat in &bracket_patterns {
        if let Ok(re) = Regex::new(pat) {
            s = re.replace_all(&s, "").into_owned();
        }
    }
    // 発売日・発売予定のプレフィックスを除去
    if let Ok(re) = Regex::new(r"^発売日[：:]\s*") {
        s = re.replace_all(&s, "").into_owned();
    }
    if let Ok(re) = Regex::new(r"^\d{4}/\d{1,2}月発売予定\s*") {
        s = re.replace_all(&s, "").into_owned();
    }
    if let Ok(re) = Regex::new(r"^\d{1,2}/\d{1,2}\s+発売予定\s*") {
        s = re.replace_all(&s, "").into_owned();
    }
    s.trim().to_string()
}

/// DMM通販 注文手続き完了メール用パーサー
pub struct DmmConfirmParser;

impl EmailParser for DmmConfirmParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let body = email_body.trim();
        if body.contains("<table") || body.contains("<html") {
            parse_from_html(body)
        } else {
            parse_from_text(body)
        }
    }
}

/// HTML からパース
fn parse_from_html(html: &str) -> Result<OrderInfo, String> {
    let document = Html::parse_document(html);

    let order_number = extract_order_number_from_html(&document)?;
    let order_date = extract_order_date_from_html(&document);
    let delivery_address = extract_delivery_address_from_html(&document);
    let items = extract_items_from_html(&document)?;
    let (subtotal, shipping_fee, total_amount) = extract_amounts_from_html(&document);

    Ok(OrderInfo {
        order_number,
        order_date,
        delivery_address,
        delivery_info: None,
        items,
        subtotal,
        shipping_fee,
        total_amount,
    })
}

fn extract_order_number_from_html(document: &Html) -> Result<String, String> {
    let tr_selector = Selector::parse("tr").unwrap_or_else(|_| Selector::parse("div").unwrap());
    let td_selector = Selector::parse("td").unwrap_or_else(|_| Selector::parse("div").unwrap());
    // 大文字・小文字の両方でパースし、そのまま使用（将来の注文詳細ページURL対応のため）
    let prefix_re = Regex::new(r"([A-Za-z]{2}-\d+)").unwrap();

    // 接頭辞（KC-, BS-等）必須。数字のみだと他メール（キャンセル・番号変更）と連携できないためエラー
    // 構造: <tr><td>BS-27892474</td><td>発送元：千葉配送センター</td><td>発送：...</td></tr>
    for tr in document.select(&tr_selector) {
        let tds: Vec<_> = tr.select(&td_selector).collect();
        for (i, td) in tds.iter().enumerate() {
            let text = td.text().collect::<String>();
            if text.contains("発送元") || text.contains("発送先") {
                if i > 0 {
                    let prev_td = &tds[i - 1];
                    let prev_text = prev_td.text().collect::<String>().trim().to_string();
                    if !prev_text.is_empty() {
                        if let Some(cap) = prefix_re.captures(&prev_text) {
                            if let Some(m) = cap.get(1) {
                                return Ok(m.as_str().to_string());
                            }
                        }
                    }
                }
                break;
            }
        }
    }

    // フォールバック: ご注文番号：KC-12345678 形式（接頭辞必須、大文字小文字両対応）
    let prefix_patterns = [
        Regex::new(r"ご注文番号\s*[：:]\s*([A-Za-z]{2}-\d+)"),
        Regex::new(r"注文番号\s*[：:]\s*([A-Za-z]{2}-\d+)"),
    ];
    for el in document.select(&td_selector) {
        let text = el.text().collect::<String>();
        for re in prefix_patterns.iter().flatten() {
            if let Some(cap) = re.captures(&text) {
                if let Some(m) = cap.get(1) {
                    return Ok(m.as_str().to_string());
                }
            }
        }
    }
    Err("Order number with prefix (KC-, BS-, etc.) not found".to_string())
}

fn extract_order_date_from_html(document: &Html) -> Option<String> {
    let td_selector = Selector::parse("td").unwrap_or_else(|_| Selector::parse("div").unwrap());
    let patterns = [
        Regex::new(r"ご注文日\s*[：:]\s*(\d{4})/(\d{1,2})/(\d{1,2})"),
        Regex::new(r"注文手続き日\s*[：:]\s*(\d{4})/(\d{1,2})/(\d{1,2})"),
        Regex::new(r"ご注文確定日\s*[：:]\s*(\d{4})/(\d{1,2})/(\d{1,2})"),
    ];

    for el in document.select(&td_selector) {
        let text = el.text().collect::<String>();
        if let Some(captures) = patterns.iter().flatten().find_map(|re| re.captures(&text)) {
            if let (Some(y), Some(m), Some(d)) = (captures.get(1), captures.get(2), captures.get(3)) {
                if let (Ok(month), Ok(day)) = (m.as_str().parse::<u32>(), d.as_str().parse::<u32>()) {
                    return Some(format!("{}-{:02}-{:02}", y.as_str(), month, day));
                }
            }
        }
    }
    None
}

fn extract_delivery_address_from_html(document: &Html) -> Option<super::DeliveryAddress> {
    let td_selector = Selector::parse("td").unwrap_or_else(|_| Selector::parse("div").unwrap());
    let re = Regex::new(r"受取人のお名前\s*[：:]\s*(.+)").ok()?;
    let re2 = Regex::new(r"購入者のお名前\s*[：:]\s*(.+)").ok()?;

    for el in document.select(&td_selector) {
        let text = el.text().collect::<String>();
        if let Some(captures) = re.captures(&text).or_else(|| re2.captures(&text)) {
            if let Some(m) = captures.get(1) {
                let name = m.as_str().trim().trim_end_matches('様').trim().to_string();
                if !name.is_empty() {
                    return Some(super::DeliveryAddress {
                        name,
                        postal_code: None,
                        address: None,
                    });
                }
            }
        }
    }
    None
}

fn extract_items_from_html(document: &Html) -> Result<Vec<OrderItem>, String> {
    let mut items = Vec::new();

    // 商品リンク（dmmref=gMono_Mail_Purchase）から商品名を取得
    // おすすめ商品（Recommend を含む）は除外
    let a_selector = Selector::parse("a[href*='dmmref=gMono_Mail_Purchase']").ok();
    if let Some(ref sel) = a_selector {
        for el in document.select(sel) {
            // おすすめ商品のリンクを除外（Recommend を含む場合はスキップ）
            if let Some(href) = el.value().attr("href") {
                if href.contains("Recommend") {
                    continue;
                }
            }
            let name = normalize_product_name(&el.text().collect::<String>());
            if name.is_empty() {
                continue;
            }
            // 同じ行ブロック内の価格・数量を探す（親の tr の兄弟をたどる）
            if let Some((unit_price, quantity)) = find_price_quantity_near_element(document, el) {
                if unit_price > 0 {
                    items.push(OrderItem {
                        name,
                        manufacturer: None,
                        model_number: None,
                        unit_price,
                        quantity,
                        subtotal: unit_price * quantity,
                    });
                }
            }
        }
    }

    // 商品リンクがなければ img[alt] から取得
    // おすすめ商品セクション内の画像は除外
    if items.is_empty() {
        let img_selector = Selector::parse("img[alt]").ok();
        if let Some(ref sel) = img_selector {
            for el in document.select(sel) {
                // おすすめ商品セクション内の画像を除外
                let mut is_recommend = false;
                let mut current = el;
                for _ in 0..10 {
                    if let Some(p) = current.parent_element() {
                        current = p;
                        let text = current.text().collect::<String>();
                        if text.contains("おすすめ商品") || text.contains("おすすめ") {
                            is_recommend = true;
                            break;
                        }
                    } else {
                        break;
                    }
                }
                if is_recommend {
                    continue;
                }

                if let Some(name) = el.value().attr("alt") {
                    let name = normalize_product_name(name);
                    if name.is_empty() || name.len() < 2 {
                        continue;
                    }
                    // 親テーブル内の価格を探す
                    if let Some((unit_price, quantity)) = find_price_quantity_near_element(document, el) {
                        if unit_price > 0 {
                            items.push(OrderItem {
                                name,
                                manufacturer: None,
                                model_number: None,
                                unit_price,
                                quantity,
                                subtotal: unit_price * quantity,
                            });
                        }
                    }
                }
            }
        }
    }

    if items.is_empty() {
        Err("No items found".to_string())
    } else {
        Ok(items)
    }
}

/// 要素の近くにある価格（○円）と数量（数量：N）を探す
/// 親要素をたどり、その中から抽出する
fn find_price_quantity_near_element(
    _document: &Html,
    element: scraper::ElementRef,
) -> Option<(i64, i64)> {
    let price_re = Regex::new(r"([\d,]+)円").ok()?;
    let qty_re = Regex::new(r"数量\s*[：:]\s*(\d+)").ok()?;

    let mut container = element;
    for _ in 0..10 {
        if let Some(p) = container.parent_element() {
            container = p;
        } else {
            break;
        }
        let text = container.text().collect::<String>();
        let mut unit_price = 0i64;
        let mut quantity = 1i64;

        for cap in price_re.captures_iter(&text) {
            if let Some(m) = cap.get(1) {
                if let Ok(p) = m.as_str().replace(',', "").parse::<i64>() {
                    if p > 0 && p < 100_000_000 {
                        unit_price = p;
                        break;
                    }
                }
            }
        }
        if let Some(cap) = qty_re.captures(&text) {
            if let Some(m) = cap.get(1) {
                quantity = m.as_str().parse().unwrap_or(1);
            }
        }

        if unit_price > 0 {
            return Some((unit_price, quantity));
        }
    }
    None
}

fn extract_amounts_from_html(document: &Html) -> (Option<i64>, Option<i64>, Option<i64>) {
    let text = document.root_element().text().collect::<String>();

    let subtotal_re = Regex::new(r"商品小計\s*[：:]\s*([\d,]+)円").unwrap_or_else(|_| Regex::new("").unwrap());
    let shipping_re = Regex::new(r"送料\s*[：:]\s*([\d,]+)円").unwrap_or_else(|_| Regex::new("").unwrap());
    let total_re = Regex::new(r"お支払い金額\s*[：:]\s*[\s\S]*?([\d,]+)円\s*\(税込\)").unwrap_or_else(|_| Regex::new("").unwrap());
    let total_re2 = Regex::new(r"支払い合計\s*[：:]\s*([\d,]+)円").unwrap_or_else(|_| Regex::new("").unwrap());

    let mut subtotal = None;
    let mut shipping_fee = None;
    let mut total_amount = None;

    for line in text.lines() {
        if let Some(cap) = subtotal_re.captures(line) {
            if let Some(m) = cap.get(1) {
                subtotal = m.as_str().replace(',', "").parse().ok();
            }
        }
        if let Some(cap) = shipping_re.captures(line) {
            if let Some(m) = cap.get(1) {
                shipping_fee = m.as_str().replace(',', "").parse().ok();
            }
        }
        if let Some(cap) = total_re.captures(line) {
            if let Some(m) = cap.get(1) {
                total_amount = m.as_str().replace(',', "").parse().ok();
            }
        }
        if total_amount.is_none() {
            if let Some(cap) = total_re2.captures(line) {
                if let Some(m) = cap.get(1) {
                    total_amount = m.as_str().replace(',', "").parse().ok();
                }
            }
        }
    }

    (subtotal, shipping_fee, total_amount)
}

/// テキストからパース（フォールバック）
fn parse_from_text(body: &str) -> Result<OrderInfo, String> {
    let lines: Vec<&str> = body.lines().collect();

    let order_number = extract_order_number(&lines)?;
    let order_date = extract_order_date(&lines);
    let delivery_address = extract_delivery_address(&lines);
    let items = extract_order_items(&lines)?;
    let (subtotal, shipping_fee, total_amount) = extract_amounts(&lines);

    Ok(OrderInfo {
        order_number,
        order_date,
        delivery_address,
        delivery_info: None,
        items,
        subtotal,
        shipping_fee,
        total_amount,
    })
}

fn extract_order_number(lines: &[&str]) -> Result<String, String> {
    // 大文字・小文字の両方でパースし、そのまま使用（将来の注文詳細ページURL対応のため）
    let prefix_re = Regex::new(r"([A-Za-z]{2}-\d+)").unwrap();

    // 接頭辞（KC-, BS-等）必須。数字のみだと他メール（キャンセル・番号変更）と連携できないためエラー
    // 「発送元」「発送先」を含む行から、その直前の部分で注文番号を抽出
    for line in lines {
        if line.contains("発送元") || line.contains("発送先") {
            let before_ship = line
                .split("発送元")
                .next()
                .unwrap_or("")
                .split("発送先")
                .next()
                .unwrap_or("")
                .trim();
            if !before_ship.is_empty() {
                if let Some(cap) = prefix_re.captures(before_ship) {
                    if let Some(m) = cap.get(1) {
                        return Ok(m.as_str().to_string());
                    }
                }
            }
        }
    }

    // フォールバック: ご注文番号：KC-12345678 形式（接頭辞必須、大文字小文字両対応）
    let patterns = [
        Regex::new(r"ご注文番号\s*[：:]\s*([A-Za-z]{2}-\d+)"),
        Regex::new(r"注文番号\s*[：:]\s*([A-Za-z]{2}-\d+)"),
    ];
    for line in lines {
        for re in patterns.iter().flatten() {
            if let Some(cap) = re.captures(line) {
                if let Some(m) = cap.get(1) {
                    return Ok(m.as_str().to_string());
                }
            }
        }
    }
    Err("Order number with prefix (KC-, BS-, etc.) not found".to_string())
}

fn extract_order_date(lines: &[&str]) -> Option<String> {
    let patterns = [
        Regex::new(r"ご注文日\s*[：:]\s*(\d{4})/(\d{1,2})/(\d{1,2})"),
        Regex::new(r"注文手続き日\s*[：:]\s*(\d{4})/(\d{1,2})/(\d{1,2})"),
        Regex::new(r"ご注文確定日\s*[：:]\s*(\d{4})/(\d{1,2})/(\d{1,2})"),
    ];
    for line in lines {
        for re in patterns.iter().flatten() {
            if let Some(cap) = re.captures(line) {
                let year = cap.get(1)?.as_str();
                let month = cap.get(2)?.as_str().parse::<u32>().ok()?;
                let day = cap.get(3)?.as_str().parse::<u32>().ok()?;
                return Some(format!("{}-{:02}-{:02}", year, month, day));
            }
        }
    }
    None
}

fn extract_delivery_address(lines: &[&str]) -> Option<super::DeliveryAddress> {
    let patterns = [
        Regex::new(r"受取人のお名前\s*[：:]\s*(.+)"),
        Regex::new(r"購入者のお名前\s*[：:]\s*(.+)"),
    ];
    for line in lines {
        let line = line.trim();
        for re in patterns.iter().flatten() {
            if let Some(cap) = re.captures(line) {
                if let Some(m) = cap.get(1) {
                    let name = m.as_str().trim().trim_end_matches('様').trim().to_string();
                    if !name.is_empty() {
                        return Some(super::DeliveryAddress { name, postal_code: None, address: None });
                    }
                }
            }
        }
    }
    None
}

fn extract_order_items(lines: &[&str]) -> Result<Vec<OrderItem>, String> {
    let pattern_a = Regex::new(r"発送日:\s*(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})\s+(.+)\s+(\d+)個\s*([\d,]+)円");
    let pattern_b = Regex::new(r"発売日[：:]\s*(.+)\s+(\d+)個\s*([\d,]+)円");
    let pattern_c = Regex::new(r"\d{1,2}/\d{1,2}\s+発売予定\s+(.+)\s+(\d+)個\s*([\d,]+)円");
    // テキストのみ形式: 商品名 数量個 価格円（発売日等のプレフィックスなし）
    let pattern_d = Regex::new(r"^(.+)\s+(\d+)個\s*([\d,]+)円\s*$");

    let mut items = Vec::new();
    for line in lines {
        let line = line.trim();

        if let Ok(re) = &pattern_a {
            if let Some(cap) = re.captures(line) {
                if let (Some(name), Some(qty), Some(price)) = (cap.get(2), cap.get(3), cap.get(4)) {
                    if let (Ok(q), Ok(p)) = (qty.as_str().parse::<i64>(), price.as_str().replace(',', "").parse::<i64>()) {
                        if p > 0 {
                            items.push(OrderItem {
                                name: normalize_product_name(name.as_str()),
                                manufacturer: None,
                                model_number: None,
                                unit_price: p,
                                quantity: q,
                                subtotal: p * q,
                            });
                        }
                    }
                }
                continue;
            }
        }

        if let Ok(re) = &pattern_b {
            if let Some(cap) = re.captures(line) {
                if let (Some(name), Some(qty), Some(price)) = (cap.get(1), cap.get(2), cap.get(3)) {
                    if let (Ok(q), Ok(p)) = (qty.as_str().parse::<i64>(), price.as_str().replace(',', "").parse::<i64>()) {
                        if p > 0 {
                            items.push(OrderItem {
                                name: normalize_product_name(name.as_str()),
                                manufacturer: None,
                                model_number: None,
                                unit_price: p,
                                quantity: q,
                                subtotal: p * q,
                            });
                        }
                    }
                }
                continue;
            }
        }

        if let Ok(re) = &pattern_c {
            if let Some(cap) = re.captures(line) {
                if let (Some(name), Some(qty), Some(price)) = (cap.get(1), cap.get(2), cap.get(3)) {
                    if let (Ok(q), Ok(p)) = (qty.as_str().parse::<i64>(), price.as_str().replace(',', "").parse::<i64>()) {
                        if p > 0 {
                            items.push(OrderItem {
                                name: normalize_product_name(name.as_str()),
                                manufacturer: None,
                                model_number: None,
                                unit_price: p,
                                quantity: q,
                                subtotal: p * q,
                            });
                        }
                    }
                }
                continue;
            }
        }

        if let Ok(re) = &pattern_d {
            if let Some(cap) = re.captures(line) {
                if let (Some(name), Some(qty), Some(price)) = (cap.get(1), cap.get(2), cap.get(3)) {
                    if let (Ok(q), Ok(p)) = (qty.as_str().parse::<i64>(), price.as_str().replace(',', "").parse::<i64>()) {
                        if p > 0 {
                            let name_normalized = normalize_product_name(name.as_str());
                            if !name_normalized.is_empty() && name_normalized.len() > 2 {
                                items.push(OrderItem {
                                    name: name_normalized,
                                    manufacturer: None,
                                    model_number: None,
                                    unit_price: p,
                                    quantity: q,
                                    subtotal: p * q,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    if items.is_empty() {
        Err("No items found".to_string())
    } else {
        Ok(items)
    }
}

fn extract_amounts(lines: &[&str]) -> (Option<i64>, Option<i64>, Option<i64>) {
    let subtotal_re = Regex::new(r"商品小計\s*[：:]\s*([\d,]+)円").unwrap_or_else(|_| Regex::new("").unwrap());
    let shipping_re = Regex::new(r"送料\s*[：:]\s*([\d,]+)円").unwrap_or_else(|_| Regex::new("").unwrap());
    let total_patterns = [
        Regex::new(r"お支払い金額\s*[：:]\s*([\d,]+)円").unwrap_or_else(|_| Regex::new("").unwrap()),
        Regex::new(r"支払い合計\s*[：:]\s*([\d,]+)円").unwrap_or_else(|_| Regex::new("").unwrap()),
        Regex::new(r"合計\s*[：:]\s*([\d,]+)円").unwrap_or_else(|_| Regex::new("").unwrap()),
    ];

    let mut subtotal = None;
    let mut shipping_fee = None;
    let mut total_amount = None;

    for line in lines {
        if let Some(cap) = subtotal_re.captures(line) {
            if let Some(m) = cap.get(1) {
                subtotal = m.as_str().replace(',', "").parse().ok();
            }
        }
        if let Some(cap) = shipping_re.captures(line) {
            if let Some(m) = cap.get(1) {
                shipping_fee = m.as_str().replace(',', "").parse().ok();
            }
        }
        for re in &total_patterns {
            if let Some(cap) = re.captures(line) {
                if let Some(m) = cap.get(1) {
                    total_amount = m.as_str().replace(',', "").parse().ok();
                    break;
                }
            }
        }
    }
    (subtotal, shipping_fee, total_amount)
}

// テストはローカル環境でのみ実行（サンプルファイルに個人情報が含まれるため）
#[cfg(all(test, not(ci)))]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dmm_confirm() {
        let sample_email = include_str!("../../../sample/dmm_mail_confirm.txt");
        let parser = DmmConfirmParser;
        let result = parser.parse(sample_email);

        assert!(result.is_ok());
        let order_info = result.unwrap();

        assert_eq!(order_info.order_number, "BS-27599843");
        assert_eq!(order_info.order_date, Some("2025-06-01".to_string()));
        assert_eq!(order_info.items.len(), 1);
        assert_eq!(
            order_info.items[0].name,
            "30MM ARMORED CORE VI FIRES OF RUBICON BALAM INDUSTRIES BD-011 MELANDER"
        );
        assert_eq!(order_info.items[0].unit_price, 2530);
        assert_eq!(order_info.items[0].quantity, 1);
        assert_eq!(order_info.subtotal, Some(2530));
        assert_eq!(order_info.shipping_fee, Some(530));
        assert_eq!(order_info.total_amount, Some(3060));
        assert!(order_info.delivery_address.is_some());
        assert_eq!(
            order_info.delivery_address.as_ref().unwrap().name,
            "テスト 太郎"
        );
    }
}

/// 形式B・形式C および HTML パース - CI でも実行
#[cfg(test)]
mod tests_format_b {
    use super::*;

    #[test]
    fn test_parse_dmm_confirm_format_b() {
        let email = r#"テスト 太郎 様

DMM通販をご利用いただき、ありがとうございます。
下記の内容にてご注文を承りましたのでご確認ください。

■　ご注文内容確認
ご注文番号:28156389　
ご注文日：2025/9/16
お支払い方法:クレジットカード

受取人のお名前：テスト 太郎 様

───────────────────────────────────
BS-28156389　発送元：千葉配送センター　発送：配送業者は発送時に確定
───────────────────────────────────
発売日：2026/03月発売予定 30MF クラスアップアーマー（ローザングラディエーター） 1個 979円

商品小計:979円
送料:530円
お支払い金額:1,509円(税込)
"#;
        let parser = DmmConfirmParser;
        let result = parser.parse(email);

        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let order_info = result.unwrap();

        assert_eq!(order_info.order_number, "BS-28156389");
        assert_eq!(order_info.order_date, Some("2025-09-16".to_string()));
        assert_eq!(order_info.items.len(), 1);
        assert_eq!(
            order_info.items[0].name,
            "30MF クラスアップアーマー（ローザングラディエーター）"
        );
        assert_eq!(order_info.items[0].unit_price, 979);
        assert_eq!(order_info.items[0].quantity, 1);
        assert_eq!(order_info.subtotal, Some(979));
        assert_eq!(order_info.shipping_fee, Some(530));
        assert_eq!(order_info.total_amount, Some(1509));
        assert_eq!(
            order_info.delivery_address.as_ref().unwrap().name,
            "テスト 太郎"
        );
    }

    #[test]
    fn test_parse_dmm_confirm_format_c() {
        let email = r#"テスト 太郎 様

DMM通販をご利用いただき、ありがとうございます。
下記の内容にてご注文を承りましたのでご確認ください。

■　ご注文内容確認
ご注文番号:24167237　
ご注文日：2023/12/26
お支払い方法:クレジットカード

受取人のお名前：テスト 太郎 様

───────────────────────────────────
KC-24167237　発送元：石川配送センター　発送：日本郵便
───────────────────────────────────
12/29 発売予定 サンプル商品【テスト】 1個 6,556円

商品小計:6,556円
送料:0円
お支払い金額:6,556円(税込)
"#;
        let parser = DmmConfirmParser;
        let result = parser.parse(email);

        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let order_info = result.unwrap();

        assert_eq!(order_info.order_number, "KC-24167237");
        assert_eq!(order_info.order_date, Some("2023-12-26".to_string()));
        assert_eq!(order_info.items.len(), 1);
        assert_eq!(order_info.items[0].name, "サンプル商品【テスト】");
        assert_eq!(order_info.items[0].unit_price, 6556);
        assert_eq!(order_info.items[0].quantity, 1);
        assert_eq!(order_info.subtotal, Some(6556));
        assert_eq!(order_info.shipping_fee, Some(0));
        assert_eq!(order_info.total_amount, Some(6556));
    }

    #[test]
    fn test_parse_dmm_confirm_html() {
        let html = r#"<html><body>
<table>
<tr><td>KC-23458091</td><td>発送元：千葉配送センター</td><td>発送：日本郵便</td></tr>
<tr><td>ご注文日：2023/8/22</td></tr>
<tr><td>受取人のお名前：テスト 太郎 様</td></tr>
</table>
<table>
<tr>
<td>
<a href="https://www.dmm.com/mono/hobby/-/detail/=/cid=cha_2308211721081/?dmmref=gMono_Mail_Purchase">BUSTER DOLL ガンナー</a>
</td>
<td>5,643円</td>
<td>数量：1</td>
</tr>
</table>
<table>
<tr><td>商品小計：5,643円</td></tr>
<tr><td>送料：0円</td></tr>
<tr><td>お支払い金額：<span style="color:#c00000;">5,643円(税込)</span></td></tr>
</table>
</body></html>"#;
        let parser = DmmConfirmParser;
        let result = parser.parse(html);

        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let order_info = result.unwrap();

        assert_eq!(order_info.order_number, "KC-23458091");
        assert_eq!(order_info.order_date, Some("2023-08-22".to_string()));
        assert_eq!(order_info.items.len(), 1);
        assert_eq!(order_info.items[0].name, "BUSTER DOLL ガンナー");
        assert_eq!(order_info.items[0].unit_price, 5643);
        assert_eq!(order_info.items[0].quantity, 1);
        assert_eq!(order_info.subtotal, Some(5643));
        assert_eq!(order_info.shipping_fee, Some(0));
        assert_eq!(order_info.total_amount, Some(5643));
        assert_eq!(
            order_info.delivery_address.as_ref().unwrap().name,
            "テスト 太郎"
        );
    }

    /// 注文番号が「発送元」の td と別セル（直前の td）にある構造
    #[test]
    fn test_parse_dmm_confirm_html_order_number_in_prev_td() {
        let html = r#"<html><body>
<table><tr><td>受取人のお名前：テスト 太郎 様</td></tr></table>
<table>
<tbody>
<tr>
<td align="left" valign="middle">BS-27892474</td>
<td align="left" valign="middle">発送元：千葉配送センター</td>
<td align="left" valign="middle">発送：配送業者は発送時に確定</td>
</tr>
</tbody>
</table>
<table>
<tr>
<td><a href="https://www.dmm.com/mono/hobby/-/detail/=/cid=test/?dmmref=gMono_Mail_Purchase">サンプル商品</a></td>
<td>1,000円</td>
<td>数量：1</td>
</tr>
</table>
<table>
<tr><td>商品小計：1,000円</td></tr>
<tr><td>送料：0円</td></tr>
<tr><td>お支払い金額：1,000円(税込)</td></tr>
</table>
</body></html>"#;
        let parser = DmmConfirmParser;
        let result = parser.parse(html);

        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let order_info = result.unwrap();
        assert_eq!(order_info.order_number, "BS-27892474");
    }

    /// 数字のみの注文番号はエラー（接頭辞必須、他メールと連携できないため）
    #[test]
    fn test_parse_dmm_confirm_rejects_numeric_only_order_number() {
        let email = r#"ご注文番号:23458091
ご注文日：2024/1/1
受取人のお名前：テスト 太郎 様

商品A 1個 1,000円
合計:1,000円(税込)"#;
        let parser = DmmConfirmParser;
        let result = parser.parse(email);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("prefix"));
    }

    /// 【11月再生産分】等のプレフィックスが除去される
    #[test]
    fn test_parse_dmm_confirm_product_name_normalize() {
        let email = r#"ご注文番号:KC-99999999
ご注文日：2024/1/1
受取人のお名前：テスト 太郎 様

【11月再生産分】商品A 1個 1,000円
合計:1,000円(税込)"#;
        let parser = DmmConfirmParser;
        let result = parser.parse(email);

        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let order_info = result.unwrap();

        assert_eq!(order_info.items[0].name, "商品A");
    }

    /// ご注文確定日形式
    #[test]
    fn test_parse_dmm_confirm_order_date_kakutei() {
        let email = r#"ご注文番号:KC-12345678
ご注文確定日：2024/3/15
受取人のお名前：テスト 太郎 様

商品名 1個 1,000円
合計:1,000円(税込)"#;
        let parser = DmmConfirmParser;
        let result = parser.parse(email);

        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let order_info = result.unwrap();

        assert_eq!(order_info.order_number, "KC-12345678");
        assert_eq!(order_info.order_date, Some("2024-03-15".to_string()));
    }

    /// テキストのみ形式（発売日等プレフィックスなし、合計:○円）
    #[test]
    fn test_parse_dmm_confirm_text_only() {
        let email = r#"テスト 太郎 様

DMM通販をご利用いただき、ありがとうございます。
下記の内容にてご注文を承りましたのでご確認ください。

■　ご注文内容確認
ご注文番号:17033992　
ご注文日：2019/12/5
お支払い方法:クレジットカード

受取人のお名前：テスト 太郎 様

───────────────────────────────────
KC-17033992　発送元１　石川配送センター　発送：日本郵便
───────────────────────────────────
CD（オトギフロンティア サウンドトラック2 Verion.319）(グッズ) 1個 3,000円

送料:0円
配送オプション料:0円
代引手数料:0円
合計:3,000円(税込)

《オプション》
配達日：指定なし
"#;
        let parser = DmmConfirmParser;
        let result = parser.parse(email);

        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let order_info = result.unwrap();

        assert_eq!(order_info.order_number, "KC-17033992");
        assert_eq!(order_info.order_date, Some("2019-12-05".to_string()));
        assert_eq!(order_info.items.len(), 1);
        assert_eq!(
            order_info.items[0].name,
            "CD（オトギフロンティア サウンドトラック2 Verion.319）(グッズ)"
        );
        assert_eq!(order_info.items[0].unit_price, 3000);
        assert_eq!(order_info.items[0].quantity, 1);
        assert_eq!(order_info.shipping_fee, Some(0));
        assert_eq!(order_info.total_amount, Some(3000));
        assert_eq!(
            order_info.delivery_address.as_ref().unwrap().name,
            "テスト 太郎"
        );
    }

    /// おすすめ商品セクションが含まれる場合、おすすめ商品は除外される
    #[test]
    fn test_parse_dmm_confirm_html_exclude_recommend() {
        let html = r#"<html><body>
<table>
<tr><td>KC-23458091</td><td>発送元：千葉配送センター</td><td>発送：日本郵便</td></tr>
<tr><td>ご注文日：2023/8/22</td></tr>
<tr><td>受取人のお名前：テスト 太郎 様</td></tr>
</table>
<table>
<tr>
<td>
<a href="https://www.dmm.com/mono/hobby/-/detail/=/cid=cha_2308211721081/?dmmref=gMono_Mail_Purchase">BUSTER DOLL ガンナー</a>
</td>
<td>5,643円</td>
<td>数量：1</td>
</tr>
</table>
<table>
<tr><td>商品小計：5,643円</td></tr>
<tr><td>送料：0円</td></tr>
<tr><td>お支払い金額：<span style="color:#c00000;">5,643円(税込)</span></td></tr>
</table>
<!-- レコメンド -->
<table>
<tr>
<td>おすすめ商品</td>
</tr>
<tr>
<td>
<a href="https://www.dmm.com/mono/hobby/-/detail/=/cid=cha_toumusical1905/?dmmref=gMono_Mail_Purchase_Recommend_Ranking_Hobby">【再販】舞台『刀剣乱舞』七周年感謝祭-夢語刀宴會- ランダム...</a>
</td>
<td>800円</td>
</tr>
</table>
</body></html>"#;
        let parser = DmmConfirmParser;
        let result = parser.parse(html);

        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let order_info = result.unwrap();

        assert_eq!(order_info.order_number, "KC-23458091");
        assert_eq!(order_info.order_date, Some("2023-08-22".to_string()));
        // おすすめ商品は除外され、注文商品のみが取得される
        assert_eq!(order_info.items.len(), 1);
        assert_eq!(order_info.items[0].name, "BUSTER DOLL ガンナー");
        assert_eq!(order_info.items[0].unit_price, 5643);
        assert_eq!(order_info.items[0].quantity, 1);
    }
}
