//! メールパース関連のビジネスロジック
//!
//! このモジュールはメールパースに関する純粋関数を提供します。
//! 外部依存を持たないため、テストが容易です。

use crate::parsers::{EmailParser, OrderInfo};

/// パーサータイプ名からパーサーが存在するかチェックする
///
/// # Arguments
/// * `parser_type` - パーサータイプ名
///
/// # Returns
/// パーサーが存在する場合はtrue
pub fn is_valid_parser_type(parser_type: &str) -> bool {
    matches!(
        parser_type,
        "hobbysearch_confirm"
            | "hobbysearch_confirm_yoyaku"
            | "hobbysearch_change"
            | "hobbysearch_send"
    )
}

/// パースを試行し、結果を返す
///
/// # Arguments
/// * `parser` - パーサーインスタンス
/// * `email_body` - メール本文
///
/// # Returns
/// パース結果
pub fn try_parse(parser: &dyn EmailParser, email_body: &str) -> Result<OrderInfo, String> {
    parser.parse(email_body)
}

/// 送信者アドレスと件名からパーサータイプの候補を取得する
///
/// # Arguments
/// * `from_address` - 送信者アドレス
/// * `subject` - メール件名
/// * `shop_settings` - ショップ設定リスト（タプル: (sender_address, parser_type, subject_filters_json)）
///
/// # Returns
/// マッチするパーサータイプのリスト
pub fn get_candidate_parsers<'a>(
    from_address: &str,
    subject: Option<&str>,
    shop_settings: &'a [(String, String, Option<String>)],
) -> Vec<&'a str> {
    shop_settings
        .iter()
        .filter_map(|(addr, parser_type, subject_filters_json)| {
            // 送信元アドレスが一致するか確認
            if !from_address.contains(addr) {
                return None;
            }

            // 件名フィルターがない場合は、アドレス一致だけでOK
            let Some(filters_json) = subject_filters_json else {
                return Some(parser_type.as_str());
            };

            // 件名フィルターがある場合は、件名も確認
            let Ok(filters) = serde_json::from_str::<Vec<String>>(filters_json) else {
                return Some(parser_type.as_str()); // JSONパースエラー時はフィルター無視
            };

            // 件名がない場合は除外
            let subj = subject?;

            // いずれかのフィルターに一致すればOK
            if filters.iter().any(|filter| subj.contains(filter)) {
                Some(parser_type.as_str())
            } else {
                None
            }
        })
        .collect()
}

/// ドメインをメールアドレスから抽出する
///
/// # Arguments
/// * `email` - メールアドレス
///
/// # Returns
/// ドメイン部分（@の後ろ）
pub fn extract_domain(email: &str) -> Option<&str> {
    email.split('@').nth(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== is_valid_parser_type Tests ====================

    #[test]
    fn test_is_valid_parser_type_hobbysearch_confirm() {
        assert!(is_valid_parser_type("hobbysearch_confirm"));
    }

    #[test]
    fn test_is_valid_parser_type_hobbysearch_confirm_yoyaku() {
        assert!(is_valid_parser_type("hobbysearch_confirm_yoyaku"));
    }

    #[test]
    fn test_is_valid_parser_type_hobbysearch_change() {
        assert!(is_valid_parser_type("hobbysearch_change"));
    }

    #[test]
    fn test_is_valid_parser_type_hobbysearch_send() {
        assert!(is_valid_parser_type("hobbysearch_send"));
    }

    #[test]
    fn test_is_valid_parser_type_unknown() {
        assert!(!is_valid_parser_type("unknown_parser"));
    }

    #[test]
    fn test_is_valid_parser_type_empty() {
        assert!(!is_valid_parser_type(""));
    }

    // ==================== get_candidate_parsers Tests ====================

    #[test]
    fn test_get_candidate_parsers_single_match() {
        let settings = vec![(
            "shop@example.com".to_string(),
            "hobbysearch_confirm".to_string(),
            None,
        )];

        let candidates = get_candidate_parsers("shop@example.com", Some("注文確認"), &settings);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], "hobbysearch_confirm");
    }

    #[test]
    fn test_get_candidate_parsers_with_subject_filter_match() {
        let settings = vec![(
            "shop@example.com".to_string(),
            "hobbysearch_confirm".to_string(),
            Some(r#"["注文確認"]"#.to_string()),
        )];

        let candidates = get_candidate_parsers("shop@example.com", Some("注文確認メール"), &settings);

        assert_eq!(candidates.len(), 1);
    }

    #[test]
    fn test_get_candidate_parsers_with_subject_filter_no_match() {
        let settings = vec![(
            "shop@example.com".to_string(),
            "hobbysearch_confirm".to_string(),
            Some(r#"["注文確認"]"#.to_string()),
        )];

        let candidates = get_candidate_parsers("shop@example.com", Some("広告メール"), &settings);

        assert!(candidates.is_empty());
    }

    #[test]
    fn test_get_candidate_parsers_address_no_match() {
        let settings = vec![(
            "shop@example.com".to_string(),
            "hobbysearch_confirm".to_string(),
            None,
        )];

        let candidates = get_candidate_parsers("other@example.com", Some("注文確認"), &settings);

        assert!(candidates.is_empty());
    }

    #[test]
    fn test_get_candidate_parsers_multiple_matches() {
        let settings = vec![
            (
                "shop@example.com".to_string(),
                "hobbysearch_confirm".to_string(),
                Some(r#"["注文確認"]"#.to_string()),
            ),
            (
                "shop@example.com".to_string(),
                "hobbysearch_send".to_string(),
                Some(r#"["発送"]"#.to_string()),
            ),
        ];

        let candidates = get_candidate_parsers("shop@example.com", Some("注文確認メール"), &settings);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], "hobbysearch_confirm");
    }

    // ==================== extract_domain Tests ====================

    #[test]
    fn test_extract_domain_valid() {
        assert_eq!(extract_domain("user@example.com"), Some("example.com"));
    }

    #[test]
    fn test_extract_domain_subdomain() {
        assert_eq!(
            extract_domain("user@mail.example.co.jp"),
            Some("mail.example.co.jp")
        );
    }

    #[test]
    fn test_extract_domain_no_at() {
        assert_eq!(extract_domain("not-an-email"), None);
    }

    #[test]
    fn test_extract_domain_empty() {
        assert_eq!(extract_domain(""), None);
    }

    #[test]
    fn test_extract_domain_only_at() {
        assert_eq!(extract_domain("@"), Some(""));
    }
}
