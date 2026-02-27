use super::{extract_amounts, extract_order_date, parse_item_line};
use crate::parsers::{EmailParser, OrderInfo};

/// キッズドラゴン 注文確認メール用パーサー
///
/// キッズドラゴンの注文確認メールには注文番号フィールドが存在しないため、
/// 受注日時（`YYYY-MM-DD HH:MM` 形式）を `order_number` として使用する。
pub struct KidsDragonConfirmParser;

impl EmailParser for KidsDragonConfirmParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let lines: Vec<&str> = email_body.lines().collect();

        let items: Vec<_> = lines
            .iter()
            .filter_map(|line| parse_item_line(line))
            .collect();

        if items.is_empty() {
            return Err("No items found".to_string());
        }

        let (subtotal, shipping_fee, total_amount) = extract_amounts(&lines);

        let order_date =
            extract_order_date(&lines).ok_or("Order date not found (used as order number)")?;

        Ok(OrderInfo {
            order_number: order_date.clone(),
            order_date: Some(order_date),
            delivery_address: None,
            delivery_info: None,
            items,
            subtotal,
            shipping_fee,
            total_amount,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// サンプル確認メール（sample/原田裕基様 ご注文有難うございます キッズドラゴンです.eml の
    /// UTF-8 デコード後の本文。全角コロン `：` を含む実際のフォーマットを使用する）
    fn sample_confirm_email() -> &'static str {
        r#"【ご注文ご確認メール】  ホビーショップ　キッズドラゴン

[商品名]：バンダイ ノンスケール ＳＤ ＥＸ-スタンダードシリーズ No.004 XXXG-00W0 ウィングガンダムゼロ ＥＷ[bd-sdex-004]       594 円 x  1 個       594 円
[商品名]：バンダイ ビルダーズ パーツ ＨＤ ノンスケール ＭＳパネル ０１[bd-sdcs-019]       550 円 x  1 個       550 円
[商品名]：コトブキヤ ウェポンユニット MW-035 エネルギーシールド[wu-mw-35]       660 円 x  1 個       660 円
[商品名]：バンダイ 30MS OB-12 オプションボディパーツ アームパーツ＆レッグパーツ［ホワイト/ブラック］[bd-30ms-ob012]       990 円 x  1 個       990 円
[商品名]：バンダイ 30MS OB-11 オプションボディパーツ アームパーツ&レッグパーツ[カラーC][bd-30ms-ob11]       880 円 x  1 個       880 円
[商品名]：バンダイ ノンスケール 30MM W-09 オプションパーツセット 3[bd-30mm-w09]       594 円 x  1 個       594 円
  商品小計             4,268 円
  送料                   1,200 円
  商品合計             4,268 円
  送料合計             1,200 円
  合計                   5,468 円

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
○ お支払いについて

  金額        : 5,468 円
  方法        : 銀行振込

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
○ 受注日時
  2023年6月15日 02:17
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
"#
    }

    #[test]
    fn test_parse_confirm_item_count() {
        let order = KidsDragonConfirmParser
            .parse(sample_confirm_email())
            .unwrap();
        assert_eq!(order.items.len(), 6);
    }

    #[test]
    fn test_parse_confirm_first_item() {
        let order = KidsDragonConfirmParser
            .parse(sample_confirm_email())
            .unwrap();
        let item = &order.items[0];
        assert!(item.name.contains("ウィングガンダムゼロ"));
        assert_eq!(item.model_number, Some("bd-sdex-004".to_string()));
        assert_eq!(item.unit_price, 594);
        assert_eq!(item.quantity, 1);
        assert_eq!(item.subtotal, 594);
    }

    #[test]
    fn test_parse_confirm_item_with_brackets_in_name() {
        // 商品名に [カラーC] を含む場合でも SKU が正しく抽出される
        let order = KidsDragonConfirmParser
            .parse(sample_confirm_email())
            .unwrap();
        let item = &order.items[4]; // bd-30ms-ob11
        assert!(item.name.contains("[カラーC]"));
        assert_eq!(item.model_number, Some("bd-30ms-ob11".to_string()));
        assert_eq!(item.unit_price, 880);
    }

    #[test]
    fn test_parse_confirm_amounts() {
        let order = KidsDragonConfirmParser
            .parse(sample_confirm_email())
            .unwrap();
        assert_eq!(order.subtotal, Some(4268));
        assert_eq!(order.shipping_fee, Some(1200));
        assert_eq!(order.total_amount, Some(5468));
    }

    #[test]
    fn test_parse_confirm_order_date_as_order_number() {
        let order = KidsDragonConfirmParser
            .parse(sample_confirm_email())
            .unwrap();
        assert_eq!(order.order_date, Some("2023-06-15 02:17".to_string()));
        assert_eq!(order.order_number, "2023-06-15 02:17");
    }

    #[test]
    fn test_parse_confirm_no_delivery_info() {
        let order = KidsDragonConfirmParser
            .parse(sample_confirm_email())
            .unwrap();
        assert!(order.delivery_info.is_none());
    }

    #[test]
    fn test_parse_confirm_no_items_returns_error() {
        let result = KidsDragonConfirmParser.parse("本文なし\n2023年6月15日 02:17");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_confirm_no_date_returns_error() {
        let body = "[商品名]:テスト商品[sku-001]       594 円 x  1 個       594 円\n";
        let result = KidsDragonConfirmParser.parse(body);
        assert!(result.is_err());
    }
}
