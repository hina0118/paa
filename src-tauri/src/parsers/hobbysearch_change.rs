use super::{DeliveryAddress, EmailParser, OrderInfo, OrderItem};
use regex::Regex;

/// 組み換えメール用パーサー
/// 注: このパーサーは既存の注文番号に対して商品を完全に置き換えます
/// 元の注文（統合元）との紐付けは将来的に実装予定
pub struct HobbySearchChangeParser;

impl EmailParser for HobbySearchChangeParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let lines: Vec<&str> = email_body.lines().collect();

        // 注文番号を抽出（[注文番号] 形式）
        let order_number = extract_order_number(&lines)?;

        // 配送先情報を抽出
        let delivery_address = extract_delivery_address(&lines);

        // 組み換え後の商品情報を抽出（[ご予約内容]セクション）
        let items = extract_yoyaku_items(&lines)?;

        // 予約商品合計を抽出
        let subtotal = extract_yoyaku_total(&lines);

        Ok(OrderInfo {
            order_number,
            order_date: None,
            delivery_address,
            delivery_info: None,
            items,
            subtotal,
            shipping_fee: None,
            total_amount: None,
        })
    }
}

/// 注文番号を抽出（[注文番号] XX-XXXX-XXXX 形式）
fn extract_order_number(lines: &[&str]) -> Result<String, String> {
    let order_number_pattern = Regex::new(r"\[注文番号\]\s*(\d+-\d+-\d+)")
        .map_err(|e| format!("Regex error: {e}"))?;

    for line in lines {
        if let Some(captures) = order_number_pattern.captures(line) {
            if let Some(order_number) = captures.get(1) {
                return Ok(order_number.as_str().to_string());
            }
        }
    }

    Err("Order number not found".to_string())
}

/// 配送先情報を抽出
fn extract_delivery_address(lines: &[&str]) -> Option<DeliveryAddress> {
    let mut in_delivery_section = false;
    let mut name: Option<String> = None;
    let mut postal_code: Option<String> = None;
    let mut address: Option<String> = None;

    for line in lines {
        let trimmed = line.trim();

        // [商品お届け先] セクション開始（同じ行に名前がある場合もある）
        if trimmed.starts_with("[商品お届け先]") {
            in_delivery_section = true;
            // 同じ行に名前がある場合（例: "[商品お届け先]  原田 裕基 様"）
            if trimmed.ends_with('様') {
                let name_part = trimmed
                    .trim_start_matches("[商品お届け先]")
                    .trim()
                    .trim_end_matches('様')
                    .trim();
                name = Some(name_part.to_string());
            }
            continue;
        }

        if in_delivery_section {
            // セクション終了判定
            if trimmed.is_empty() || trimmed.starts_with('[') {
                break;
            }

            // 郵便番号と住所を抽出（同じ行にある場合）
            if trimmed.starts_with('〒') {
                // 郵便番号だけを抽出（例: "〒812-0044 福岡県..." → "812-0044"）
                let rest = trimmed.trim_start_matches('〒').trim();
                if let Some(space_pos) = rest.find(' ') {
                    postal_code = Some(rest[..space_pos].trim().to_string());
                    address = Some(rest[space_pos..].trim().to_string());
                } else {
                    postal_code = Some(rest.to_string());
                }
            }
            // 住所だけの行（都道府県で始まる行）
            else if (trimmed.contains('県') || trimmed.contains('都') || trimmed.contains('府')) && address.is_none() {
                address = Some(trimmed.to_string());
            }
            // 名前を抽出（「様」で終わる行）
            else if trimmed.ends_with('様') && name.is_none() {
                name = Some(trimmed.trim_end_matches('様').trim().to_string());
            }
        }
    }

    name.map(|n| DeliveryAddress {
        name: n,
        postal_code,
        address,
    })
}

/// 組み換え後の商品情報を抽出（[ご予約内容]セクション）
fn extract_yoyaku_items(lines: &[&str]) -> Result<Vec<OrderItem>, String> {
    let mut items = Vec::new();
    let mut in_yoyaku_section = false;

    // 商品行のパターン: "メーカー 品番 商品名 (プラモデル) シリーズ"
    // 次の行: "単価：X円 × 個数：Y = Z円"
    let price_pattern =
        Regex::new(r"単価：([\d,]+)円\s*×\s*個数：(\d+)\s*=\s*([\d,]+)円")
            .map_err(|e| format!("Regex error: {e}"))?;

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // [ご予約内容]セクション開始
        if line == "[ご予約内容]" {
            in_yoyaku_section = true;
            i += 1;
            continue;
        }

        // セクション終了判定（予約商品合計または空行）
        if in_yoyaku_section && (line.starts_with("予約商品合計") || line.starts_with("一回の発送ごとに")) {
            break;
        }

        if in_yoyaku_section && !line.is_empty() && !line.starts_with("単価：") {
            // 次の行に価格情報があるか確認
            if i + 1 < lines.len() {
                let next_line = lines[i + 1].trim();
                if let Some(captures) = price_pattern.captures(next_line) {
                    // 商品名行を解析
                    let (name, manufacturer, model_number) = parse_item_line(line);

                    // 価格情報を取得
                    let unit_price = captures
                        .get(1)
                        .map(|m| m.as_str().replace(',', "").parse::<i64>().unwrap_or(0))
                        .unwrap_or(0);
                    let quantity = captures
                        .get(2)
                        .map(|m| m.as_str().parse::<i64>().unwrap_or(1))
                        .unwrap_or(1);
                    let subtotal = captures
                        .get(3)
                        .map(|m| m.as_str().replace(',', "").parse::<i64>().unwrap_or(0))
                        .unwrap_or(0);

                    items.push(OrderItem {
                        name,
                        manufacturer,
                        model_number,
                        unit_price,
                        quantity,
                        subtotal,
                    });

                    // 価格情報の行をスキップ
                    i += 2;
                    continue;
                }
            }
        }

        i += 1;
    }

    if items.is_empty() {
        Err("No items found".to_string())
    } else {
        Ok(items)
    }
}

/// 商品行から商品名、メーカー、品番を抽出
fn parse_item_line(line: &str) -> (String, Option<String>, Option<String>) {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.is_empty() {
        return (line.to_string(), None, None);
    }

    // 最初の部分をメーカーとして扱う
    let manufacturer = Some(parts[0].to_string());

    // 2番目の部分が数字で始まる場合は品番
    let model_number = if parts.len() > 1 && parts[1].chars().next().map_or(false, |c| c.is_numeric()) {
        Some(parts[1].to_string())
    } else {
        None
    };

    // (プラモデル) または (ディスプレイ) の直前までを商品名として抽出
    let name = if let Some(paren_pos) = line.find(" (プラモデル)") {
        line[..paren_pos].trim().to_string()
    } else if let Some(paren_pos) = line.find(" (ディスプレイ)") {
        line[..paren_pos].trim().to_string()
    } else {
        line.to_string()
    };

    (name, manufacturer, model_number)
}

/// 予約商品合計を抽出
fn extract_yoyaku_total(lines: &[&str]) -> Option<i64> {
    let total_pattern = Regex::new(r"予約商品合計\s*([\d,]+)円").ok()?;

    for line in lines {
        if let Some(captures) = total_pattern.captures(line) {
            return captures
                .get(1)
                .and_then(|m| m.as_str().replace(',', "").parse::<i64>().ok());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hobbysearch_change() {
        let sample_email = include_str!("../../../sample/hobbysearch_mail_change.txt");
        let parser = HobbySearchChangeParser;
        let result = parser.parse(sample_email);

        assert!(result.is_ok());
        let order_info = result.unwrap();

        // 注文番号の確認
        assert_eq!(order_info.order_number, "25-1015-1825");

        // 商品数の確認（組み換え後）
        assert_eq!(order_info.items.len(), 4);

        // 最初の商品の確認
        assert_eq!(
            order_info.items[0].name,
            "グッドスマイルカンパニー 189270 PLAMATEA ストレイト・クーガー"
        );
        assert_eq!(order_info.items[0].unit_price, 7912);
        assert_eq!(order_info.items[0].quantity, 1);

        // 予約商品合計の確認
        assert_eq!(order_info.subtotal, Some(28024));

        // 配送先の確認
        assert!(order_info.delivery_address.is_some());
        let address = order_info.delivery_address.unwrap();
        assert_eq!(address.name, "原田 裕基");
    }
}
