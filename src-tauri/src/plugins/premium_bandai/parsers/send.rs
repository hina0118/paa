//! プレミアムバンダイ 商品発送完了メール用パーサー
//!
//! 件名：`商品発送完了のお知らせ`
//! 送信元：`evidence_bc@p-bandai.jp`
//!
//! フォーマット: multipart/alternative（テキスト + HTML）
//! 発送日・追跡番号・配送業者を抽出し、`delivery_info` に格納する。
//! 金額情報は含まれないため、confirm で登録済みの価格を保持する。
//!
//! ## 対応フォーマット
//!
//! ### プレーンテキスト形式
//! - 商品セクション: `■発送商品` / `■ご注文商品`
//! - 数量: `数量：N個`
//! - 配送業者: `配送業者：佐川急便`（構造化行）
//! - 追跡番号: `お問い合わせ番号：NNNN`（同一行）
//!
//! ### HTML メール形式（実際のプレミアムバンダイメール）
//! - 商品セクション: `購入した商品`
//! - 数量: `&times;N`（HTML エンティティ、`body_to_lines` でデコードされない）
//! - 配送業者: 本文テキスト内（例: `佐川急便にてご注文商品を発送`）
//! - 追跡番号: `お問合せ伝票番号`（`い` なし）ラベルの次行に数字のみ

use super::{
    body_to_lines, carrier_tracking_url, extract_carrier, extract_order_date, extract_order_number,
    extract_send_date, extract_tracking_number, normalize_product_name,
};
use crate::parsers::{DeliveryInfo, EmailParser, OrderInfo, OrderItem};
use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};

/// 数量表現
///
/// - `数量：1個` / `個数：1個`（プレーンテキスト形式）
/// - `×1`（Unicode 乗算記号 U+00D7）
/// - `&times;1`（HTML エンティティ、`body_to_lines` でデコードされない）
static SEND_QUANTITY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:(?:数量|個数)[：:]\s*(\d+)個?|×(\d+)|&times;(\d+))")
        .expect("Invalid SEND_QUANTITY_RE")
});

/// 発送商品セクション開始行を検出するパターン（行全体がセクションヘッダーであること）
///
/// `^...$` アンカーにより「佐川急便にてご注文商品を発送させていただきました」のような
/// 文章行には **マッチしない**。対応形式:
/// - `■発送商品` / `■ご注文商品`（プレーンテキスト形式）
/// - `購入した商品` / `ご注文商品`（HTML テーブル形式の単独行）
/// - `【ご注文明細】` / `ご注文明細`（HTML メール形式の発送通知）
static SEND_ITEM_SECTION_START_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:■|【)?(?:発送商品|ご注文商品|購入した商品|ご?注文明細)(?:】\s*)?$")
        .expect("Invalid SEND_ITEM_SECTION_START_RE")
});

/// HTML パースで使用する数量正規表現
///
/// scraper の `text()` は HTML エンティティをデコードするため、`&times;` → `×` に変換済みで届く。
static SEND_HTML_QTY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^×(\d+)$").expect("Invalid SEND_HTML_QTY_RE")
});

/// プレミアムバンダイ 発送通知メール用パーサー
pub struct PremiumBandaiSendParser;

impl EmailParser for PremiumBandaiSendParser {
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String> {
        let body_lines = body_to_lines(email_body);
        let lines: Vec<&str> = body_lines.iter().map(|s| s.as_str()).collect();

        let order_number =
            extract_order_number(&lines).ok_or_else(|| "Order number not found".to_string())?;

        // 発送日を order_date として使用（`発送日：YYYY年M月D日` 優先、なければ `注文日` を使用）
        let order_date = extract_send_date(&lines).or_else(|| extract_order_date(&lines));

        // HTML 形式ではまず HTML パーサで商品を抽出し、見つからなければテキストベースのパーサにフォールバックする
        let items = {
            let html_items = extract_items_from_send_html(email_body);
            if !html_items.is_empty() {
                html_items
            } else {
                extract_send_items(&lines)
            }
        };
        if items.is_empty() {
            return Err("No items found".to_string());
        }

        let tracking_number = extract_tracking_number(&lines)
            .ok_or_else(|| "Tracking number not found".to_string())?;

        let carrier =
            extract_carrier(&lines).ok_or_else(|| "Carrier not found".to_string())?;

        let carrier_url = carrier_tracking_url(&carrier, &tracking_number);

        let delivery_info = DeliveryInfo {
            carrier,
            tracking_number,
            delivery_date: None,
            delivery_time: None,
            carrier_url,
        };

        Ok(OrderInfo {
            order_number,
            order_date,
            delivery_address: None,
            delivery_info: Some(delivery_info),
            items,
            subtotal: None,
            shipping_fee: None,
            total_amount: None,
        })
    }
}

/// 発送通知メールから商品リストを抽出する
///
/// プレーンテキスト形式と HTML メール形式の両方に対応する。
/// 商品名行 → 数量行（省略可、省略時は 1 個）のパターンを想定する。
fn extract_send_items(lines: &[&str]) -> Vec<OrderItem> {
    let mut items: Vec<OrderItem> = Vec::new();
    let mut pending_name: Option<String> = None;
    let mut in_item_section = false;

    for line in lines {
        let trimmed = line.trim();

        // 発送商品セクション開始マーカー（プレーンテキスト・HTML 両対応）
        // 行全体がセクションヘッダーの場合のみマッチ（文章中の部分一致は除外）
        if SEND_ITEM_SECTION_START_RE.is_match(trimmed) {
            in_item_section = true;
            continue;
        }

        if !in_item_section {
            continue;
        }

        // 配送情報セクションで商品セクション終了
        if trimmed.contains("配送情報")
            || trimmed.contains("配送業者")
            || trimmed.contains("お問い合わせ番号")
            || trimmed.contains("お問合せ伝票番号")
            || trimmed.contains("お問い合わせ伝票番号")
            || trimmed.contains("追跡番号")
        {
            if let Some(name) = pending_name.take() {
                items.push(OrderItem {
                    name,
                    manufacturer: None,
                    model_number: None,
                    unit_price: 0,
                    quantity: 1,
                    subtotal: 0,
                    image_url: None,
                });
            }
            break;
        }

        // 区切り行・空行はスキップ
        if trimmed.is_empty() || trimmed.starts_with("---") || trimmed.starts_with("===") {
            continue;
        }

        // 数量行（`数量：N個` / `×N` / `&times;N`）
        if let Some(caps) = SEND_QUANTITY_RE.captures(trimmed) {
            let qty: i64 = caps
                .get(1)
                .or_else(|| caps.get(2))
                .or_else(|| caps.get(3))
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(1);

            if let Some(name) = pending_name.take() {
                items.push(OrderItem {
                    name,
                    manufacturer: None,
                    model_number: None,
                    unit_price: 0,
                    quantity: qty,
                    subtotal: 0,
                    image_url: None,
                });
            }
            continue;
        }

        // 注文番号・日付などのヘッダー行はスキップ
        if trimmed.starts_with("ご注文番号")
            || trimmed.starts_with("注文番号")
            || trimmed.starts_with("ご注文日")
            || trimmed.starts_with("注文日")
            || trimmed.starts_with("発送日")
            || trimmed.starts_with("お支払")
        {
            continue;
        }

        // 前の商品が未確定なら確定
        if let Some(name) = pending_name.take() {
            items.push(OrderItem {
                name,
                manufacturer: None,
                model_number: None,
                unit_price: 0,
                quantity: 1,
                subtotal: 0,
                image_url: None,
            });
        }

        // 新しい商品名候補
        if !trimmed.is_empty() {
            pending_name = Some(normalize_product_name(trimmed));
        }
    }

    // 末尾の未確定商品を確定
    if let Some(name) = pending_name {
        items.push(OrderItem {
            name,
            manufacturer: None,
            model_number: None,
            unit_price: 0,
            quantity: 1,
            subtotal: 0,
            image_url: None,
        });
    }

    items
}

/// HTML パートから発送通知メールの商品リストを抽出する
///
/// プレミアムバンダイの発送通知メール HTML 形式:
/// - 商品テーブル: `<td><strong>商品名</strong></td>` を含む `<table>`
/// - 数量: `<td align="right">×N</td>`（scraper が `&times;` を `×` にデコード済み）
/// - 外側のラッパーテーブルは `<td> > <strong>` が複数あるためスキップ
///
/// HTML でない入力（`<table>` が存在しない場合）は空リストを返す。
fn extract_items_from_send_html(html: &str) -> Vec<OrderItem> {
    let document = Html::parse_document(html);
    let table_sel = Selector::parse("table").unwrap();
    let td_strong_sel = Selector::parse("td > strong").unwrap();
    let qty_td_sel = Selector::parse("td[align='right']").unwrap();
    let img_sel = Selector::parse("img").unwrap();
    let mut items = Vec::new();

    for table in document.select(&table_sel) {
        let mut strongs = table.select(&td_strong_sel);
        let strong = match strongs.next() {
            Some(s) => s,
            None => continue,
        };
        // 外側のテーブル（複数の <td> > <strong> を含む）をスキップ
        if strongs.next().is_some() {
            continue;
        }

        let name = normalize_product_name(strong.text().collect::<String>().trim());
        if name.is_empty() {
            continue;
        }

        // 数量マーカー（`<td align="right">×N</td>`）がない = 商品テーブルでない（追跡番号テーブル等）のでスキップ
        let quantity = match table
            .select(&qty_td_sel)
            .find_map(|td| {
                let text = td.text().collect::<String>();
                SEND_HTML_QTY_RE
                    .captures(text.trim())
                    .and_then(|c| c[1].parse::<i64>().ok())
            }) {
            Some(q) => q,
            None => continue,
        };

        let image_url = table
            .select(&img_sel)
            .next()
            .and_then(|img| img.value().attr("src").map(|s| s.to_string()));

        items.push(OrderItem {
            name,
            manufacturer: None,
            model_number: None,
            unit_price: 0,
            quantity,
            subtotal: 0,
            image_url,
        });
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_send_plain() -> &'static str {
        r#"商品を発送しました。

■ご注文番号：12345
■発送日：2025年1月20日

■発送商品
figma テスト【再販】
数量：1個

■配送情報
配送業者：佐川急便
お問い合わせ番号：123456789012
"#
    }

    fn sample_send_multiple_items() -> &'static str {
        r#"■ご注文番号：12345
■発送日：2025年2月5日

■発送商品
商品A
数量：1個
商品B【2025年4月発送】
数量：2個

■配送情報
配送業者：ヤマト運輸
お問い合わせ番号：987654321098
"#
    }

    fn sample_send_yamato() -> &'static str {
        r#"■ご注文番号：12345
■発送日：2025年1月20日

■発送商品
商品X

■配送情報
配送業者：ヤマト運輸
お問い合わせ番号：123456789012
"#
    }

    fn sample_send_yupack() -> &'static str {
        r#"■ご注文番号：12345
■発送日：2025年1月20日

■発送商品
商品Y

■配送情報
配送業者：ゆうパック
お問い合わせ番号：111222333444
"#
    }

    /// 実際のプレミアムバンダイ HTML メールを body_to_lines 処理した後の形式を模したサンプル
    ///
    /// 特徴:
    /// - 配送業者は本文テキストから検出（`佐川急便にて`）
    /// - 追跡番号はラベル（`お問合せ伝票番号`）の次行
    /// - 商品セクションは `購入した商品` ヘッダー
    /// - 数量は `&times;N` 形式（HTML エンティティ）
    /// - 注文番号は 5 桁ゼロパディング（例: `00122`）
    fn sample_send_html_format() -> &'static str {
        r#"プレミアムバンダイをご利用いただきましてまことにありがとうございます。
以下の通り佐川急便にてご注文商品を発送させていただきましたので、お知らせいたします。
ご注文番号：00122
ご注文日：2025年03月28日
購入した商品
figma テスト【再販】
&times;1
お問合せ伝票番号
360350813325
"#
    }

    fn sample_send_html_multiple_items() -> &'static str {
        r#"以下の通り佐川急便にてご注文商品を発送させていただきましたので、お知らせいたします。
ご注文番号：00123
ご注文日：2025年03月28日
購入した商品
商品A
&times;1
商品B【2025年4月発送】
&times;2
お問合せ伝票番号
360350813326
"#
    }

    // ─── 基本テスト（プレーンテキスト形式）───

    #[test]
    fn test_parse_send_order_number() {
        let order = PremiumBandaiSendParser.parse(sample_send_plain()).unwrap();
        assert_eq!(order.order_number, "12345");
    }

    #[test]
    fn test_parse_send_order_date_from_send_date() {
        let order = PremiumBandaiSendParser.parse(sample_send_plain()).unwrap();
        assert_eq!(order.order_date, Some("2025-01-20".to_string()));
    }

    #[test]
    fn test_parse_send_single_item() {
        let order = PremiumBandaiSendParser.parse(sample_send_plain()).unwrap();
        assert_eq!(order.items.len(), 1);
        assert_eq!(order.items[0].name, "figma テスト");
        assert_eq!(order.items[0].quantity, 1);
    }

    #[test]
    fn test_parse_send_tracking_number() {
        let order = PremiumBandaiSendParser.parse(sample_send_plain()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.tracking_number, "123456789012");
    }

    #[test]
    fn test_parse_send_carrier_sagawa() {
        let order = PremiumBandaiSendParser.parse(sample_send_plain()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.carrier, "佐川急便");
        assert!(delivery.carrier_url.unwrap().contains("sagawa-exp.co.jp"));
    }

    #[test]
    fn test_parse_send_no_amounts() {
        let order = PremiumBandaiSendParser.parse(sample_send_plain()).unwrap();
        assert!(order.subtotal.is_none());
        assert!(order.shipping_fee.is_none());
        assert!(order.total_amount.is_none());
    }

    // ─── 複数商品テスト（プレーンテキスト形式）───

    #[test]
    fn test_parse_send_multiple_items_count() {
        let order = PremiumBandaiSendParser
            .parse(sample_send_multiple_items())
            .unwrap();
        assert_eq!(order.items.len(), 2);
    }

    #[test]
    fn test_parse_send_multiple_items_names_normalized() {
        let order = PremiumBandaiSendParser
            .parse(sample_send_multiple_items())
            .unwrap();
        assert_eq!(order.items[0].name, "商品A");
        assert_eq!(order.items[1].name, "商品B");
    }

    #[test]
    fn test_parse_send_multiple_items_quantities() {
        let order = PremiumBandaiSendParser
            .parse(sample_send_multiple_items())
            .unwrap();
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[1].quantity, 2);
    }

    // ─── 配送業者別テスト ───

    #[test]
    fn test_parse_send_carrier_yamato() {
        let order = PremiumBandaiSendParser.parse(sample_send_yamato()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.carrier, "ヤマト運輸");
        assert!(delivery
            .carrier_url
            .unwrap()
            .contains("kuronekoyamato.co.jp"));
    }

    #[test]
    fn test_parse_send_carrier_yupack() {
        let order = PremiumBandaiSendParser.parse(sample_send_yupack()).unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.carrier, "ゆうパック");
        assert!(delivery.carrier_url.unwrap().contains("post.japanpost.jp"));
    }

    // ─── HTML メール形式テスト ───

    #[test]
    fn test_parse_send_html_format_order_number_with_leading_zero() {
        let order = PremiumBandaiSendParser
            .parse(sample_send_html_format())
            .unwrap();
        assert_eq!(order.order_number, "00122");
    }

    #[test]
    fn test_parse_send_html_format_order_date_from_order_date() {
        // HTML 形式は発送日フィールドなし → 注文日を使用
        let order = PremiumBandaiSendParser
            .parse(sample_send_html_format())
            .unwrap();
        assert_eq!(order.order_date, Some("2025-03-28".to_string()));
    }

    #[test]
    fn test_parse_send_html_format_single_item_name_normalized() {
        let order = PremiumBandaiSendParser
            .parse(sample_send_html_format())
            .unwrap();
        assert_eq!(order.items.len(), 1);
        assert_eq!(order.items[0].name, "figma テスト");
    }

    #[test]
    fn test_parse_send_html_format_quantity_from_times_entity() {
        // `&times;1` → 数量 1
        let order = PremiumBandaiSendParser
            .parse(sample_send_html_format())
            .unwrap();
        assert_eq!(order.items[0].quantity, 1);
    }

    #[test]
    fn test_parse_send_html_format_tracking_next_line() {
        // `お問合せ伝票番号` の次行から追跡番号を取得
        let order = PremiumBandaiSendParser
            .parse(sample_send_html_format())
            .unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.tracking_number, "360350813325");
    }

    #[test]
    fn test_parse_send_html_format_carrier_from_body_text() {
        // 本文テキスト `佐川急便にてご注文商品を発送` から配送業者を検出
        let order = PremiumBandaiSendParser
            .parse(sample_send_html_format())
            .unwrap();
        let delivery = order.delivery_info.unwrap();
        assert_eq!(delivery.carrier, "佐川急便");
        assert!(delivery.carrier_url.unwrap().contains("sagawa-exp.co.jp"));
    }

    #[test]
    fn test_parse_send_html_format_multiple_items() {
        let order = PremiumBandaiSendParser
            .parse(sample_send_html_multiple_items())
            .unwrap();
        assert_eq!(order.items.len(), 2);
        assert_eq!(order.items[0].name, "商品A");
        assert_eq!(order.items[0].quantity, 1);
        assert_eq!(order.items[1].name, "商品B");
        assert_eq!(order.items[1].quantity, 2);
    }

    // ─── エラーケース ───

    #[test]
    fn test_parse_send_no_order_number_returns_error() {
        let result = PremiumBandaiSendParser.parse(
            "■発送商品\n商品A\n数量：1個\n■配送情報\n配送業者：佐川急便\nお問い合わせ番号：123456789012",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_send_no_items_returns_error() {
        let result = PremiumBandaiSendParser.parse(
            "■ご注文番号：12345\n■発送日：2025年1月20日\n■配送情報\n配送業者：佐川急便\nお問い合わせ番号：123456789012",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_send_no_tracking_number_returns_error() {
        let result = PremiumBandaiSendParser
            .parse("■ご注文番号：12345\n■発送商品\n商品A\n数量：1個\n■配送情報\n配送業者：佐川急便");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_send_no_carrier_returns_error() {
        let result = PremiumBandaiSendParser.parse(
            "■ご注文番号：12345\n■発送商品\n商品A\n数量：1個\n■配送情報\nお問い合わせ番号：123456789012",
        );
        assert!(result.is_err());
    }

    // ─── HTML パーサ直接テスト ───

    #[test]
    fn test_extract_items_from_send_html_basic() {
        // 実際の HTML 形式: <td><strong>商品名</strong></td> + <td align="right">×1</td>
        let html = r#"<table width="580"><tr><td><strong>figma テスト【再販】</strong></td></tr><tr><td align="right">×1</td></tr></table>"#;
        let items = extract_items_from_send_html(html);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "figma テスト");
        assert_eq!(items[0].quantity, 1);
    }

    #[test]
    fn test_extract_items_from_send_html_multiple_items() {
        // 複数の商品テーブルが正しく抽出されること
        let html = r#"<table><tr><td><strong>商品A</strong></td></tr><tr><td align="right">×2</td></tr></table><table><tr><td><strong>商品B【再販】</strong></td></tr><tr><td align="right">×1</td></tr></table>"#;
        let items = extract_items_from_send_html(html);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "商品A");
        assert_eq!(items[0].quantity, 2);
        assert_eq!(items[1].name, "商品B");
        assert_eq!(items[1].quantity, 1);
    }

    #[test]
    fn test_extract_items_from_send_html_plain_text_returns_empty() {
        // プレーンテキストには <table> が存在しないため空リストを返す（テキストベースにフォールバック）
        let items = extract_items_from_send_html("figma テスト\n×1");
        assert!(items.is_empty(), "プレーンテキストは空リストのはず");
    }

    #[test]
    fn test_extract_items_from_send_html_tracking_number_table_skipped() {
        // 追跡番号テーブル（<td align="right">×N</td> なし）は商品として誤認識されないこと
        let html = r#"<table>
            <tr>
                <td>お問合せ番号</td>
                <td><strong>360344868240</strong></td>
            </tr>
        </table>"#;
        let items = extract_items_from_send_html(html);
        assert!(items.is_empty(), "追跡番号テーブルは商品として認識されないはず: {:?}", items);
    }
}
