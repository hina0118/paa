//! プレミアムバンダイ 共通ヘルパー

use once_cell::sync::Lazy;
use regex::Regex;

pub mod confirm;
pub mod omatome;
pub mod send;

/// `<br>` / `<br/>` / `<br />` タグを改行に置換するパターン
static BR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)<br\s*/?>").expect("Invalid BR_RE"));

/// HTML タグ全体を除去するパターン
static HTML_TAG_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<[^>]+>").expect("Invalid HTML_TAG_RE"));

/// `img` タグの `src` 属性を抽出するパターン
static IMG_SRC_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)<img[^>]+src=["']([^"']+)["'][^>]*>"#).expect("Invalid IMG_SRC_RE")
});

/// HTML メール検出パターン
///
/// `<br>` タグだけでなく `<table>` / `<td>` / `<th>` などの HTML 構造タグも検出する。
/// `<br>` なしのテーブル形式 HTML メールに対応するため。
static HTML_DETECT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)<(?:br|html|head|body|table|tr|td|th|div|p|span)\b")
        .expect("Invalid HTML_DETECT_RE")
});

/// 注文番号（数字列）: `ご注文番号：12345` 形式
///
/// マッチ後、`extract_order_number` 側で長さが 5 桁かどうかを検証する。
/// `regex` クレートは lookahead 非対応のため、コード側で長さチェックを行う。
static ORDER_NUMBER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"ご?注文番号[：:]\s*(\d+)").expect("Invalid ORDER_NUMBER_RE"));

/// 注文番号: `【ご?注文No.】　00130` 形式（同一行、おまとめメール・HTML メール）
static ORDER_NUMBER_NO_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"【ご?注文No\.】\s*(\d+)").expect("Invalid ORDER_NUMBER_NO_RE"));

/// 注文日（YYYY年M月D日）
static ORDER_DATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"ご?注文日[：:]\s*(\d{4})年(\d{1,2})月(\d{1,2})日").expect("Invalid ORDER_DATE_RE")
});

/// 注文日（YYYY-MM-DD 形式）: `【ご?注文日】　2025-05-14 12:04:59` 形式（同一行、おまとめメール・HTML メール）
static ORDER_DATE_ISO_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"【ご?注文日】\s*(\d{4})-(\d{2})-(\d{2})").expect("Invalid ORDER_DATE_ISO_RE")
});

/// 注文番号ラベルのみの行（次行に値が来る HTML テーブル形式）: `【注文No.】`
static ORDER_NUMBER_NO_LABEL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^【ご?注文No\.】\s*$").expect("Invalid ORDER_NUMBER_NO_LABEL_RE"));

/// 注文日ラベルのみの行（次行に値が来る HTML テーブル形式）: `【注文日】`
static ORDER_DATE_ISO_LABEL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^【ご?注文日】\s*$").expect("Invalid ORDER_DATE_ISO_LABEL_RE"));

/// ISO 形式日付値行: `2025-05-14` または `2025-05-14 12:02:14`（次行パターン用）
static ISO_DATE_VALUE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(\d{4})-(\d{2})-(\d{2})").expect("Invalid ISO_DATE_VALUE_RE"));

/// 送料ラベルのみの行（次行に値が来る HTML テーブル形式）: `送料：`
static SHIPPING_LABEL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^送料[：:]\s*$").expect("Invalid SHIPPING_LABEL_RE"));

/// 支払手数料ラベルのみの行（次行に値が来る HTML テーブル形式）
static PAYMENT_FEE_LABEL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:支払|代引|決済)[手数]*料[：:]\s*$").expect("Invalid PAYMENT_FEE_LABEL_RE")
});

/// `N円` 形式の金額値行（次行パターン用）: `660円`
static YEN_AMOUNT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([\d,]+)円$").expect("Invalid YEN_AMOUNT_RE"));

/// 単価行：`単価：￥5,000（税込）` または `￥5,000（税込）`
static UNIT_PRICE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:単価[：:]\s*)?[¥￥]([\d,]+)(?:（税込）)?").expect("Invalid UNIT_PRICE_RE")
});

/// 個数行：`個数：1個` または `数量：1個`
static QUANTITY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:個数|数量)[：:]\s*(\d+)個?").expect("Invalid QUANTITY_RE"));

/// 商品ごとの小計行：`小計：￥5,000`
static ITEM_SUBTOTAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^小計[：:]\s*[¥￥]([\d,]+)").expect("Invalid ITEM_SUBTOTAL_RE"));

/// 送料行
///
/// 対応形式:
/// - `送料：￥660`（`¥` 前置き）
/// - `送料：　660円`（`円` 後置き、おまとめメール形式）
static SHIPPING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^送料[：:]\s*[¥￥]?([\d,]+)").expect("Invalid SHIPPING_RE"));

/// 支払手数料行（代引手数料・決済手数料も含む）
///
/// 対応形式:
/// - `支払手数料：￥330`
/// - `決済手数料：　0円`（おまとめメール形式）
static PAYMENT_FEE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:支払|代引|決済)[手数]*料[：:]\s*[¥￥]?([\d,]+)")
        .expect("Invalid PAYMENT_FEE_RE")
});

/// 合計行
///
/// 対応形式:
/// - `合計：￥11,000`
/// - `お支払い合計金額：　6,930円`（おまとめメール形式）
/// - `支払合計金額：　4,290円`（HTML メール形式）
static TOTAL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:合計|お支払い?合計金額|支払合計金額)[：:]\s*[¥￥]?([\d,]+)")
        .expect("Invalid TOTAL_RE")
});

/// 発送日（YYYY年M月D日）
static SEND_DATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"発送日[：:]\s*(\d{4})年(\d{1,2})月(\d{1,2})日").expect("Invalid SEND_DATE_RE")
});

/// 追跡番号：同一行パターン `お問い合わせ番号：123456789012` / `追跡番号：...`
///
/// 実際のプレミアムバンダイメールでは `お問合せ伝票番号`（`い` なし）のラベルの後、
/// 次行に追跡番号が来るパターンも存在する。そちらは `extract_tracking_number` で対応する。
static TRACKING_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:お問合せ伝票番号|お問い合わせ番号|追跡番号)[：:]\s*(\d+)")
        .expect("Invalid TRACKING_RE")
});

/// 配送業者：構造化行パターン `配送業者：佐川急便`
static CARRIER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:配送業者|配送会社)[：:]\s*(.+)").expect("Invalid CARRIER_RE"));

/// 商品名正規化: 末尾・先頭の【】ブロックを除去
///
/// 対象パターン:
/// - `【再販】`
/// - `【再生産】`
/// - `【N次！YYYY年M月発送】`（例: `【2次！2025年4月発送】`、半角）
/// - `【N次：YYYY年M月発送】`（例: `【３次：２０２５年８月発送】`、全角数字）
/// - `【YYYY年M月発送】`（例: `【2025年4月発送】` / `【２０２５年８月発送】`）
///
/// 全角数字（`０`-`９`）にも対応する。
static NORMALIZE_SUFFIX_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"【(?:再販|再生産|[\d０-９]+次[！：][\d０-９]{4}年[\d０-９]{1,2}月発送|[\d０-９]{4}年[\d０-９]{1,2}月発送)】",
    )
    .expect("Invalid NORMALIZE_SUFFIX_RE")
});

/// 商品名を正規化する
///
/// `【再販】`、`【再生産】`、`【YYYY年M月発送】` などの接尾辞を除去してトリムする。
pub fn normalize_product_name(name: &str) -> String {
    NORMALIZE_SUFFIX_RE.replace_all(name, "").trim().to_string()
}

/// HTML ボディをテキスト行のリストに変換する
fn html_to_lines(html: &str) -> Vec<String> {
    let with_newlines = BR_RE.replace_all(html, "\n");
    let without_tags = HTML_TAG_RE.replace_all(&with_newlines, "");
    without_tags.lines().map(|l| l.trim().to_string()).collect()
}

/// メール本文をテキスト行のリストに変換する
///
/// HTML タグ（`<br>`, `<table>`, `<td>`, `<th>` 等）を含む場合は `html_to_lines()` を使用し、
/// プレーンテキストの場合はそのまま分割する。
///
/// `<br>` タグを含まないテーブル形式 HTML メール（`<td>`/`<th>` 構造のみ）にも対応する。
pub fn body_to_lines(body: &str) -> Vec<String> {
    if HTML_DETECT_RE.is_match(body) {
        html_to_lines(body)
    } else {
        body.lines().map(|l| l.trim().to_string()).collect()
    }
}

/// 注文番号（5桁数字）を抽出する
///
/// 以下の3パターンを試みる:
/// 1. `ご注文番号：12345`
/// 2. `【ご?注文No.】　00130`（同一行、おまとめメール形式）
/// 3. `【注文No.】` ラベルのみ → 次の非空行が 5 桁数字（HTML テーブル形式）
///
/// 抽出した数字列が 5 桁でない場合は `None` を返す。
pub fn extract_order_number(lines: &[&str]) -> Option<String> {
    let try_5digit = |num: &str| -> Option<String> {
        if num.len() == 5 {
            Some(num.to_string())
        } else {
            None
        }
    };

    for (i, line) in lines.iter().enumerate() {
        // Pattern 1: ご注文番号：12345
        if let Some(caps) = ORDER_NUMBER_RE.captures(line) {
            if let Some(v) = try_5digit(&caps[1]) {
                return Some(v);
            }
        }
        // Pattern 2: 【ご?注文No.】　00130（同一行）
        if let Some(caps) = ORDER_NUMBER_NO_RE.captures(line) {
            if let Some(v) = try_5digit(&caps[1]) {
                return Some(v);
            }
        }
        // Pattern 3: 【注文No.】 ラベルのみ → 次の非空行に 5 桁数字（HTML テーブル形式）
        if ORDER_NUMBER_NO_LABEL_RE.is_match(line) {
            if let Some(v) = lines.get(i + 1..).unwrap_or(&[])
                .iter()
                .map(|l| l.trim())
                .find(|t| !t.is_empty())
                .and_then(&try_5digit)
            {
                return Some(v);
            }
        }
    }
    None
}

/// 日本語形式の日付 `YYYY年M月D日` を `YYYY-MM-DD` に変換する
fn parse_jp_date(year: &str, month: &str, day: &str) -> Option<String> {
    let y: u32 = year.parse().ok()?;
    let m: u32 = month.parse().ok()?;
    let d: u32 = day.parse().ok()?;
    Some(format!("{:04}-{:02}-{:02}", y, m, d))
}

/// 注文日を `YYYY-MM-DD` 形式で抽出する
///
/// 以下の3パターンを試みる:
/// 1. `ご注文日：2025年1月15日`（日本語形式）
/// 2. `【ご?注文日】　2025-05-14 12:04:59`（同一行、ISO 形式、おまとめメール）
/// 3. `【注文日】` ラベルのみ → 次の非空行が ISO 日付（HTML テーブル形式）
pub fn extract_order_date(lines: &[&str]) -> Option<String> {
    for (i, line) in lines.iter().enumerate() {
        // Pattern 1: 日本語形式
        if let Some(caps) = ORDER_DATE_RE.captures(line) {
            if let Some(v) = parse_jp_date(&caps[1], &caps[2], &caps[3]) {
                return Some(v);
            }
        }
        // Pattern 2: ISO 形式（同一行、おまとめメール）
        if let Some(caps) = ORDER_DATE_ISO_RE.captures(line) {
            return Some(format!("{}-{}-{}", &caps[1], &caps[2], &caps[3]));
        }
        // Pattern 3: 【注文日】 ラベルのみ → 次の非空行に ISO 日付（HTML テーブル形式）
        if ORDER_DATE_ISO_LABEL_RE.is_match(line) {
            if let Some(date_line) = lines.get(i + 1..).unwrap_or(&[])
                .iter()
                .map(|l| l.trim())
                .find(|t| !t.is_empty())
            {
                if let Some(caps) = ISO_DATE_VALUE_RE.captures(date_line) {
                    return Some(format!("{}-{}-{}", &caps[1], &caps[2], &caps[3]));
                }
            }
        }
    }
    None
}

/// 発送日（`YYYY年M月D日`）を `YYYY-MM-DD` 形式で抽出する
pub fn extract_send_date(lines: &[&str]) -> Option<String> {
    lines.iter().find_map(|line| {
        SEND_DATE_RE
            .captures(line)
            .and_then(|c| parse_jp_date(&c[1], &c[2], &c[3]))
    })
}

/// `N円` または `¥N` 形式の行から金額を解析する（次行パターン用）
fn parse_amount_line(line: &str) -> Option<i64> {
    // `N円` 形式（例: `660円`）
    if let Some(caps) = YEN_AMOUNT_RE.captures(line) {
        return caps[1].replace(',', "").parse().ok();
    }
    // `¥N` または `￥N` 形式
    UNIT_PRICE_RE
        .captures(line)
        .and_then(|c| c[1].replace(',', "").parse().ok())
}

/// 送料を抽出する
///
/// 以下の2パターンに対応する:
/// 1. 同一行: `送料：￥660` / `送料：　660円`
/// 2. 次行: `送料：` ラベルのみ → 次の非空行に金額（HTML テーブル形式）
pub fn extract_shipping_fee(lines: &[&str]) -> Option<i64> {
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        // 同一行マッチ
        if let Some(v) = SHIPPING_RE
            .captures(trimmed)
            .and_then(|c| c[1].replace(',', "").parse().ok())
        {
            return Some(v);
        }
        // 次行マッチ: `送料：` ラベルのみ
        if SHIPPING_LABEL_RE.is_match(trimmed) {
            if let Some(next) = lines.get(i + 1..).unwrap_or(&[])
                .iter()
                .map(|l| l.trim())
                .find(|t| !t.is_empty())
            {
                if let Some(v) = parse_amount_line(next) {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// 支払手数料を抽出する
///
/// 以下の2パターンに対応する:
/// 1. 同一行: `支払手数料：￥330` / `決済手数料：　0円`
/// 2. 次行: `決済手数料：` ラベルのみ → 次の非空行に金額（HTML テーブル形式）
pub fn extract_payment_fee(lines: &[&str]) -> Option<i64> {
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        // 同一行マッチ
        if let Some(v) = PAYMENT_FEE_RE
            .captures(trimmed)
            .and_then(|c| c[1].replace(',', "").parse().ok())
        {
            return Some(v);
        }
        // 次行マッチ: `決済手数料：` ラベルのみ
        if PAYMENT_FEE_LABEL_RE.is_match(trimmed) {
            if let Some(next) = lines.get(i + 1..).unwrap_or(&[])
                .iter()
                .map(|l| l.trim())
                .find(|t| !t.is_empty())
            {
                if let Some(v) = parse_amount_line(next) {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// 合計金額を抽出する
pub fn extract_total_amount(lines: &[&str]) -> Option<i64> {
    lines.iter().find_map(|line| {
        TOTAL_RE
            .captures(line.trim())
            .and_then(|c| c[1].replace(',', "").parse().ok())
    })
}

/// 追跡番号を抽出する
///
/// 以下の2パターンに対応する:
/// 1. 同一行: `お問い合わせ番号：123456789012`
/// 2. 次行: `お問合せ伝票番号` の次の非空行が数字のみ（実際のプレミアムバンダイ HTML メール）
pub fn extract_tracking_number(lines: &[&str]) -> Option<String> {
    // Pattern 1: 同一行
    for line in lines.iter() {
        if let Some(caps) = TRACKING_RE.captures(line) {
            return Some(caps[1].to_string());
        }
    }

    // Pattern 2: ラベルの次行に数字のみ
    let mut after_label = false;
    for line in lines.iter() {
        let trimmed = line.trim();
        if trimmed.contains("お問合せ伝票番号")
            || trimmed.contains("お問い合わせ番号")
            || trimmed.contains("追跡番号")
        {
            after_label = true;
            continue;
        }
        if after_label && !trimmed.is_empty() {
            if trimmed.chars().all(|c| c.is_ascii_digit()) {
                return Some(trimmed.to_string());
            }
            // 数字以外の行が来たらリセット
            after_label = false;
        }
    }
    None
}

/// 配送業者名を抽出する
///
/// 以下の2パターンに対応する:
/// 1. 構造化行: `配送業者：佐川急便`
/// 2. 本文テキスト: `佐川急便にて` / `ヤマト運輸にて` 等（実際のプレミアムバンダイ HTML メール）
pub fn extract_carrier(lines: &[&str]) -> Option<String> {
    // Pattern 1: 構造化行
    if let Some(carrier) = lines
        .iter()
        .find_map(|line| CARRIER_RE.captures(line).map(|c| c[1].trim().to_string()))
    {
        return Some(carrier);
    }

    // Pattern 2: 本文テキストに含まれる配送業者名
    for line in lines.iter() {
        let t = line.trim();
        if t.contains("佐川急便") {
            return Some("佐川急便".to_string());
        }
        if t.contains("ヤマト運輸") || t.contains("クロネコヤマト") {
            return Some("ヤマト運輸".to_string());
        }
        if t.contains("ゆうパック") || t.contains("日本郵便") {
            return Some("ゆうパック".to_string());
        }
    }
    None
}

/// 配送業者に対応する追跡 URL を生成する
pub fn carrier_tracking_url(carrier: &str, tracking_number: &str) -> Option<String> {
    if carrier.contains("佐川") {
        Some(format!(
            "https://k2k.sagawa-exp.co.jp/p/web/okurijosearch.do?okurijoNo={}",
            tracking_number
        ))
    } else if carrier.contains("ヤマト") {
        Some(format!(
            "https://jizen.kuronekoyamato.co.jp/jizen/servlet/com.nec_fielding.jizen.web.JizenServlet?id={}",
            tracking_number
        ))
    } else if carrier.contains("ゆうパック")
        || carrier.contains("日本郵便")
        || carrier.contains("郵便")
    {
        Some(format!(
            "https://trackings.post.japanpost.jp/services/srv/search/direct?reqCodeNo1={}",
            tracking_number
        ))
    } else {
        None
    }
}

/// HTML 本文から `<img>` の `src` を抽出する
///
/// 「おすすめ商品」セクション以降は除外する。
/// `.gif` 拡張子（スペーサー画像・トラッキングピクセル等）は除外する。
/// JPEG / PNG / WebP 以外の形式は `image_utils` で保存に失敗するため、
/// 事前にフィルターしてノイズログを抑制する。
pub fn extract_image_urls_from_html(html: &str) -> Vec<String> {
    // おすすめ商品セクション以前のみ対象とする
    let target = if let Some(pos) = html.find("おすすめ商品") {
        &html[..pos]
    } else {
        html
    };

    IMG_SRC_RE
        .captures_iter(target)
        .map(|c| c[1].to_string())
        // GIF はスペーサー・ロゴ・トラッキングピクセル等であることが多く、
        // 商品画像として保存できないため除外する
        .filter(|url| !url.to_lowercase().ends_with(".gif"))
        .collect()
}

/// 行インデックスが「おすすめ商品」セクション開始より前かを判定するためのインデックスを返す
pub fn find_recommend_section_line(lines: &[&str]) -> Option<usize> {
    lines.iter().position(|l| {
        l.contains("おすすめ商品")
            || l.contains("関連商品")
            || l.contains("RECOMMEND")
            || l.contains("あわせて買いたい")
    })
}

/// `¥5,000` / `￥5,000（税込）` 形式から金額を解析する
pub fn parse_price(s: &str) -> Option<i64> {
    UNIT_PRICE_RE
        .captures(s)
        .and_then(|c| c[1].replace(',', "").parse().ok())
}

/// 行から個数を解析する
pub fn parse_quantity(line: &str) -> Option<i64> {
    QUANTITY_RE.captures(line).and_then(|c| c[1].parse().ok())
}

/// 行から商品ごとの小計を解析する
pub fn parse_item_subtotal(line: &str) -> Option<i64> {
    ITEM_SUBTOTAL_RE
        .captures(line.trim())
        .and_then(|c| c[1].replace(',', "").parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_product_name_strip_resale() {
        assert_eq!(
            normalize_product_name("figma テスト【再販】"),
            "figma テスト"
        );
    }

    #[test]
    fn test_normalize_product_name_strip_reseisan() {
        assert_eq!(
            normalize_product_name("figma テスト【再生産】"),
            "figma テスト"
        );
    }

    #[test]
    fn test_normalize_product_name_strip_month_dispatch() {
        assert_eq!(
            normalize_product_name("figma テスト【2025年4月発送】"),
            "figma テスト"
        );
    }

    #[test]
    fn test_normalize_product_name_strip_nth_dispatch() {
        assert_eq!(
            normalize_product_name("figma テスト【2次！2025年4月発送】"),
            "figma テスト"
        );
    }

    /// 全角数字 + `：`（実際のおまとめメール形式）
    #[test]
    fn test_normalize_product_name_strip_fullwidth_nth_dispatch() {
        assert_eq!(
            normalize_product_name("ＨＧ 1/144 テスト【３次：２０２５年８月発送】"),
            "ＨＧ 1/144 テスト"
        );
    }

    /// 全角数字の年月形式
    #[test]
    fn test_normalize_product_name_strip_fullwidth_month_dispatch() {
        assert_eq!(
            normalize_product_name("ＨＧ テスト【２０２５年８月発送】"),
            "ＨＧ テスト"
        );
    }

    #[test]
    fn test_normalize_product_name_no_suffix() {
        assert_eq!(normalize_product_name("figma テスト"), "figma テスト");
    }

    #[test]
    fn test_extract_order_number_5digit() {
        let lines = vec!["ご注文番号：12345"];
        assert_eq!(extract_order_number(&lines), Some("12345".to_string()));
    }

    #[test]
    fn test_extract_order_number_without_go() {
        let lines = vec!["注文番号：12345"];
        assert_eq!(extract_order_number(&lines), Some("12345".to_string()));
    }

    #[test]
    fn test_extract_order_number_not_5digit_returns_none() {
        let lines = vec!["注文番号：1234567"];
        assert_eq!(extract_order_number(&lines), None);
    }

    /// 実際のおまとめメール形式: `【ご注文No.】　 00130`
    #[test]
    fn test_extract_order_number_no_format() {
        let lines = vec!["【ご注文No.】\u{3000} 00130"]; // 全角スペース + 半角スペース
        assert_eq!(extract_order_number(&lines), Some("00130".to_string()));
    }

    /// HTML テーブル形式: `【注文No.】`（`ご` なし）ラベルのみ → 次行に `00129`
    #[test]
    fn test_extract_order_number_no_go_next_line() {
        let lines = vec!["【注文No.】", "", "00129"];
        assert_eq!(extract_order_number(&lines), Some("00129".to_string()));
    }

    /// HTML テーブル形式: `【注文No.】`（`ご` なし）でも `ご` ありと同じ結果
    #[test]
    fn test_extract_order_number_no_go_same_line() {
        let lines = vec!["【注文No.】\u{3000}00129"];
        assert_eq!(extract_order_number(&lines), Some("00129".to_string()));
    }

    #[test]
    fn test_extract_order_date_jp() {
        let lines = vec!["ご注文日：2025年1月15日"];
        assert_eq!(extract_order_date(&lines), Some("2025-01-15".to_string()));
    }

    #[test]
    fn test_extract_order_date_single_digit_month_day() {
        let lines = vec!["注文日：2025年1月5日"];
        assert_eq!(extract_order_date(&lines), Some("2025-01-05".to_string()));
    }

    /// 実際のおまとめメール形式: `【ご注文日】　　2025-05-14 12:04:59`
    #[test]
    fn test_extract_order_date_iso_format() {
        let lines = vec!["【ご注文日】\u{3000}\u{3000}2025-05-14 12:04:59"];
        assert_eq!(extract_order_date(&lines), Some("2025-05-14".to_string()));
    }

    /// HTML テーブル形式: `【注文日】`（`ご` なし）ラベルのみ → 次行に `2025-05-14 12:02:14`
    #[test]
    fn test_extract_order_date_no_go_next_line() {
        let lines = vec!["【注文日】", "", "2025-05-14 12:02:14"];
        assert_eq!(extract_order_date(&lines), Some("2025-05-14".to_string()));
    }

    #[test]
    fn test_extract_shipping_fee_zero() {
        let lines = vec!["送料：￥0"];
        assert_eq!(extract_shipping_fee(&lines), Some(0));
    }

    #[test]
    fn test_extract_shipping_fee_nonzero() {
        let lines = vec!["送料：￥660"];
        assert_eq!(extract_shipping_fee(&lines), Some(660));
    }

    /// おまとめメール形式: `送料：　660円`（`¥` なし、`円` 後置き）
    #[test]
    fn test_extract_shipping_fee_yen_suffix() {
        let lines = vec!["送料：\u{3000}660円"];
        assert_eq!(extract_shipping_fee(&lines), Some(660));
    }

    #[test]
    fn test_extract_payment_fee() {
        let lines = vec!["支払手数料：￥330"];
        assert_eq!(extract_payment_fee(&lines), Some(330));
    }

    /// おまとめメール形式: `決済手数料：　0円`
    #[test]
    fn test_extract_payment_fee_kessai() {
        let lines = vec!["決済手数料：\u{3000}0円"];
        assert_eq!(extract_payment_fee(&lines), Some(0));
    }

    /// HTML テーブル形式: `送料：` ラベルのみ → 次行に `660円`
    #[test]
    fn test_extract_shipping_fee_next_line() {
        let lines = vec!["送料：", "660円"];
        assert_eq!(extract_shipping_fee(&lines), Some(660));
    }

    /// HTML テーブル形式: `決済手数料：` ラベルのみ → 次行に `0円`
    #[test]
    fn test_extract_payment_fee_next_line() {
        let lines = vec!["決済手数料：", "0円"];
        assert_eq!(extract_payment_fee(&lines), Some(0));
    }

    #[test]
    fn test_extract_total_amount() {
        let lines = vec!["合計：￥5,330"];
        assert_eq!(extract_total_amount(&lines), Some(5330));
    }

    /// おまとめメール形式: `お支払い合計金額：　6,930円`
    #[test]
    fn test_extract_total_amount_oshiharai() {
        let lines = vec!["お支払い合計金額：\u{3000}6,930円"];
        assert_eq!(extract_total_amount(&lines), Some(6930));
    }

    /// HTML メール形式: `支払合計金額：　4,290円`（`お` なし）
    #[test]
    fn test_extract_total_amount_shiharai_gokei() {
        let lines = vec!["支払合計金額：\u{3000}4,290円"];
        assert_eq!(extract_total_amount(&lines), Some(4290));
    }

    #[test]
    fn test_extract_tracking_number_same_line() {
        let lines = vec!["お問い合わせ番号：123456789012"];
        assert_eq!(
            extract_tracking_number(&lines),
            Some("123456789012".to_string())
        );
    }

    /// 実際のプレミアムバンダイ HTML メール: `お問合せ伝票番号` の次行に数字
    #[test]
    fn test_extract_tracking_number_next_line() {
        let lines = vec!["お問合せ伝票番号", "360350813325"];
        assert_eq!(
            extract_tracking_number(&lines),
            Some("360350813325".to_string())
        );
    }

    #[test]
    fn test_extract_carrier_structured() {
        let lines = vec!["配送業者：佐川急便"];
        assert_eq!(extract_carrier(&lines), Some("佐川急便".to_string()));
    }

    /// 実際のプレミアムバンダイ HTML メール: 本文テキストから配送業者を検出
    #[test]
    fn test_extract_carrier_from_body_text() {
        let lines = vec![
            "「プレミアムバンダイ」をご利用いただきましてまことにありがとうございます。",
            "以下の通り佐川急便にてご注文商品を発送させていただきましたので、お知らせいたします。",
        ];
        assert_eq!(extract_carrier(&lines), Some("佐川急便".to_string()));
    }

    #[test]
    fn test_extract_carrier_yamato_from_body_text() {
        let lines = vec!["ヤマト運輸にてご注文商品を発送しました。"];
        assert_eq!(extract_carrier(&lines), Some("ヤマト運輸".to_string()));
    }

    #[test]
    fn test_carrier_tracking_url_sagawa() {
        let url = carrier_tracking_url("佐川急便", "123456789012");
        assert!(url.unwrap().contains("sagawa-exp.co.jp"));
    }

    #[test]
    fn test_carrier_tracking_url_yamato() {
        let url = carrier_tracking_url("ヤマト運輸", "123456789012");
        assert!(url.unwrap().contains("kuronekoyamato.co.jp"));
    }

    #[test]
    fn test_carrier_tracking_url_yupack() {
        let url = carrier_tracking_url("ゆうパック", "123456789012");
        assert!(url.unwrap().contains("post.japanpost.jp"));
    }

    #[test]
    fn test_carrier_tracking_url_unknown() {
        let url = carrier_tracking_url("未知の業者", "123456789012");
        assert!(url.is_none());
    }

    #[test]
    fn test_extract_image_urls_from_html_basic() {
        let html = r#"<img src="https://example.com/img/product1.jpg"><img src="https://example.com/img/product2.jpg">"#;
        let urls = extract_image_urls_from_html(html);
        assert_eq!(urls.len(), 2);
        assert!(urls[0].contains("product1.jpg"));
    }

    #[test]
    fn test_extract_image_urls_excludes_recommend_section() {
        let html = r#"<img src="https://example.com/img/order_product.jpg">おすすめ商品<img src="https://example.com/img/recommend.jpg">"#;
        let urls = extract_image_urls_from_html(html);
        assert_eq!(urls.len(), 1);
        assert!(urls[0].contains("order_product.jpg"));
    }

    #[test]
    fn test_extract_image_urls_filters_gif() {
        // GIF はスペーサー・トラッキングピクセル等として除外される
        let html = r#"<img src="https://example.com/logo.gif"><img src="https://example.com/img/product.jpg"><img src="https://example.com/spacer.GIF">"#;
        let urls = extract_image_urls_from_html(html);
        assert_eq!(urls.len(), 1, "GIF 2件が除外され JPEG 1件のみのはず");
        assert!(urls[0].contains("product.jpg"));
    }

    #[test]
    fn test_extract_image_urls_keeps_png_and_webp() {
        // PNG / WebP は GIF でないので除外されない
        let html = r#"<img src="https://example.com/img/product.png"><img src="https://example.com/img/product2.webp">"#;
        let urls = extract_image_urls_from_html(html);
        assert_eq!(urls.len(), 2);
    }

    #[test]
    fn test_find_recommend_section_line() {
        let lines = vec!["商品名A", "合計：￥5,000", "おすすめ商品", "商品名B"];
        assert_eq!(find_recommend_section_line(&lines), Some(2));
    }

    #[test]
    fn test_find_recommend_section_line_not_found() {
        let lines = vec!["商品名A", "合計：￥5,000"];
        assert_eq!(find_recommend_section_line(&lines), None);
    }

    #[test]
    fn test_body_to_lines_plain_text() {
        let body = "注文番号：12345\n送料：￥0";
        let lines = body_to_lines(body);
        assert_eq!(lines[0], "注文番号：12345");
    }

    #[test]
    fn test_body_to_lines_html() {
        let body = "<p>注文番号：12345<br>送料：￥0</p>";
        let lines = body_to_lines(body);
        assert!(lines.iter().any(|l| l == "注文番号：12345"));
    }

    /// `<br>` を含まないテーブル形式 HTML メールもタグが除去される（email 650 形式）
    ///
    /// 実際の HTML メールは `<th>`/`<td>` タグ間に改行があり、タグ除去後は別行になる。
    #[test]
    fn test_body_to_lines_html_table_no_br() {
        let body = "<table>\n<tr>\n<th>注文番号</th>\n<td>12345</td>\n</tr>\n</table>";
        let lines = body_to_lines(body);
        assert!(lines.iter().any(|l| l == "注文番号"), "got: {:?}", lines);
        assert!(lines.iter().any(|l| l == "12345"), "got: {:?}", lines);
    }

    /// `<td>` 内の価格行もタグが除去されて価格正規表現にマッチできる
    #[test]
    fn test_body_to_lines_html_table_price_line() {
        let body = "<table>\n<tr>\n<td>1,980円&times;1＝1,980円</td>\n</tr>\n</table>";
        let lines = body_to_lines(body);
        assert!(
            lines.iter().any(|l| l == "1,980円&times;1＝1,980円"),
            "got: {:?}",
            lines
        );
    }
}
