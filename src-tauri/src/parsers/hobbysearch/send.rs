use super::{extract_amounts, extract_delivery_address, extract_delivery_info, parse_item_line};
use crate::parsers::{EmailParser, OrderInfo, OrderItem};
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

    /// 複数注文の同時発送に対応。
    /// [ご購入内容]内に[注文番号]セクションが存在する場合、各注文を個別の OrderInfo として返す。
    /// 各注文は個別の商品リストを持ち、配送情報（追跡番号等）は全注文で共有する。
    /// [注文番号]セクションが存在しない場合は None を返し、parse() にフォールバックする。
    fn parse_multi(&self, email_body: &str) -> Option<Result<Vec<OrderInfo>, String>> {
        let lines: Vec<&str> = email_body.lines().collect();

        let sections = extract_order_sections(&lines);
        if sections.is_empty() {
            return None; // [注文番号]セクションなし → parse() にフォールバック
        }

        let delivery_address = extract_delivery_address(&lines);
        let delivery_info = extract_delivery_info(&lines);

        let mut orders = Vec::new();
        for (order_number, items) in sections {
            if items.is_empty() {
                continue;
            }
            orders.push(OrderInfo {
                order_number,
                order_date: None,
                delivery_address: delivery_address.clone(),
                delivery_info: delivery_info.clone(),
                items,
                // 複数注文の合算金額は個別注文に分配できないため None とする
                subtotal: None,
                shipping_fee: None,
                total_amount: None,
            });
        }

        if orders.is_empty() {
            Some(Err("No orders found in [注文番号] sections".to_string()))
        } else {
            Some(Ok(orders))
        }
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

/// [ご購入内容]セクション内の[注文番号]ごとに商品を分割して返す。
/// 戻り値: Vec<(注文番号, 商品リスト)>
/// [注文番号]行が1つも見つからない場合は空 Vec を返す。
fn extract_order_sections(lines: &[&str]) -> Vec<(String, Vec<OrderItem>)> {
    let order_number_pattern = match Regex::new(r"\[注文番号\]\s*(\d+-\d+-\d+)") {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let price_pattern =
        match Regex::new(r"単価：([\d,]+)円\s*×\s*個数：(\d+)\s*=\s*([\d,]+)円") {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };

    let mut sections: Vec<(String, Vec<OrderItem>)> = Vec::new();
    let mut in_purchase_section = false;
    let mut current_order_number: Option<String> = None;
    let mut current_items: Vec<OrderItem> = Vec::new();

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
            if let Some(num) = current_order_number.take() {
                sections.push((num, std::mem::take(&mut current_items)));
            }
            break;
        }

        if !in_purchase_section {
            i += 1;
            continue;
        }

        // [注文番号]行の検出
        if let Some(caps) = order_number_pattern.captures(line) {
            // 前のセクションを保存
            if let Some(num) = current_order_number.take() {
                sections.push((num, std::mem::take(&mut current_items)));
            }
            current_order_number =
                Some(caps.get(1).unwrap().as_str().to_string());
            i += 1;
            continue;
        }

        // 現在のセクション内の商品を解析
        if current_order_number.is_some() && !line.is_empty() && !line.starts_with("単価：") {
            if i + 1 < lines.len() {
                let next_line = lines[i + 1].trim();
                if let Some(captures) = price_pattern.captures(next_line) {
                    let (name, manufacturer, model_number) = parse_item_line(line);
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

                    current_items.push(OrderItem {
                        name,
                        manufacturer,
                        model_number,
                        unit_price,
                        quantity,
                        subtotal,
                        image_url: None,
                    });

                    i += 2;
                    continue;
                }
            }
        }

        i += 1;
    }

    // ループ終了後に残ったセクションを保存（区切り線がない場合）
    if let Some(num) = current_order_number {
        sections.push((num, current_items));
    }

    sections
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
                            image_url: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hobbysearch_send() {
        // NOTE: `sample/` 配下のファイルは使わず、テスト内でダミー本文を生成する。
        let sample_email = r#"[代表注文番号] 25-0807-1624

[商品お届け先]
山田 太郎 様
〒812-0044 福岡県テスト市1-2-3

[運送会社] 佐川急便
[配送伝票] 470550808943

[ご購入内容]
マックスファクトリー 014554 PLAMAX BP-02 ソフィア・F・シャーリング 虎アーマーVer. (プラモデル)
単価：9,350円 × 個数：1 = 9,350円
メーカー2 0002 商品2 (プラモデル)
単価：12,000円 × 個数：1 = 12,000円
メーカー3 0003 商品3 (ディスプレイ)
単価：8,000円 × 個数：1 = 8,000円
メーカー4 0004 商品4 (プラモデル)
単価：7,000円 × 個数：1 = 7,000円
メーカー5 0005 商品5 (プラモデル)
単価：6,000円 × 個数：1 = 6,000円
メーカー6 0006 商品6 (プラモデル)
単価：2,312円 × 個数：2 = 4,624円

小計 46,974円
送料 0円
合計 46,974円
"#;
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
        assert_eq!(address.name, "山田 太郎");
        assert_eq!(address.postal_code, Some("812-0044".to_string()));
    }

    /// parse_multi は [注文番号]セクションが存在しない場合 None を返す（parse() へフォールバック）
    #[test]
    fn test_parse_multi_returns_none_without_order_number_sections() {
        let sample_email = r#"[代表注文番号] 25-0807-1624

[運送会社] 佐川急便
[配送伝票] 470550808943

[ご購入内容]
メーカー1 0001 商品1 (プラモデル)
単価：1,000円 × 個数：1 = 1,000円

小計 1,000円
"#;
        let parser = HobbySearchSendParser;
        assert!(parser.parse_multi(sample_email).is_none());
    }

    /// 複数注文の同時発送: 各[注文番号]ごとに OrderInfo が生成される
    #[test]
    fn test_parse_multi_multiple_orders() {
        let sample_email = r#"このたびはホビーサーチをご利用いただき、誠にありがとうございます。

[商品お届け先]
テスト 太郎 様
〒100-0001 東京都テスト区1-1-1

[運送会社] 佐川急便
[配送日] 最短発送(指定なし)
[配送時間] 指定なし
[配送伝票] 999000111222
[運送会社URL]
http://k2k.sagawa-exp.co.jp/p/sagawa/web/okurijoinput.jsp

*****************************************************************
発送内容
*****************************************************************
[代表注文番号] 25-0101-0001
[決済方法] クレジットカード
[運送会社] 佐川急便
[ご購入内容]
[注文番号] 25-0101-0001
メーカーA A001 テスト商品1 (プラモデル) テストシリーズ
単価：627円 × 個数：1 = 627円
メーカーB B001 テスト商品2 (プラモデル) テストシリーズ
単価：940円 × 個数：1 = 940円
メーカーC C001 テスト商品3 (プラモデル) テストシリーズ
単価：990円 × 個数：1 = 990円
メーカーD D001 テスト商品4 (プラモデル) テストシリーズ
単価：990円 × 個数：1 = 990円
[注文番号] 25-0102-0002
メーカーE E001 テスト商品5 (プラモデル) テストシリーズ
単価：2,695円 × 個数：1 = 2,695円
小計　　　　　　　　     6,242円
送料　　　　　　　　       660円
合計　　　　　　　　     6,902円
"#;

        let parser = HobbySearchSendParser;
        let result = parser.parse_multi(sample_email);

        assert!(result.is_some(), "parse_multi should return Some");
        let orders = result.unwrap().expect("parse_multi should succeed");

        // 2注文分の OrderInfo が返される
        assert_eq!(orders.len(), 2);

        // 1件目: 25-0101-0001（商品4点）
        let order1 = &orders[0];
        assert_eq!(order1.order_number, "25-0101-0001");
        assert_eq!(order1.items.len(), 4);
        assert_eq!(order1.items[0].unit_price, 627);
        assert_eq!(order1.items[1].unit_price, 940);
        assert_eq!(order1.items[2].unit_price, 990);
        assert_eq!(order1.items[3].unit_price, 990);

        // 2件目: 25-0102-0002（商品1点）
        let order2 = &orders[1];
        assert_eq!(order2.order_number, "25-0102-0002");
        assert_eq!(order2.items.len(), 1);
        assert_eq!(order2.items[0].unit_price, 2695);

        // 配送情報は両注文で共有される
        for order in &orders {
            let delivery = order.delivery_info.as_ref().expect("delivery_info should be present");
            assert_eq!(delivery.carrier, "佐川急便");
            assert_eq!(delivery.tracking_number, "999000111222");
        }

        // 合算金額は個別注文に分配しないため None
        for order in &orders {
            assert!(order.subtotal.is_none());
            assert!(order.shipping_fee.is_none());
            assert!(order.total_amount.is_none());
        }
    }

    /// 単一注文で[注文番号]セクションがある場合: 1件の OrderInfo を返す
    #[test]
    fn test_parse_multi_single_order_with_order_number_section() {
        let sample_email = r#"[代表注文番号] 25-0807-1624

[運送会社] 佐川急便
[配送伝票] 470550808943

[ご購入内容]
[注文番号] 25-0807-1624
メーカー1 0001 商品A (プラモデル)
単価：1,000円 × 個数：1 = 1,000円
メーカー2 0002 商品B (プラモデル)
単価：2,000円 × 個数：2 = 4,000円

小計 5,000円
"#;
        let parser = HobbySearchSendParser;
        let result = parser.parse_multi(sample_email);

        assert!(result.is_some());
        let orders = result.unwrap().expect("should succeed");

        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].order_number, "25-0807-1624");
        assert_eq!(orders[0].items.len(), 2);

        let delivery = orders[0].delivery_info.as_ref().unwrap();
        assert_eq!(delivery.tracking_number, "470550808943");
    }
}
