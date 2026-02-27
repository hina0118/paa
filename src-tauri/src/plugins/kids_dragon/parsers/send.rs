use super::{extract_amounts, extract_order_date, parse_item_line};
use crate::parsers::{EmailParser, OrderInfo};

/// キッズドラゴン 発送通知メール用パーサー
///
/// confirm と同一の商品行フォーマットを使用する。
/// 発送通知には追跡番号が含まれないため `delivery_info` は設定しない。
/// 分割発送の場合、発送通知の商品リストは注文確認より少なくなる場合がある。
pub struct KidsDragonSendParser;

impl EmailParser for KidsDragonSendParser {
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
            delivery_info: None, // 追跡番号なし
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

    /// サンプル発送通知メール（sample/商品の発送が完了致しました.eml の
    /// UTF-8 デコード後の本文。全角コロン `：` を含む実際のフォーマットを使用する）
    fn sample_send_email() -> &'static str {
        r#"商品の発送が完了致しました

ホビーショップ　キッズドラゴン

[商品名]：バンダイ ビルダーズ パーツ ＨＤ ノンスケール ＭＳパネル ０１[bd-sdcs-019]       550 円 x  1 個       550 円
[商品名]：コトブキヤ ウェポンユニット MW-035 エネルギーシールド[wu-mw-35]       660 円 x  1 個       660 円
  商品小計             2,684 円
  送料                   1,200 円
  商品合計             2,684 円
  送料合計             1,200 円
  合計                   3,884 円

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
○ 送り主、お届け先情報

  送り主    : ＨＯＢＢＹ ＳＨＯＰ キッズドラゴン
  発送方法  : ヤマト宅急便

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
○ 受注日時
  2023年6月15日 02:17
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
"#
    }

    #[test]
    fn test_parse_send_item_count() {
        let order = KidsDragonSendParser.parse(sample_send_email()).unwrap();
        // 分割発送のため注文確認より少ない 2 商品
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_send_first_item() {
        let order = KidsDragonSendParser.parse(sample_send_email()).unwrap();
        let item = &order.items[0];
        assert!(item.name.contains("ＭＳパネル"));
        assert_eq!(item.model_number, Some("bd-sdcs-019".to_string()));
        assert_eq!(item.unit_price, 550);
        assert_eq!(item.quantity, 1);
        assert_eq!(item.subtotal, 550);
    }

    #[test]
    fn test_parse_send_amounts() {
        let order = KidsDragonSendParser.parse(sample_send_email()).unwrap();
        assert_eq!(order.subtotal, Some(2684));
        assert_eq!(order.shipping_fee, Some(1200));
        assert_eq!(order.total_amount, Some(3884));
    }

    #[test]
    fn test_parse_send_order_date_as_order_number() {
        let order = KidsDragonSendParser.parse(sample_send_email()).unwrap();
        assert_eq!(order.order_date, Some("2023-06-15 02:17".to_string()));
        assert_eq!(order.order_number, "2023-06-15 02:17");
    }

    #[test]
    fn test_parse_send_no_delivery_info() {
        // 追跡番号なし
        let order = KidsDragonSendParser.parse(sample_send_email()).unwrap();
        assert!(order.delivery_info.is_none());
    }

    #[test]
    fn test_parse_send_no_items_returns_error() {
        let result = KidsDragonSendParser.parse("本文なし\n2023年6月15日 02:17");
        assert!(result.is_err());
    }
}
