use super::hobbysearch_common::{
    extract_amounts, extract_delivery_address, extract_delivery_info, parse_item_line,
};
use super::{EmailParser, OrderInfo, OrderItem};
use regex::Regex;

/// 発送通知メール用パーサー
pub struct HobbySearchSendParser;

impl EmailParser for HobbySearchSendParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let lines: Vec<&str> = email_body.lines().collect();

        // 代表注文番号を抽出
        let order_number = extract_representative_order_number(&lines)?;

        // 配送先情報を抽出
        let delivery_address = extract_delivery_address(&lines);

        // 配送情報を抽出（追跡番号など）
        let delivery_info = extract_delivery_info(&lines);

        // 商品情報を抽出（[ご購入内容]セクション）
        let items = extract_purchase_items(&lines)?;

        // 金額情報を抽出
        let (subtotal, shipping_fee, total_amount) = extract_amounts(&lines);

        Ok(OrderInfo {
            order_number,
            order_date: None,
            delivery_address,
            delivery_info,
            items,
            subtotal,
            shipping_fee,
            total_amount,
        })
    }
}

/// 代表注文番号を抽出（[代表注文番号] 形式）
fn extract_representative_order_number(lines: &[&str]) -> Result<String, String> {
    let order_number_pattern =
        Regex::new(r"\[代表注文番号\]\s*(\d+-\d+-\d+)").map_err(|e| format!("Regex error: {e}"))?;

    for line in lines {
        if let Some(captures) = order_number_pattern.captures(line) {
            if let Some(order_number) = captures.get(1) {
                return Ok(order_number.as_str().to_string());
            }
        }
    }

    Err("Representative order number not found".to_string())
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
        if in_purchase_section && (line.starts_with("小計") || line.starts_with("****")) {
            break;
        }

        if in_purchase_section {
            // [注文番号]行はスキップ
            if line.starts_with("[注文番号]") {
                i += 1;
                continue;
            }

            if !line.is_empty() && !line.starts_with("単価：") {
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
    fn test_parse_hobbysearch_send() {
        let sample_email = include_str!("../../../sample/hobbysearch_mail_send.txt");
        let parser = HobbySearchSendParser;
        let result = parser.parse(sample_email);

        assert!(result.is_ok());
        let order_info = result.unwrap();

        // 注文番号の確認
        assert_eq!(order_info.order_number, "25-0807-1624");

        // 商品数の確認
        assert_eq!(order_info.items.len(), 6);

        // 最初の商品の確認
        assert_eq!(
            order_info.items[0].name,
            "マックスファクトリー 014554 PLAMAX BP-02 ソフィア・F・シャーリング 虎アーマーVer."
        );
        assert_eq!(order_info.items[0].unit_price, 9350);
        assert_eq!(order_info.items[0].quantity, 1);

        // 配送情報の確認
        assert!(order_info.delivery_info.is_some());
        let delivery_info = order_info.delivery_info.unwrap();
        assert_eq!(delivery_info.carrier, "佐川急便");
        assert_eq!(delivery_info.tracking_number, "470550808943");

        // 金額情報の確認
        assert_eq!(order_info.subtotal, Some(46974));
        assert_eq!(order_info.shipping_fee, Some(0));
        assert_eq!(order_info.total_amount, Some(46974));

        // 配送先の確認
        assert!(order_info.delivery_address.is_some());
        let address = order_info.delivery_address.unwrap();
        assert_eq!(address.name, "原田 裕基");
        assert_eq!(address.postal_code, Some("812-0044".to_string()));
    }
}
