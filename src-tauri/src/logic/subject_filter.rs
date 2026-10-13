//! ショップ設定の送信元・件名フィルター判定
//!
//! Gmail 同期時の保存判定（`should_save_message`）とメールパース時のパーサー候補選択で
//! 同じ判定規則を使うための共通実装。
//!
//! # 件名フィルターの規則
//! - 未設定・JSON パースエラー・空リストはフィルターなし（全許可）
//! - フィルターがあるのに件名がない場合は除外
//! - いずれかのフィルターに一致すれば通過（正規表現として解釈し、無効な正規表現は部分一致）

use crate::gmail::ShopSettings;

/// 件名フィルターの1パターン
#[derive(Debug, Clone)]
enum SubjectPattern {
    Regex(regex::Regex),
    Contains(String),
}

impl SubjectPattern {
    fn new(filter: String) -> Self {
        match regex::Regex::new(&filter) {
            Ok(re) => Self::Regex(re),
            Err(_) => Self::Contains(filter),
        }
    }

    fn is_match(&self, subject: &str) -> bool {
        match self {
            Self::Regex(re) => re.is_match(subject),
            Self::Contains(s) => subject.contains(s.as_str()),
        }
    }
}

/// コンパイル済みの件名フィルター
///
/// JSON 解析と正規表現のコンパイルは生成時に1回だけ行い、判定ごとに繰り返さない。
#[derive(Debug, Clone, Default)]
pub struct SubjectFilter {
    /// `None` はフィルターなし（全許可）
    patterns: Option<Vec<SubjectPattern>>,
}

impl SubjectFilter {
    /// `shop_settings.subject_filters`（JSON 配列文字列）から生成する
    pub fn from_json(subject_filters: Option<&str>) -> Self {
        let patterns = subject_filters
            .and_then(|json| serde_json::from_str::<Vec<String>>(json).ok())
            .filter(|list| !list.is_empty())
            .map(|list| list.into_iter().map(SubjectPattern::new).collect());
        Self { patterns }
    }

    /// 件名がフィルターを通過するか
    pub fn matches(&self, subject: Option<&str>) -> bool {
        match (&self.patterns, subject) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(patterns), Some(subj)) => patterns.iter().any(|p| p.is_match(subj)),
        }
    }
}

/// 判定用に前処理済みのショップ設定
#[derive(Debug, Clone)]
pub struct CompiledShopSetting {
    pub sender_address: String,
    pub parser_type: String,
    pub shop_name: String,
    pub subject_filter: SubjectFilter,
}

impl CompiledShopSetting {
    pub fn new(
        sender_address: String,
        parser_type: String,
        subject_filters: Option<&str>,
        shop_name: String,
    ) -> Self {
        Self {
            sender_address,
            parser_type,
            shop_name,
            subject_filter: SubjectFilter::from_json(subject_filters),
        }
    }

    /// `ShopSettings` のリストをまとめて前処理する
    pub fn compile_all(settings: &[ShopSettings]) -> Vec<Self> {
        settings.iter().map(Self::from).collect()
    }

    /// 送信元アドレス（正規化済み）が一致し、件名がフィルターを通過するか
    pub fn matches(&self, normalized_from: &str, subject: Option<&str>) -> bool {
        self.sender_address.eq_ignore_ascii_case(normalized_from)
            && self.subject_filter.matches(subject)
    }
}

impl From<&ShopSettings> for CompiledShopSetting {
    fn from(s: &ShopSettings) -> Self {
        Self::new(
            s.sender_address.clone(),
            s.parser_type.clone(),
            s.subject_filters.as_deref(),
            s.shop_name.clone(),
        )
    }
}

impl From<ShopSettings> for CompiledShopSetting {
    fn from(s: ShopSettings) -> Self {
        let subject_filter = SubjectFilter::from_json(s.subject_filters.as_deref());
        Self {
            sender_address: s.sender_address,
            parser_type: s.parser_type,
            shop_name: s.shop_name,
            subject_filter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_filter_allows_all() {
        let f = SubjectFilter::from_json(None);
        assert!(f.matches(None));
        assert!(f.matches(Some("何でも")));
    }

    #[test]
    fn test_invalid_json_and_empty_list_allow_all() {
        assert!(SubjectFilter::from_json(Some("not json")).matches(None));
        assert!(SubjectFilter::from_json(Some("[]")).matches(None));
    }

    #[test]
    fn test_filter_requires_subject() {
        let f = SubjectFilter::from_json(Some(r#"["注文確認"]"#));
        assert!(!f.matches(None));
        assert!(f.matches(Some("【注文確認】ありがとう")));
        assert!(!f.matches(Some("発送のお知らせ")));
    }

    #[test]
    fn test_regex_and_invalid_regex_fallback() {
        let f = SubjectFilter::from_json(Some(r#"["ご注文番号：.*", "[invalid"]"#));
        assert!(f.matches(Some("ご注文番号：ABC")));
        assert!(f.matches(Some("件名 [invalid パターン")));
        assert!(!f.matches(Some("キャンセル")));
    }

    #[test]
    fn test_compiled_setting_matches_sender_case_insensitive() {
        let s = CompiledShopSetting::new(
            "Shop@Example.com".to_string(),
            "p".to_string(),
            None,
            "Shop".to_string(),
        );
        assert!(s.matches("shop@example.com", None));
        assert!(!s.matches("other@example.com", None));
    }
}
