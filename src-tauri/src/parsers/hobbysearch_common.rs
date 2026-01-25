use super::{DeliveryAddress, DeliveryInfo};
use once_cell::sync::Lazy;
use regex::Regex;

/// 金額抽出用の正規表現（静的キャッシュ）
static AMOUNT_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([\d,]+)円").expect("Invalid regex pattern"));

/// 予約商品合計抽出用の正規表現（静的キャッシュ）
static YOYAKU_TOTAL_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"予約商品合計\s*([\d,]+)円").expect("Invalid regex pattern"));

/// 配送先情報を抽出
///
/// [商品お届け先] セクションから名前、郵便番号、住所を抽出する。
/// - 同じ行に名前がある場合（例: "[商品お届け先]  原田 裕基 様"）にも対応
/// - 郵便番号と住所が同じ行にある場合（例: "〒812-0044 福岡県..."）にも対応
pub fn extract_delivery_address(lines: &[&str]) -> Option<DeliveryAddress> {
    let mut in_delivery_section = false;
    let mut name: Option<String> = None;
    let mut postal_code: Option<String> = None;
    let mut address: Option<String> = None;

    for line in lines {
        let trimmed = line.trim();

        // [商品お届け先] セクション開始（同じ行に名前がある場合もある）
        if trimmed.starts_with("[商品お届け先]") {
            in_delivery_section = true;
            // 同じ行に名前がある場合（例: "[商品お届け先]  原田 裕基 様"）
            if trimmed.ends_with('様') {
                let name_part = trimmed
                    .trim_start_matches("[商品お届け先]")
                    .trim()
                    .trim_end_matches('様')
                    .trim();
                name = Some(name_part.to_string());
            }
            continue;
        }

        if in_delivery_section {
            // セクション終了判定
            if trimmed.is_empty() || trimmed.starts_with('[') {
                break;
            }

            // 郵便番号と住所を抽出（同じ行にある場合）
            if trimmed.starts_with('〒') {
                // 郵便番号だけを抽出（例: "〒812-0044 福岡県..." → "812-0044"）
                let rest = trimmed.trim_start_matches('〒').trim();
                if let Some(space_pos) = rest.find(' ') {
                    postal_code = Some(rest[..space_pos].trim().to_string());
                    address = Some(rest[space_pos..].trim().to_string());
                } else {
                    postal_code = Some(rest.to_string());
                }
            }
            // 住所だけの行（都道府県で始まる行）
            else if (trimmed.contains('県') || trimmed.contains('都') || trimmed.contains('府'))
                && address.is_none()
            {
                address = Some(trimmed.to_string());
            }
            // 名前を抽出（「様」で終わる行）
            else if trimmed.ends_with('様') && name.is_none() {
                name = Some(trimmed.trim_end_matches('様').trim().to_string());
            }
        }
    }

    name.map(|n| DeliveryAddress {
        name: n,
        postal_code,
        address,
    })
}

/// 商品行から商品名、メーカー、品番を抽出
///
/// 形式例: "マックスファクトリー 014554 PLAMAX BP-02 ソフィア・F・シャーリング 虎アーマーVer. (プラモデル) PLAMAX、BP-02"
/// - 最初の部分をメーカーとして扱う
/// - 2番目の部分が数字で始まる場合は品番
/// - (プラモデル) または (ディスプレイ) の直前までを商品名として抽出
pub fn parse_item_line(line: &str) -> (String, Option<String>, Option<String>) {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.is_empty() {
        return (line.to_string(), None, None);
    }

    // 最初の部分をメーカーとして扱う
    let manufacturer = Some(parts[0].to_string());

    // 2番目の部分が数字で始まる場合は品番
    let model_number = if parts.len() > 1 && parts[1].chars().next().is_some_and(|c| c.is_numeric())
    {
        Some(parts[1].to_string())
    } else {
        None
    };

    // (プラモデル) または (ディスプレイ) の直前までを商品名として抽出
    let name = if let Some(paren_pos) = line.find(" (プラモデル)") {
        line[..paren_pos].trim().to_string()
    } else if let Some(paren_pos) = line.find(" (ディスプレイ)") {
        line[..paren_pos].trim().to_string()
    } else {
        line.to_string()
    };

    (name, manufacturer, model_number)
}

/// 金額情報を抽出
///
/// 小計、送料、合計金額を抽出する。
/// 返り値: (subtotal, shipping_fee, total_amount)
pub fn extract_amounts(lines: &[&str]) -> (Option<i64>, Option<i64>, Option<i64>) {
    let mut subtotal: Option<i64> = None;
    let mut shipping_fee: Option<i64> = None;
    let mut total_amount: Option<i64> = None;

    for line in lines {
        let trimmed = line.trim();

        if trimmed.starts_with("小計") {
            subtotal = extract_amount_from_line(trimmed);
        } else if trimmed.starts_with("送料") {
            shipping_fee = extract_amount_from_line(trimmed);
        } else if trimmed.starts_with("合計") {
            total_amount = extract_amount_from_line(trimmed);
        }
    }

    (subtotal, shipping_fee, total_amount)
}

/// 行から金額を抽出
///
/// "小計　　　　　　　　    46,974円" のような形式から金額を抽出する。
pub fn extract_amount_from_line(line: &str) -> Option<i64> {
    AMOUNT_PATTERN.captures(line).and_then(|captures| {
        captures
            .get(1)
            .and_then(|m| m.as_str().replace(',', "").parse::<i64>().ok())
    })
}

/// 配送情報を抽出
///
/// 運送会社、配送伝票番号、配送日、配送時間、運送会社URLを抽出する。
pub fn extract_delivery_info(lines: &[&str]) -> Option<DeliveryInfo> {
    let mut carrier: Option<String> = None;
    let mut tracking_number: Option<String> = None;
    let mut delivery_date: Option<String> = None;
    let mut delivery_time: Option<String> = None;
    let mut carrier_url: Option<String> = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with("[運送会社]") {
            carrier = Some(trimmed.trim_start_matches("[運送会社]").trim().to_string());
        } else if trimmed.starts_with("[配送日]") {
            delivery_date = Some(trimmed.trim_start_matches("[配送日]").trim().to_string());
        } else if trimmed.starts_with("[配送時間]") {
            delivery_time = Some(trimmed.trim_start_matches("[配送時間]").trim().to_string());
        } else if trimmed.starts_with("[配送伝票]") {
            tracking_number = Some(trimmed.trim_start_matches("[配送伝票]").trim().to_string());
        } else if trimmed == "[運送会社URL]" && i + 1 < lines.len() {
            // 次の行にURLがある
            let next_line = lines[i + 1].trim();
            if next_line.starts_with("http") {
                carrier_url = Some(next_line.to_string());
            }
        }
    }

    if let (Some(carrier), Some(tracking_number)) = (carrier, tracking_number) {
        Some(DeliveryInfo {
            carrier,
            tracking_number,
            delivery_date,
            delivery_time,
            carrier_url,
        })
    } else {
        None
    }
}

/// 予約商品合計を抽出
///
/// "予約商品合計　8,096円" のような形式から金額を抽出する。
pub fn extract_yoyaku_total(lines: &[&str]) -> Option<i64> {
    for line in lines {
        if let Some(captures) = YOYAKU_TOTAL_PATTERN.captures(line) {
            return captures
                .get(1)
                .and_then(|m| m.as_str().replace(',', "").parse::<i64>().ok());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_delivery_address_basic() {
        let lines = vec![
            "[商品お届け先]",
            "山田 太郎 様",
            "〒100-0001 東京都千代田区1-1-1",
        ];
        let result = extract_delivery_address(&lines);
        assert!(result.is_some());
        let addr = result.unwrap();
        assert_eq!(addr.name, "山田 太郎");
        assert_eq!(addr.postal_code, Some("100-0001".to_string()));
        assert_eq!(addr.address, Some("東京都千代田区1-1-1".to_string()));
    }

    #[test]
    fn test_extract_delivery_address_name_on_same_line() {
        let lines = vec![
            "[商品お届け先]  山田 太郎 様",
            "〒100-0001 東京都千代田区1-1-1",
        ];
        let result = extract_delivery_address(&lines);
        assert!(result.is_some());
        let addr = result.unwrap();
        assert_eq!(addr.name, "山田 太郎");
    }

    #[test]
    fn test_extract_delivery_address_no_section() {
        let lines = vec!["山田 太郎 様", "〒100-0001 東京都千代田区1-1-1"];
        let result = extract_delivery_address(&lines);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_item_line_with_model_number() {
        let line = "メーカー 12345 商品名 (プラモデル)";
        let (name, manufacturer, model_number) = parse_item_line(line);
        assert_eq!(name, "メーカー 12345 商品名");
        assert_eq!(manufacturer, Some("メーカー".to_string()));
        assert_eq!(model_number, Some("12345".to_string()));
    }

    #[test]
    fn test_parse_item_line_without_model_number() {
        let line = "メーカー ABC 商品名 (ディスプレイ)";
        let (name, manufacturer, model_number) = parse_item_line(line);
        assert_eq!(name, "メーカー ABC 商品名");
        assert_eq!(manufacturer, Some("メーカー".to_string()));
        assert_eq!(model_number, None);
    }

    #[test]
    fn test_parse_item_line_no_category() {
        let line = "メーカー 12345 商品名";
        let (name, manufacturer, model_number) = parse_item_line(line);
        assert_eq!(name, "メーカー 12345 商品名");
        assert_eq!(manufacturer, Some("メーカー".to_string()));
        assert_eq!(model_number, Some("12345".to_string()));
    }

    #[test]
    fn test_extract_amounts_all_present() {
        let lines = vec![
            "小計　　　　　　　　1,000円",
            "送料　　　　　　　　500円",
            "合計　　　　　　　　1,500円",
        ];
        let (subtotal, shipping, total) = extract_amounts(&lines);
        assert_eq!(subtotal, Some(1000));
        assert_eq!(shipping, Some(500));
        assert_eq!(total, Some(1500));
    }

    #[test]
    fn test_extract_amounts_partial() {
        let lines = vec!["小計　　　　　　　　2,500円", "合計　　　　　　　　2,500円"];
        let (subtotal, shipping, total) = extract_amounts(&lines);
        assert_eq!(subtotal, Some(2500));
        assert_eq!(shipping, None);
        assert_eq!(total, Some(2500));
    }

    #[test]
    fn test_extract_amount_from_line_with_comma() {
        assert_eq!(extract_amount_from_line("小計 46,974円"), Some(46974));
    }

    #[test]
    fn test_extract_amount_from_line_without_comma() {
        assert_eq!(extract_amount_from_line("送料 500円"), Some(500));
    }

    #[test]
    fn test_extract_amount_from_line_no_amount() {
        assert_eq!(extract_amount_from_line("送料無料"), None);
    }

    #[test]
    fn test_extract_delivery_info_complete() {
        let lines = vec![
            "[運送会社] ヤマト運輸",
            "[配送伝票] 1234567890",
            "[配送日] 2024/01/15",
            "[配送時間] 14:00-16:00",
            "[運送会社URL]",
            "https://example.com/track",
        ];
        let result = extract_delivery_info(&lines);
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.carrier, "ヤマト運輸");
        assert_eq!(info.tracking_number, "1234567890");
        assert_eq!(info.delivery_date, Some("2024/01/15".to_string()));
        assert_eq!(info.delivery_time, Some("14:00-16:00".to_string()));
        assert_eq!(info.carrier_url, Some("https://example.com/track".to_string()));
    }

    #[test]
    fn test_extract_delivery_info_minimal() {
        let lines = vec!["[運送会社] 佐川急便", "[配送伝票] 9876543210"];
        let result = extract_delivery_info(&lines);
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.carrier, "佐川急便");
        assert_eq!(info.tracking_number, "9876543210");
        assert_eq!(info.delivery_date, None);
        assert_eq!(info.delivery_time, None);
        assert_eq!(info.carrier_url, None);
    }

    #[test]
    fn test_extract_delivery_info_missing_required() {
        let lines = vec!["[運送会社] ヤマト運輸"];
        let result = extract_delivery_info(&lines);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_yoyaku_total_found() {
        let lines = vec!["その他の情報", "予約商品合計　8,096円", "備考"];
        assert_eq!(extract_yoyaku_total(&lines), Some(8096));
    }

    #[test]
    fn test_extract_yoyaku_total_with_spaces() {
        let lines = vec!["予約商品合計  12,345円"];
        assert_eq!(extract_yoyaku_total(&lines), Some(12345));
    }

    #[test]
    fn test_extract_yoyaku_total_not_found() {
        let lines = vec!["商品合計　8,096円", "送料　500円"];
        assert_eq!(extract_yoyaku_total(&lines), None);
    }
}
