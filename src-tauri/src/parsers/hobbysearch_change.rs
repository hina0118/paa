use super::hobbysearch_common::{extract_amounts, extract_delivery_address, parse_item_line};
use super::{EmailParser, OrderInfo, OrderItem};
use regex::Regex;

/// 組み換え（購入分）メール用パーサー
/// 注: このパーサーは既存の注文番号に対して商品を完全に置き換えます
/// [ご購入内容]セクションを持つ組み替えメールを処理
pub struct HobbySearchChangeParser;

impl EmailParser for HobbySearchChangeParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let lines: Vec<&str> = email_body.lines().collect();

        // 注文番号を抽出（[注文番号] 形式）
        let order_number = extract_order_number(&lines)?;

        // 配送先情報を抽出
        let delivery_address = extract_delivery_address(&lines);

        // 商品情報を抽出（[ご購入内容]セクション）
        let items = extract_purchase_items(&lines)?;

        // 金額情報を抽出（小計・送料・合計）
        let (subtotal, shipping_fee, total_amount) = extract_amounts(&lines);

        Ok(OrderInfo {
            order_number,
            order_date: None,
            delivery_address,
            delivery_info: None,
            items,
            subtotal,
            shipping_fee,
            total_amount,
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

/// 商品情報を抽出（[ご購入内容]セクション）
fn extract_purchase_items(lines: &[&str]) -> Result<Vec<OrderItem>, String> {
    let mut items = Vec::new();
    let mut in_purchase_section = false;

    // 商品行のパターン: "メーカー 品番 商品名 (プラモデル) シリーズ"
    // 次の行: "単価：X円 × 個数：Y = Z円"
    let price_pattern = Regex::new(r"単価：([\d,]+)円\s*×\s*個数：(\d+)\s*=\s*([\d,]+)円")
        .map_err(|e| format!("Regex error: {e}"))?;

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // [ご購入内容]セクション開始
        if line == "[ご購入内容]" {
            in_purchase_section = true;
            i += 1;
            continue;
        }

        // セクション終了判定（小計または区切り線）
        if in_purchase_section && (line.starts_with("小計") || line.starts_with("[▼")) {
            break;
        }

        if in_purchase_section && !line.is_empty() && !line.starts_with("単価：") {
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
                        image_url: None,
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

        // 注文番号の確認（XX-XXXX-XXXX 形式）
        let order_no_re = Regex::new(r"^\d+-\d+-\d+$").unwrap();
        assert!(order_no_re.is_match(&order_info.order_number));

        // 商品数の確認
        assert!(!order_info.items.is_empty());

        // 最初の商品の確認（名前・単価・数量が正しくパースされていること）
        assert!(!order_info.items[0].name.is_empty());
        assert!(order_info.items[0].unit_price > 0);
        assert!(order_info.items[0].quantity > 0);

        // 金額情報の確認
        assert!(order_info.subtotal.unwrap() > 0);
        assert!(order_info.shipping_fee.unwrap() >= 0);
        assert!(order_info.total_amount.unwrap() > 0);

        // 配送先の確認
        assert!(order_info.delivery_address.is_some());
        let address = order_info.delivery_address.unwrap();
        assert!(!address.name.is_empty());
    }
}
