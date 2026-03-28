use super::{body_to_lines, extract_items, extract_order_number};
use crate::parsers::{EmailParser, OrderInfo};

/// 駿河屋 注文確認メール用パーサー
///
/// 件名：`ご注文ありがとうございます` を含む
/// 送信元：`order@suruga-ya.jp`
///
/// 取引番号は `取引番号:S2204166697` 形式（`S` + 10桁）。
/// 注文日は本文に含まれないため、`dispatch()` 側で `apply_internal_date()` を使用する。
/// 合計・送料は注文確認メールには記載なし。
pub struct SurugayaConfirmParser;

impl EmailParser for SurugayaConfirmParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let body_lines = body_to_lines(email_body);
        let lines: Vec<&str> = body_lines.iter().map(|s| s.as_str()).collect();

        let order_number =
            extract_order_number(&lines).ok_or_else(|| "Order number not found".to_string())?;

        let items = extract_items(&lines);
        if items.is_empty() {
            return Err("No items found".to_string());
        }

        Ok(OrderInfo {
            order_number,
            order_date: None, // internal_date で補完
            delivery_address: None,
            delivery_info: None,
            items,
            subtotal: None,
            shipping_fee: None,
            total_amount: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_confirm() -> &'static str {
        r#"この度は駿河屋にご注文頂き、誠にありがとうございます。

下記商品のご注文を承りました。
ご注文内容を確認いたしまして、商品の確保・検品に入らせて頂きます。


取引番号:S2204166697
山田 太郎様  ご注文の商品
──────────────────────────────────
1-1 \1,656 中古プラモデル 1/144 HG グレイズアイン 「機動戦士ガンダム 鉄血のオルフェンズ」 (603103980001)
1-2 \828 中古プラモデル 1/144 HGBC ジャイアントガトリング 「ガンダムビルドファイターズトライ」 (603055964001)
1-3 \644 中古プラモデル 1/144 HGBC ボールデンアームアームズ 「ガンダムビルドファイターズトライ」 (603102455001)
1-4 \368 中古プラモデル 1/144 長距離狙撃用オプションアーマー(アルト用/ダークグレー) 「30 MINUTES MISSIONS」 (603100256001)
1-5 \1,472 中古プラモデル 1/144 HG MBF-P02 ガンダムアストレイ レッドフレーム(フライトユニット装備) 「機動戦士ガンダムSEED DESTINY ASTRAY」 (603098529001)
1-6 \1,748 中古プラモデル 1/144 HGBC ティルトローターパック 「ガンダムビルドダイバーズ」 (603089459001)
1-7 \1,104 中古プラモデル 1/144 HGAC GUNPLA EVOLUTION PROJECT OZ-06MS リーオー 「新機動戦記ガンダムW」 (603089200001)
1-8 \1,288 中古プラモデル 1/144 HG GN-000 オーガンダム(実戦配備型) 「機動戦士ガンダム00」 [5055732] (603101318001)
──────────────────────────────────
[支払方法] クレジット
"#
    }

    #[test]
    fn test_parse_confirm_order_number() {
        let order = SurugayaConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.order_number, "S2204166697");
    }

    #[test]
    fn test_parse_confirm_no_order_date() {
        let order = SurugayaConfirmParser.parse(sample_confirm()).unwrap();
        assert!(order.order_date.is_none());
    }

    #[test]
    fn test_parse_confirm_item_count() {
        let order = SurugayaConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items.len(), 8);
    }

    #[test]
    fn test_parse_confirm_first_item_price() {
        let order = SurugayaConfirmParser.parse(sample_confirm()).unwrap();
        assert_eq!(order.items[0].unit_price, 1656);
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[0].subtotal, 1656);
    }

    #[test]
    fn test_parse_confirm_first_item_name_strips_code() {
        let order = SurugayaConfirmParser.parse(sample_confirm()).unwrap();
        let name = &order.items[0].name;
        assert!(name.contains("グレイズアイン"));
        // 末尾の商品コード (603103980001) が除去されていること
        assert!(!name.contains("603103980001"));
    }

    #[test]
    fn test_parse_confirm_item_with_bracket_strips_both() {
        // item 1-8: `[5055732] (603101318001)` が両方除去されること
        let order = SurugayaConfirmParser.parse(sample_confirm()).unwrap();
        let name = &order.items[7].name;
        assert!(name.contains("オーガンダム"));
        assert!(!name.contains("5055732"));
        assert!(!name.contains("603101318001"));
    }

    #[test]
    fn test_parse_confirm_item_with_paren_in_name() {
        // item 1-4: `(アルト用/ダークグレー)` が商品名として保持されること
        let order = SurugayaConfirmParser.parse(sample_confirm()).unwrap();
        let name = &order.items[3].name;
        assert!(name.contains("アルト用/ダークグレー"));
        assert_eq!(order.items[3].unit_price, 368);
    }

    #[test]
    fn test_parse_confirm_no_subtotal() {
        let order = SurugayaConfirmParser.parse(sample_confirm()).unwrap();
        assert!(order.subtotal.is_none());
        assert!(order.shipping_fee.is_none());
        assert!(order.total_amount.is_none());
    }

    #[test]
    fn test_parse_confirm_no_delivery_info() {
        let order = SurugayaConfirmParser.parse(sample_confirm()).unwrap();
        assert!(order.delivery_info.is_none());
    }

    #[test]
    fn test_parse_confirm_no_order_number_returns_error() {
        let body = "1-1 \\1,000 商品A (600000000001)";
        assert!(SurugayaConfirmParser.parse(body).is_err());
    }

    #[test]
    fn test_parse_confirm_no_items_returns_error() {
        let body = "取引番号:S2204166697\n\nお問い合わせはこちら";
        assert!(SurugayaConfirmParser.parse(body).is_err());
    }
}
