//! DMM通販「ご注文分割完了のお知らせ」メール用パーサー
//!
//! 送信元: info@mail.dmm.com
//! 件名: DMM通販：ご注文分割完了のお知らせ
//!
//! 1通のメールに複数の分割後注文が含まれるため、parse_multi で Vec<OrderInfo> を返す。

use super::{EmailParser, OrderInfo, OrderItem};
use regex::Regex;

/// 商品名から【○月再生産分】等のプレフィックスを除去（dmm_confirm と同様）
fn normalize_product_name(name: &str) -> String {
    let mut s = name.trim().to_string();
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
    s.trim().to_string()
}

pub struct DmmSplitCompleteParser;

impl EmailParser for DmmSplitCompleteParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        match self.parse_multi(email_body) {
            Some(Ok(orders)) if !orders.is_empty() => Ok(orders.into_iter().next().unwrap()),
            Some(Ok(_)) => Err("No orders found in split completion email".to_string()),
            Some(Err(e)) => Err(e),
            None => Err("parse_multi not implemented".to_string()),
        }
    }

    fn parse_multi(&self, email_body: &str) -> Option<Result<Vec<OrderInfo>, String>> {
        Some(parse_split_orders(email_body))
    }
}

/// 本文を「注文番号:」で区切り、各ブロックから OrderInfo を構築する
fn parse_split_orders(body: &str) -> Result<Vec<OrderInfo>, String> {
    let order_number_re =
        Regex::new(r"注文番号\s*[：:]\s*([A-Za-z]{2}-\d+)").map_err(|e| e.to_string())?;
    // [10月発送予定] 商品名 1個 594円 または 商品名 1個 1,100円
    let item_re = Regex::new(r"^(?:\[\d+月発送予定\]\s*)?(.+?)\s+(\d+)個\s*([\d,]+)円\s*$")
        .map_err(|e| e.to_string())?;
    let shipping_re = Regex::new(r"送料\s*[：:]\s*([\d,]+)円").map_err(|e| e.to_string())?;

    let mut orders = Vec::new();
    // 「注文番号」で分割（最初の区切りは「分割後のご注文内容」等で空になりうる）
    let blocks: Vec<&str> = body
        .split("注文番号")
        .filter(|s| !s.trim().is_empty())
        .collect();

    for block in blocks {
        let block = block.trim();
        let lines: Vec<&str> = block
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if lines.is_empty() {
            continue;
        }

        // 先頭行が "： KC-xxxxx" 形式（split("注文番号") で "注文番号" が外れている）
        let order_number = lines.first().and_then(|first| {
            let with_prefix = format!("注文番号{}", first);
            order_number_re
                .captures(&with_prefix)
                .or_else(|| order_number_re.captures(first))
                .and_then(|cap| cap.get(1))
                .map(|m| m.as_str().to_string())
        });

        let order_number = match order_number {
            Some(n) => n,
            None => continue,
        };

        let mut items: Vec<OrderItem> = Vec::new();
        let mut shipping_fee: Option<i64> = None;

        for line in &lines[1..] {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(cap) = shipping_re.captures(line) {
                if let Some(m) = cap.get(1) {
                    if let Ok(fee) = m.as_str().replace(',', "").parse::<i64>() {
                        shipping_fee = Some(fee);
                    }
                }
                continue;
            }
            if let Some(cap) = item_re.captures(line) {
                if let (Some(name), Some(qty), Some(price)) = (cap.get(1), cap.get(2), cap.get(3)) {
                    let name = normalize_product_name(name.as_str());
                    if name.len() < 2 {
                        continue;
                    }
                    if let (Ok(q), Ok(p)) = (
                        qty.as_str().parse::<i64>(),
                        price.as_str().replace(',', "").parse::<i64>(),
                    ) {
                        if p > 0 && q > 0 {
                            items.push(OrderItem {
                                name,
                                manufacturer: None,
                                model_number: None,
                                unit_price: p,
                                quantity: q,
                                subtotal: p * q,
                                image_url: None,
                            });
                        }
                    }
                }
            }
        }

        if !items.is_empty() {
            let subtotal: i64 = items.iter().map(|i| i.subtotal).sum();
            let total = shipping_fee.map(|s| subtotal + s).unwrap_or(subtotal);
            orders.push(OrderInfo {
                order_number,
                order_date: None,
                delivery_address: None,
                delivery_info: None,
                items,
                subtotal: Some(subtotal),
                shipping_fee,
                total_amount: Some(total),
            });
        }
    }

    if orders.is_empty() {
        Err("No valid split orders found".to_string())
    } else {
        Ok(orders)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dmm_split_complete() {
        // サンプル .eml は ISO-2022-JP のため、UTF-8 の本文を模したテスト用テキストで検証
        let body_utf8 = r#"注文 完了 メール

DMM通販をご利用いただき、ありがとうございます。ご注文の分割手続きが完了いたしました。

■■ 分割後のご注文内容 ■■

注文番号: KC-23812833
!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
[10月発送予定] SDEX ガンダム 1個 594円
[11月発送予定] HGSEED 1/144 R03 ガンダム 1個 1,100円
送料: 530円
!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

注文番号: KC-23945758
!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
[3月発送予定] HGSEED 1/144 ガンダム 1個 1,650円
送料: 530円
!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
"#;
        let parser = DmmSplitCompleteParser;
        let result = parser.parse_multi(body_utf8).unwrap();
        assert!(result.is_ok());
        let orders = result.unwrap();
        assert_eq!(orders.len(), 2);
        assert_eq!(orders[0].order_number, "KC-23812833");
        assert_eq!(orders[0].items.len(), 2);
        assert_eq!(orders[0].shipping_fee, Some(530));
        assert_eq!(orders[1].order_number, "KC-23945758");
        assert_eq!(orders[1].items.len(), 1);
        assert_eq!(orders[1].shipping_fee, Some(530));
    }
}
