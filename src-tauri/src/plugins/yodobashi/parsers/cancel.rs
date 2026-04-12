//! ヨドバシ・ドット・コム 注文キャンセルメール用パーサー
//!
//! 件名：`ヨドバシ・ドット・コム：ご注文内容変更のご連絡`
//! 送信元：`cancel@yodobashi.com`
//!
//! `【キャンセル対象のご注文商品】` セクションから注文番号・キャンセル商品リストを抽出する。
//! 1通のメールで複数商品がキャンセルされる場合があるため `Vec<CancelInfo>` を返す。

use once_cell::sync::Lazy;
use regex::Regex;

use crate::parsers::cancel_info::CancelInfo;

pub struct YodobashiCancelParser;

// ─── 正規表現 ────────────────────────────────────────────────────────────────

/// `【変更対象のご注文番号】 7538892732`
static ORDER_NUMBER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"【変更対象のご注文番号】\s*(\d+)").expect("ORDER_NUMBER_RE"));

/// キャンセルセクションの数量・価格行
/// `　　1 点　   880 円` → trimmed: `1 点　   880 円`
static CANCEL_QTY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(\d+)\s*点\s+([\d,]+)\s*円").expect("CANCEL_QTY_RE"));

// ─── ヘルパー ─────────────────────────────────────────────────────────────────

fn extract_order_number(body: &str) -> Option<String> {
    ORDER_NUMBER_RE
        .captures(body)
        .map(|c| c[1].trim().to_string())
}

/// `【キャンセル対象のご注文商品】` セクションからキャンセル商品リストを抽出する
///
/// 商品名の折り返し形式は confirm メールと同じ。
/// 数量行は `合計` プレフィックスなし（`N 点　X,XXX 円`）。
/// `・配達料金：` または `◎【変更前】` でセクション終了。
fn extract_cancel_items(body: &str) -> Vec<(String, i64)> {
    let mut items: Vec<(String, i64)> = Vec::new();
    let mut in_cancel_section = false;
    let mut current_name: Option<String> = None;
    let mut collecting_name = false;

    for line in body.lines() {
        let trimmed = line.trim();

        if trimmed == "【キャンセル対象のご注文商品】" {
            in_cancel_section = true;
            continue;
        }

        if !in_cancel_section {
            continue;
        }

        // セクション終了
        if trimmed.starts_with("・配達料金") || trimmed.starts_with("◎【変更前】") {
            break;
        }

        // 商品名の折り返し行を収集中
        if collecting_name {
            if let Some(ref mut name) = current_name {
                if let Some(rest) = trimmed.strip_suffix('」') {
                    name.push_str(rest);
                    collecting_name = false;
                } else {
                    name.push_str(trimmed);
                }
            }
            continue;
        }

        // 商品名行の開始
        if let Some(after) = trimmed.strip_prefix("・「") {
            if let Some(name) = after.strip_suffix('」') {
                current_name = Some(name.trim().to_string());
            } else {
                current_name = Some(after.to_string());
                collecting_name = true;
            }
            continue;
        }

        // 数量・価格行（`N 点　X,XXX 円`）
        if let Some(caps) = CANCEL_QTY_RE.captures(trimmed) {
            if let Some(name) = current_name.take() {
                let quantity: i64 = caps[1].parse().unwrap_or(1);
                items.push((name, quantity));
            }
            continue;
        }
    }

    items
}

// ─── パブリック API ───────────────────────────────────────────────────────────

impl YodobashiCancelParser {
    /// メール本文からキャンセル情報のリストを抽出する
    ///
    /// 複数商品がキャンセルされている場合は複数の `CancelInfo` を返す。
    /// キャンセル商品が見つからない場合はエラーを返す。
    pub fn parse_cancel(&self, email_body: &str) -> Result<Vec<CancelInfo>, String> {
        let order_number = extract_order_number(email_body)
            .ok_or_else(|| "注文番号が見つかりません".to_string())?;

        let items = extract_cancel_items(email_body);
        if items.is_empty() {
            return Err("キャンセル対象商品が見つかりません".to_string());
        }

        let cancel_infos = items
            .into_iter()
            .map(|(product_name, cancel_quantity)| CancelInfo {
                order_number: order_number.clone(),
                product_name,
                cancel_quantity,
            })
            .collect();

        Ok(cancel_infos)
    }
}

// ─── テスト ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cancel() -> &'static str {
        r#"【変更対象のご注文番号】 7538892732
─────────────────────────────
●変更ご依頼内容：
　　注文商品の一部キャンセル
─────────────────────────────

◎【変更後】のご注文内容
─────────────────────────────
●ご注文商品
---------------------------------------------------------------
・「データ用CD-R 700MB ひろびろワイドレーベル 10枚 エコパッケージ CD
　　R700S.SWPS.10E」
　　配達希望日：2026年02月24日
　　合計 1 点　   587 円
・配達料金：　　0 円

【キャンセル対象のご注文商品】
---------------------------------------------------------------
・「SDHCメモリーカード 16GB Class10 UHS-I U1 最大読込40MB/s RSDC-016
　　GU1S」
　　1 点　   880 円
・「SDHCカード 16GB Class10 UHS-I U1 最大読込70MB/s 最大書込70MB/s H
　　DSDH16GCL10UIJP3」
　　1 点　   880 円
・配達料金：　　0 円

◎【変更前】のご注文内容
─────────────────────────────
"#
    }

    #[test]
    fn test_parse_cancel_order_number() {
        let infos = YodobashiCancelParser.parse_cancel(sample_cancel()).unwrap();
        assert!(infos.iter().all(|i| i.order_number == "7538892732"));
    }

    #[test]
    fn test_parse_cancel_item_count() {
        let infos = YodobashiCancelParser.parse_cancel(sample_cancel()).unwrap();
        assert_eq!(infos.len(), 2);
    }

    #[test]
    fn test_parse_cancel_item_names() {
        let infos = YodobashiCancelParser.parse_cancel(sample_cancel()).unwrap();
        // 折り返し部分が連結されていること
        assert!(infos[0].product_name.contains("SDHCメモリーカード"));
        assert!(infos[0].product_name.contains("GU1S"));
        assert!(infos[1].product_name.contains("SDHCカード"));
        assert!(infos[1].product_name.contains("DSDH16GCL10UIJP3"));
    }

    #[test]
    fn test_parse_cancel_quantities() {
        let infos = YodobashiCancelParser.parse_cancel(sample_cancel()).unwrap();
        assert_eq!(infos[0].cancel_quantity, 1);
        assert_eq!(infos[1].cancel_quantity, 1);
    }

    #[test]
    fn test_parse_cancel_no_order_number_returns_error() {
        let result = YodobashiCancelParser.parse_cancel(
            "【キャンセル対象のご注文商品】\n・「テスト商品」\n　　1 点　   880 円\n・配達料金：　　0 円",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cancel_no_items_returns_error() {
        let result = YodobashiCancelParser
            .parse_cancel("【変更対象のご注文番号】 7538892732\n【キャンセル対象のご注文商品】\n・配達料金：　　0 円");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cancel_does_not_pick_up_non_cancel_section_items() {
        // 「◎【変更後】のご注文内容」の商品はキャンセル対象ではない
        let infos = YodobashiCancelParser.parse_cancel(sample_cancel()).unwrap();
        assert!(!infos
            .iter()
            .any(|i| i.product_name.contains("データ用CD-R")));
    }
}
