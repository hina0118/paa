use super::hobbysearch_common::{extract_delivery_address, extract_yoyaku_total, parse_item_line};
use super::{EmailParser, OrderInfo, OrderItem};
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
    let order_number_pattern =
        Regex::new(r"\[注文番号\]\s*(\d+-\d+-\d+)").map_err(|e| format!("Regex error: {e}"))?;

    for line in lines {
        if let Some(captures) = order_number_pattern.captures(line) {
            if let Some(order_number) = captures.get(1) {
                return Ok(order_number.as_str().to_string());
            }
        }
    }

    Err("Order number not found".to_string())
}

/// 組み換え後の商品情報を抽出（[ご予約内容]セクション）
fn extract_yoyaku_items(lines: &[&str]) -> Result<Vec<OrderItem>, String> {
    let mut items = Vec::new();
    let mut in_yoyaku_section = false;

    // 商品行のパターン: "メーカー 品番 商品名 (プラモデル) シリーズ"
    // 次の行: "単価：X円 × 個数：Y = Z円"
    let price_pattern = Regex::new(r"単価：([\d,]+)円\s*×\s*個数：(\d+)\s*=\s*([\d,]+)円")
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
        if in_yoyaku_section
            && (line.starts_with("予約商品合計") || line.starts_with("一回の発送ごとに"))
        {
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

// テストはローカル環境でのみ実行（サンプルファイルに個人情報が含まれるため）
#[cfg(all(test, not(ci)))]
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
