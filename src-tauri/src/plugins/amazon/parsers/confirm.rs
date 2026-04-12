//! Amazon.co.jp 注文確認メール用パーサー
//!
//! # 対応フォーマット
//!
//! ## 新フォーマット（件名 `注文済み:`）
//! ```text
//! 注文番号
//! 250-XXXXXXX-XXXXXXX
//!
//! * 商品名
//!   数量: 1
//!   1,234 JPY
//!
//! 合計
//! 1,234 JPY
//! ```
//!
//! ## 旧フォーマット・単一注文（件名 `Amazon.co.jp ご注文の確認`）
//! ```text
//! Amazon.co.jp ご注文の確認
//! 注文番号： 250-XXXXXXX-XXXXXXX
//! ...
//! 注文番号： 250-XXXXXXX-XXXXXXX
//! 注文日： YYYY/MM/DD
//!
//!                商品名
//!                ￥ 1,234
//! ...
//!               商品の小計： ￥ 1,234
//!               配送料・手数料： ￥ 0
//!               注文合計： ￥ 1,234
//! ```
//!
//! ## 旧フォーマット・複数注文（件名 `Amazon.co.jpでのご注文XXX（N点）`）
//! ヘッダーに複数の注文番号が並ぶ。各セクションに配送先＋合計のみ（商品行なし）。
//!
//! ## 超古いフォーマット（2011年頃・件名 `Amazon.co.jp ご注文の確認`）
//! ```text
//! Amazon.co.jpにご注文いただきありがとうございます。ご注文内容は以下のとおりです。
//! ...
//! ***...***
//! 注文番号：\t250-XXXXXXX-XXXXXXX
//! ...
//! 小計：        ￥ X,XXX
//! 配送料・手数料：    ￥ 0
//! この注文の合計：    ￥ X,XXX
//! ...
//! 1 "商品名"
//! 詳細; ￥ 価格
//! ```

use crate::parsers::{EmailParser, OrderInfo, OrderItem};
use regex::Regex;

/// Amazon 注文確認メールパーサー（全フォーマット対応）
pub struct AmazonConfirmParser;

/// 注文番号パターン（例: 250-1234567-1234567）
const ORDER_NUMBER_RE: &str = r"(\d{3}-\d{7}-\d{7})";

/// カンマ区切りの数字文字列を i64 に変換
fn parse_amount(s: &str) -> Option<i64> {
    s.replace(',', "").trim().parse::<i64>().ok()
}

impl EmailParser for AmazonConfirmParser {
    /// 単一注文をパース（新フォーマット または 旧フォーマット単一注文）
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        if is_new_format(email_body) {
            parse_new_format(email_body)
        } else if is_legacy_format(email_body) {
            let orders = parse_legacy_all_orders(email_body)?;
            orders
                .into_iter()
                .next()
                .ok_or_else(|| "注文情報が見つかりません".to_string())
        } else if is_very_old_format(email_body) {
            parse_very_old_format(email_body)
        } else {
            Err("未対応の Amazon メールフォーマットです".to_string())
        }
    }

    /// 複数注文をパース（旧フォーマット・複数注文のみ）
    ///
    /// 複数注文が含まれる場合のみ `Some(Ok(Vec<OrderInfo>))` を返す。
    /// 新フォーマットや旧フォーマット単一注文は `None` を返し、`parse()` に委譲する。
    fn parse_multi(&self, email_body: &str) -> Option<Result<Vec<OrderInfo>, String>> {
        if is_new_format(email_body) {
            return None;
        }
        if !is_legacy_format(email_body) {
            return None;
        }

        // ヘッダー部（最初の === セパレータ前）の注文番号を数える
        let header_count = count_header_order_numbers(email_body);
        if header_count <= 1 {
            return None; // 単一注文は parse() に委譲
        }

        Some(parse_legacy_all_orders(email_body))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// フォーマット判定
// ─────────────────────────────────────────────────────────────────────────────

/// 新フォーマット判定（`\n注文番号\n` が含まれる）
fn is_new_format(body: &str) -> bool {
    body.contains("\n注文番号\n") || body.contains("\n注文番号\r\n")
}

/// 旧フォーマット判定（`Amazon.co.jp ご注文の確認` で始まる）
fn is_legacy_format(body: &str) -> bool {
    body.trim_start().starts_with("Amazon.co.jp ご注文の確認")
}

/// 超古いフォーマット判定（`Amazon.co.jpにご注文いただきありがとうございます` で始まる）
fn is_very_old_format(body: &str) -> bool {
    body.trim_start()
        .starts_with("Amazon.co.jpにご注文いただきありがとうございます")
}

/// 旧フォーマットのヘッダー部（最初の === セパレータ前）にある一意な注文番号の数
fn count_header_order_numbers(body: &str) -> usize {
    let header_end = body
        .find("================================================================================")
        .unwrap_or(body.len());
    let header = &body[..header_end];

    let re = Regex::new(ORDER_NUMBER_RE).unwrap();
    let numbers: std::collections::HashSet<&str> =
        re.find_iter(header).map(|m| m.as_str()).collect();
    numbers.len()
}

// ─────────────────────────────────────────────────────────────────────────────
// 新フォーマット パース
// ─────────────────────────────────────────────────────────────────────────────

fn parse_new_format(body: &str) -> Result<OrderInfo, String> {
    let order_number = extract_new_order_number(body)?;
    let items = extract_new_items(body);
    let total_amount = extract_new_total(body);

    Ok(OrderInfo {
        order_number,
        order_date: None, // apply_internal_date で内部日付を補完
        delivery_address: None,
        delivery_info: None,
        items,
        subtotal: None,
        shipping_fee: None,
        total_amount,
    })
}

/// 新フォーマットの注文番号抽出
/// パターン: `\n注文番号\n250-XXXXXXX-XXXXXXX\n`
fn extract_new_order_number(body: &str) -> Result<String, String> {
    let pattern = format!(r"\n注文番号\r?\n{}\r?\n", ORDER_NUMBER_RE);
    let re = Regex::new(&pattern).map_err(|e| format!("Regex error: {e}"))?;
    re.captures(body)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| "注文番号が見つかりません (新フォーマット)".to_string())
}

/// 新フォーマットの商品情報抽出
/// パターン: `\n* 商品名\n  数量: N\n  価格 JPY`
fn extract_new_items(body: &str) -> Vec<OrderItem> {
    let re = match Regex::new(r"\n\* ([^\n]+)\r?\n  数量: (\d+)\r?\n  ([\d,]+) JPY") {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    re.captures_iter(body)
        .map(|cap| {
            let name = cap[1].trim().to_string();
            let quantity = cap[2].parse::<i64>().unwrap_or(1);
            let unit_price = parse_amount(&cap[3]).unwrap_or(0);
            OrderItem {
                name,
                manufacturer: None,
                model_number: None,
                unit_price,
                quantity,
                subtotal: unit_price * quantity,
                image_url: None,
            }
        })
        .collect()
}

/// 新フォーマットの合計金額抽出
/// パターン: `\n合計\n価格 JPY`
fn extract_new_total(body: &str) -> Option<i64> {
    let re = Regex::new(r"\n合計\r?\n([\d,]+) JPY").ok()?;
    re.captures(body)
        .and_then(|c| c.get(1))
        .and_then(|m| parse_amount(m.as_str()))
}

// ─────────────────────────────────────────────────────────────────────────────
// 超古いフォーマット パース（2011年頃）
// ─────────────────────────────────────────────────────────────────────────────

/// 超古いフォーマット（`***...***` 区切り）をパースする
///
/// 注文番号: `注文番号：\t...\t250-XXXXXXX-XXXXXXX`
/// 商品: `1 "商品名"\n詳細; ￥ 価格`
/// 合計: `この注文の合計：  ￥ X,XXX`
fn parse_very_old_format(body: &str) -> Result<OrderInfo, String> {
    let order_number_re =
        Regex::new(&format!(r"注文番号[：:]\s*{}", ORDER_NUMBER_RE)).map_err(|e| e.to_string())?;
    let total_re =
        Regex::new(r"この注文の合計[：:]\s*[￥¥]\s*([\d,]+)").map_err(|e| e.to_string())?;
    let subtotal_re = Regex::new(r"小計[：:]\s*[￥¥]\s*([\d,]+)").map_err(|e| e.to_string())?;
    let shipping_re =
        Regex::new(r"配送料・手数料[：:]\s*[￥¥]\s*([\d,]+)").map_err(|e| e.to_string())?;

    let order_number = order_number_re
        .captures(body)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| "注文番号が見つかりません (超古いフォーマット)".to_string())?;

    let total_amount = total_re
        .captures(body)
        .and_then(|c| c.get(1))
        .and_then(|m| parse_amount(m.as_str()));

    let subtotal = subtotal_re
        .captures(body)
        .and_then(|c| c.get(1))
        .and_then(|m| parse_amount(m.as_str()));

    let shipping_fee = shipping_re
        .captures(body)
        .and_then(|c| c.get(1))
        .and_then(|m| parse_amount(m.as_str()));

    let items = extract_very_old_items(body);

    Ok(OrderInfo {
        order_number,
        order_date: None, // 本文に注文日なし → apply_internal_date で補完
        delivery_address: None,
        delivery_info: None,
        items,
        subtotal,
        shipping_fee,
        total_amount,
    })
}

/// 超古いフォーマットから商品情報を抽出する
///
/// 商品行のフォーマット:
/// ```text
/// 1 "商品名"
/// 詳細テキスト; ￥ 価格
/// ```
fn extract_very_old_items(body: &str) -> Vec<OrderItem> {
    // `数量 "商品名"` 行にマッチ（行頭の数字 + スペース + "..."）
    // \r\n 改行に対応するため \r? を末尾に付ける
    let item_re = match Regex::new(r#"(?m)^(\d+) "([^"\r\n]+)"\r?$"#) {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let mut items = Vec::new();

    for cap in item_re.captures_iter(body) {
        let quantity = cap[1].parse::<i64>().unwrap_or(1);
        let name = cap[2].trim().to_string();

        // キャプチャのバイト位置から次の非空行を探す
        // match_end は閉じ `"` の直後（行末 \r\n の前）なので、lines() で空行をスキップする
        let match_end = cap.get(0).map(|m| m.end()).unwrap_or(0);
        let after = &body[match_end..];
        let next_line = after.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();

        // 次の行から `; ￥ 価格` または `; ¥ 価格` を抽出
        let unit_price = extract_price_from_detail_line(next_line).unwrap_or(0);

        items.push(OrderItem {
            name,
            manufacturer: None,
            model_number: None,
            unit_price,
            quantity,
            subtotal: unit_price * quantity,
            image_url: None,
        });
    }

    items
}

/// `詳細テキスト; ￥ 1,234` 形式の行から価格を抽出する
fn extract_price_from_detail_line(line: &str) -> Option<i64> {
    // セミコロン以降を探す
    let after_semi = line.rsplit_once(';').map(|(_, after)| after.trim())?;
    // ￥ / ¥ を除去して数値を取り出す
    let price_str = after_semi
        .strip_prefix('￥')
        .or_else(|| after_semi.strip_prefix('¥'))?
        .trim();
    parse_amount(price_str)
}

// ─────────────────────────────────────────────────────────────────────────────
// 旧フォーマット パース（単一・複数共通）
// ─────────────────────────────────────────────────────────────────────────────

/// 旧フォーマットから全注文を抽出する
///
/// === セパレータで区切られた各セクションを走査し、`注文日：` を含む
/// セクションを注文データとして処理する。
fn parse_legacy_all_orders(body: &str) -> Result<Vec<OrderInfo>, String> {
    let order_number_re =
        Regex::new(&format!(r"注文番号[：:]\s*{}", ORDER_NUMBER_RE)).map_err(|e| e.to_string())?;
    let date_re = Regex::new(r"注文日[：:]\s*(\d{4}/\d{2}/\d{2})").map_err(|e| e.to_string())?;
    let total_re =
        Regex::new(r"注文合計[：:]\s*[￥¥]\s*([\d,]+)").map_err(|e| e.to_string())?;
    let subtotal_re =
        Regex::new(r"商品の小計[：:]\s*[￥¥]\s*([\d,]+)").map_err(|e| e.to_string())?;
    let shipping_re =
        Regex::new(r"配送料・手数料[：:]\s*[￥¥]\s*([\d,]+)").map_err(|e| e.to_string())?;

    let separator = "================================================================================";
    let mut orders = Vec::new();

    for section in body.split(separator) {
        // 注文日を持たないセクション（ヘッダー・フッター）はスキップ
        if !section.contains("注文日") {
            continue;
        }

        // セクション内の最初の注文番号を取得
        let order_number = match order_number_re
            .captures(section)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
        {
            Some(n) => n,
            None => continue,
        };

        // 注文日（`YYYY/MM/DD` → `YYYY-MM-DD`）
        let order_date = date_re
            .captures(section)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().replace('/', "-"));

        // 金額情報
        let total_amount = total_re
            .captures(section)
            .and_then(|c| c.get(1))
            .and_then(|m| parse_amount(m.as_str()));
        let subtotal = subtotal_re
            .captures(section)
            .and_then(|c| c.get(1))
            .and_then(|m| parse_amount(m.as_str()));
        let shipping_fee = shipping_re
            .captures(section)
            .and_then(|c| c.get(1))
            .and_then(|m| parse_amount(m.as_str()));

        // 商品情報：配送先（お届け先）がないセクションのみ抽出
        // 複数注文フォーマットのセクションには「お届け先」が含まれるため商品行なし
        let items = if !section.contains("お届け先") {
            extract_legacy_items(section)
        } else {
            vec![]
        };

        orders.push(OrderInfo {
            order_number,
            order_date,
            delivery_address: None,
            delivery_info: None,
            items,
            subtotal,
            shipping_fee,
            total_amount,
        });
    }

    if orders.is_empty() {
        return Err("注文情報を解析できませんでした (旧フォーマット)".to_string());
    }

    Ok(orders)
}

/// 旧フォーマットのセクションから商品情報を抽出する
///
/// 抽出範囲: `注文日：` 行の直後 〜 `_________________` セパレータ前
///
/// 商品フォーマット（連続2行）:
/// ```text
///                商品名
///                ￥ 1,234
/// ```
fn extract_legacy_items(section: &str) -> Vec<OrderItem> {
    // 注文日: 行の終端位置を見つける
    let date_re = match Regex::new(r"注文日[：:]\s*\d{4}/\d{2}/\d{2}") {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    let date_end = match date_re.find(section) {
        Some(m) => m.end(),
        None => return vec![],
    };

    // ___________________ より前を商品セクションとする
    let item_section = &section[date_end..];
    let sep_pos = item_section
        .find("_________________")
        .unwrap_or(item_section.len());
    let item_section = &item_section[..sep_pos];

    // 行ごとに走査して (商品名行, ￥価格行) のペアを抽出
    // - 商品名行: 10文字以上のインデント・非空・￥/_/http で始まらない
    // - 次の非空行: ￥ で始まる
    let lines: Vec<&str> = item_section.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        let leading_spaces = line.len() - line.trim_start().len();

        let is_name_candidate = leading_spaces >= 10
            && !trimmed.is_empty()
            && !trimmed.starts_with('￥')
            && !trimmed.starts_with('¥')
            && !trimmed.starts_with('_')
            && !trimmed.starts_with("http");

        if is_name_candidate {
            // 次の非空行を探す
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }

            if j < lines.len() {
                let next = lines[j].trim();
                // 全角・半角yen記号の両方に対応
                // 全角（￥）・半角（¥）yen記号の両方に対応
                let price_str = next
                    .strip_prefix('￥')
                    .or_else(|| next.strip_prefix('¥'));

                if let Some(price_raw) = price_str {
                    if let Some(unit_price) = parse_amount(price_raw.trim()) {
                        items.push(OrderItem {
                            name: trimmed.to_string(),
                            manufacturer: None,
                            model_number: None,
                            unit_price,
                            quantity: 1,
                            subtotal: unit_price,
                            image_url: None,
                        });
                        i = j + 1;
                        continue;
                    }
                }
            }
        }

        i += 1;
    }

    items
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── 新フォーマット ──────────────────────────────────────────────────────

    fn new_format_single_item() -> String {
        // \n区切りで各行を明示（\ 継続はインデントを消すため使用しない）
        [
            "",
            "    原田裕基様、ご注文ありがとうございます。",
            "注文済み",
            "",
            "注文番号",
            "250-4632085-0519056",
            "",
            "注文内容の表示と変更",
            "https://www.amazon.co.jp/",
            "",
            "* ロキソニンSプレミアム 24錠",
            "  数量: 1",
            "  1,010 JPY",
            "",
            "合計",
            "1,010 JPY",
            "",
        ]
        .join("\n")
    }

    fn new_format_multiple_items() -> String {
        [
            "",
            "原田裕基様、ご注文ありがとうございます。",
            "注文済み",
            "",
            "注文番号",
            "250-9086729-1983839",
            "",
            "注文内容の表示と変更",
            "https://www.amazon.co.jp/",
            "",
            "* ブロックロス シナンジュ",
            "  数量: 1",
            "  1,870 JPY",
            "",
            "* ブロックロス ガンダムエピオン",
            "  数量: 2",
            "  1,870 JPY",
            "",
            "合計",
            "6,485 JPY",
            "",
        ]
        .join("\n")
    }

    #[test]
    fn test_new_format_detection() {
        let body = new_format_single_item();
        assert!(is_new_format(&body));
        assert!(!is_new_format(
            "Amazon.co.jp ご注文の確認\r\n注文番号： 123-4567890-1234567\r\n"
        ));
    }

    #[test]
    fn test_new_format_order_number() {
        let parser = AmazonConfirmParser;
        let body = new_format_single_item();
        let result = parser.parse(&body).unwrap();
        assert_eq!(result.order_number, "250-4632085-0519056");
    }

    #[test]
    fn test_new_format_single_item() {
        let parser = AmazonConfirmParser;
        let body = new_format_single_item();
        let result = parser.parse(&body).unwrap();
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].name, "ロキソニンSプレミアム 24錠");
        assert_eq!(result.items[0].unit_price, 1010);
        assert_eq!(result.items[0].quantity, 1);
        assert_eq!(result.items[0].subtotal, 1010);
    }

    #[test]
    fn test_new_format_multiple_items() {
        let parser = AmazonConfirmParser;
        let body = new_format_multiple_items();
        let result = parser.parse(&body).unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0].name, "ブロックロス シナンジュ");
        assert_eq!(result.items[0].unit_price, 1870);
        assert_eq!(result.items[1].name, "ブロックロス ガンダムエピオン");
        assert_eq!(result.items[1].quantity, 2);
        assert_eq!(result.items[1].subtotal, 3740);
    }

    #[test]
    fn test_new_format_total() {
        let parser = AmazonConfirmParser;
        let body = new_format_single_item();
        let result = parser.parse(&body).unwrap();
        assert_eq!(result.total_amount, Some(1010));
    }

    #[test]
    fn test_new_format_no_order_date() {
        let parser = AmazonConfirmParser;
        let body = new_format_single_item();
        let result = parser.parse(&body).unwrap();
        assert!(result.order_date.is_none()); // apply_internal_date で補完される
    }

    #[test]
    fn test_new_format_parse_multi_returns_none() {
        let parser = AmazonConfirmParser;
        let body = new_format_single_item();
        assert!(parser.parse_multi(&body).is_none());
    }

    // ── 旧フォーマット・単一注文 ─────────────────────────────────────────

    fn legacy_single_order() -> String {
        // \ 継続ではインデントが消えるため、各行を配列で定義して join する
        [
            "Amazon.co.jp ご注文の確認\r\n",
            "注文番号： 250-8161261-9767842\r\n",
            "https://www.amazon.co.jp/ref=TE_tex_h\r\n",
            "_________________________________________________________________________________\r\n",
            "\r\n",
            "原田裕基 様\r\n",
            "\r\n",
            "注文履歴： https://www.amazon.co.jp/gp/css/your-orders-access\r\n",
            "\r\n",
            "================================================================================\r\n",
            "\r\n",
            "領収書/購入明細書\r\n",
            "注文番号： 250-8161261-9767842\r\n",
            "注文日： 2020/04/27\r\n",
            "\r\n",
            "               パール金属 製氷皿 M 21個取 ボックス付\r\n",
            "               ￥ 382\r\n",
            "\r\n",
            "               トイレクイックル 20枚 × 3個\r\n",
            "               ￥ 1,089\r\n",
            "\r\n",
            "_________________________________________________________________________________\r\n",
            "    \r\n",
            "              商品の小計： ￥ 2,168\r\n",
            "              配送料・手数料： ￥ 0\r\n",
            "\r\n",
            "              注文合計： ￥ 2,168\r\n",
            "\r\n",
            "================================================================================\r\n",
            "\r\n",
            "Amazon.co.jp でのご注文について\r\n",
        ]
        .join("")
    }

    #[test]
    fn test_legacy_format_detection() {
        let body = legacy_single_order();
        assert!(is_legacy_format(&body));
        let new_body = new_format_single_item();
        assert!(!is_legacy_format(&new_body));
    }

    #[test]
    fn test_legacy_single_order_number() {
        let parser = AmazonConfirmParser;
        let body = legacy_single_order();
        let result = parser.parse(&body).unwrap();
        assert_eq!(result.order_number, "250-8161261-9767842");
    }

    #[test]
    fn test_legacy_single_order_date() {
        let parser = AmazonConfirmParser;
        let body = legacy_single_order();
        let result = parser.parse(&body).unwrap();
        assert_eq!(result.order_date, Some("2020-04-27".to_string()));
    }

    #[test]
    fn test_legacy_single_order_items() {
        let parser = AmazonConfirmParser;
        let body = legacy_single_order();
        let result = parser.parse(&body).unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0].name, "パール金属 製氷皿 M 21個取 ボックス付");
        assert_eq!(result.items[0].unit_price, 382);
        assert_eq!(result.items[1].name, "トイレクイックル 20枚 × 3個");
        assert_eq!(result.items[1].unit_price, 1089);
    }

    #[test]
    fn test_legacy_single_order_amounts() {
        let parser = AmazonConfirmParser;
        let body = legacy_single_order();
        let result = parser.parse(&body).unwrap();
        assert_eq!(result.subtotal, Some(2168));
        assert_eq!(result.shipping_fee, Some(0));
        assert_eq!(result.total_amount, Some(2168));
    }

    #[test]
    fn test_legacy_single_parse_multi_returns_none() {
        let parser = AmazonConfirmParser;
        let body = legacy_single_order();
        assert!(parser.parse_multi(&body).is_none());
    }

    // ── 旧フォーマット・複数注文 ─────────────────────────────────────────

    fn legacy_multi_order() -> String {
        [
            "Amazon.co.jp ご注文の確認\r\n",
            "注文番号： 250-1111111-1111111\r\n",
            "注文番号： 250-2222222-2222222\r\n",
            "https://www.amazon.co.jp/ref=TE_tex_h\r\n",
            "_________________________________________________________________________________\r\n",
            "\r\n",
            "原田裕基 様\r\n",
            "\r\n",
            "================================================================================\r\n",
            "\r\n",
            "領収書/購入明細書\r\n",
            "注文番号： 250-1111111-1111111\r\n",
            "注文日： 2025/07/13\r\n",
            "\r\n",
            "     お届け先：\r\n",
            "               原田 裕基 様\r\n",
            "               812-0044\r\n",
            "               福岡県\r\n",
            "\r\n",
            "_________________________________________________________________________________\r\n",
            "              注文合計： ￥ 3,021\r\n",
            "\r\n",
            "================================================================================\r\n",
            "\r\n",
            "注文番号： 250-2222222-2222222\r\n",
            "注文日： 2025/07/13\r\n",
            "\r\n",
            "     お届け先：\r\n",
            "               原田 裕基 様\r\n",
            "               812-0044\r\n",
            "               福岡県\r\n",
            "\r\n",
            "_________________________________________________________________________________\r\n",
            "              注文合計： ￥ 1,339\r\n",
            "\r\n",
            "================================================================================\r\n",
            "\r\n",
            "Amazon.co.jp でのご注文について\r\n",
        ]
        .join("")
    }

    #[test]
    fn test_multi_order_parse_multi_returns_two_orders() {
        let parser = AmazonConfirmParser;
        let body = legacy_multi_order();
        let result = parser.parse_multi(&body).unwrap().unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_multi_order_numbers() {
        let parser = AmazonConfirmParser;
        let body = legacy_multi_order();
        let orders = parser.parse_multi(&body).unwrap().unwrap();
        assert_eq!(orders[0].order_number, "250-1111111-1111111");
        assert_eq!(orders[1].order_number, "250-2222222-2222222");
    }

    #[test]
    fn test_multi_order_dates() {
        let parser = AmazonConfirmParser;
        let body = legacy_multi_order();
        let orders = parser.parse_multi(&body).unwrap().unwrap();
        assert_eq!(orders[0].order_date, Some("2025-07-13".to_string()));
        assert_eq!(orders[1].order_date, Some("2025-07-13".to_string()));
    }

    #[test]
    fn test_multi_order_totals() {
        let parser = AmazonConfirmParser;
        let body = legacy_multi_order();
        let orders = parser.parse_multi(&body).unwrap().unwrap();
        assert_eq!(orders[0].total_amount, Some(3021));
        assert_eq!(orders[1].total_amount, Some(1339));
    }

    #[test]
    fn test_multi_order_no_items() {
        // 複数注文フォーマットでは商品行が存在しない
        let parser = AmazonConfirmParser;
        let body = legacy_multi_order();
        let orders = parser.parse_multi(&body).unwrap().unwrap();
        assert!(orders[0].items.is_empty());
        assert!(orders[1].items.is_empty());
    }

    #[test]
    fn test_multi_order_parse_single_returns_first() {
        // parse() で複数注文ボディを渡すと最初の注文を返す
        let parser = AmazonConfirmParser;
        let body = legacy_multi_order();
        let result = parser.parse(&body).unwrap();
        assert_eq!(result.order_number, "250-1111111-1111111");
    }

    // ── 超古いフォーマット（2011年頃）────────────────────────────────────────

    fn very_old_format() -> String {
        [
            "Amazon.co.jpにご注文いただきありがとうございます。ご注文内容は以下のとおりです。\r\n",
            "\r\n",
            "***********************************************************\r\n",
            "\t注文内容\r\n",
            "***********************************************************\r\n",
            "\r\n",
            "注文番号：\t\t\t250-0927939-6707008\r\n",
            "\r\n",
            "小計：                           ￥ 8,843\r\n",
            "配送料・手数料：                          ￥ 0\r\n",
            "代引手数料：                          ￥ 260\r\n",
            "この注文の合計：                      ￥ 9,103\r\n",
            "\r\n",
            "1 \"玉ニュータウン 2nd Season ~玉よ永遠に~ 特別版 [DVD]\"\r\n",
            "市長(ヒデオ) CV:鈴村健一; DVD; ￥ 4,949\r\n",
            "\r\n",
            "1 \"モンスターハンターポータブル 3rd HD Ver.\"\r\n",
            "Video Game; ￥ 3,894\r\n",
            "\r\n",
        ]
        .join("")
    }

    #[test]
    fn test_very_old_format_detection() {
        let body = very_old_format();
        assert!(is_very_old_format(&body));
        assert!(!is_legacy_format(&body));
        assert!(!is_new_format(&body));
    }

    #[test]
    fn test_very_old_format_order_number() {
        let parser = AmazonConfirmParser;
        let body = very_old_format();
        let result = parser.parse(&body).unwrap();
        assert_eq!(result.order_number, "250-0927939-6707008");
    }

    #[test]
    fn test_very_old_format_amounts() {
        let parser = AmazonConfirmParser;
        let body = very_old_format();
        let result = parser.parse(&body).unwrap();
        assert_eq!(result.subtotal, Some(8843));
        assert_eq!(result.shipping_fee, Some(0));
        assert_eq!(result.total_amount, Some(9103));
    }

    #[test]
    fn test_very_old_format_items() {
        let parser = AmazonConfirmParser;
        let body = very_old_format();
        let result = parser.parse(&body).unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(
            result.items[0].name,
            "玉ニュータウン 2nd Season ~玉よ永遠に~ 特別版 [DVD]"
        );
        assert_eq!(result.items[0].unit_price, 4949);
        assert_eq!(result.items[0].quantity, 1);
        assert_eq!(result.items[1].name, "モンスターハンターポータブル 3rd HD Ver.");
        assert_eq!(result.items[1].unit_price, 3894);
    }

    #[test]
    fn test_very_old_format_no_order_date() {
        let parser = AmazonConfirmParser;
        let body = very_old_format();
        let result = parser.parse(&body).unwrap();
        assert!(result.order_date.is_none());
    }

    #[test]
    fn test_very_old_format_parse_multi_returns_none() {
        let parser = AmazonConfirmParser;
        let body = very_old_format();
        assert!(parser.parse_multi(&body).is_none());
    }

    #[test]
    fn test_extract_price_from_detail_line_multi_semi() {
        // セミコロンが複数ある場合は最後のセミコロン以降を価格とする
        assert_eq!(
            extract_price_from_detail_line("市長(ヒデオ) CV:鈴村健一; DVD; ￥ 4,949"),
            Some(4949)
        );
    }

    #[test]
    fn test_extract_price_from_detail_line_simple() {
        assert_eq!(
            extract_price_from_detail_line("Video Game; ¥ 3,894"),
            Some(3894)
        );
    }

    // ── エラーケース ─────────────────────────────────────────────────────────

    #[test]
    fn test_unknown_format_returns_err() {
        let parser = AmazonConfirmParser;
        assert!(parser.parse("全く無関係なメール本文です").is_err());
    }

    #[test]
    fn test_new_format_missing_order_number_returns_err() {
        let parser = AmazonConfirmParser;
        let body = "\n注文済み\n\n合計\n1,000 JPY\n";
        // is_new_format は false（\n注文番号\n がない）なので unknown format エラー
        assert!(parser.parse(body).is_err());
    }

    #[test]
    fn test_parse_amount_with_commas() {
        assert_eq!(parse_amount("1,234"), Some(1234));
        assert_eq!(parse_amount("10,000"), Some(10000));
        assert_eq!(parse_amount("0"), Some(0));
        assert_eq!(parse_amount("abc"), None);
    }
}
