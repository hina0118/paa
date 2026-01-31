//! メールパース関連のビジネスロジック
//!
//! このモジュールはメールパースに関する純粋関数を提供します。
//! 外部依存を持たないため、テストが容易です。

use crate::logic::sync_logic::extract_email_address;
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
            | "hobbysearch_change_yoyaku"
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
/// * `from_address` - 送信者アドレス（"Name <email@domain>" 形式も可）
/// * `subject` - メール件名
/// * `shop_settings` - ショップ設定リスト（タプル: (sender_address, parser_type, subject_filters_json)）
///
/// # Returns
/// マッチするパーサータイプのリスト
///
/// # Note
/// - メールアドレスは正規化（小文字化）して完全一致で比較
/// - 大文字小文字は無視される
pub fn get_candidate_parsers<'a>(
    from_address: &str,
    subject: Option<&str>,
    shop_settings: &'a [(String, String, Option<String>)],
) -> Vec<&'a str> {
    // from_addressからメールアドレスを抽出して正規化
    let normalized_from = match extract_email_address(from_address) {
        Some(email) => email,
        None => return vec![], // 有効なメールアドレスが抽出できない場合は空を返す
    };

    shop_settings
        .iter()
        .filter_map(|(addr, parser_type, subject_filters_json)| {
            // 送信元アドレスが完全一致するか確認（大文字小文字無視、allocなし）
            if !addr.eq_ignore_ascii_case(&normalized_from) {
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

            // 空のフィルターリストは「フィルターなし＝全許可」と同じ扱い
            // (should_save_message や ShopSettings::get_subject_filters との整合性)
            if filters.is_empty() {
                return Some(parser_type.as_str());
            }

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
/// ドメイン部分（@の後ろ）。@ が 1 つでドメイン部が非空の場合のみ `Some` を返す。
pub fn extract_domain(email: &str) -> Option<&str> {
    let mut parts = email.split('@');

    // ローカル部（@ の前）は何でもよいが、@ が無い場合は None
    let _local = parts.next()?;

    // ドメイン部（@ の後ろ）が存在しない場合は None
    let domain = parts.next()?;

    // 追加の @ が存在する場合（@ が 2 個以上）は無効
    if parts.next().is_some() {
        return None;
    }

    // ドメイン部が空文字列の場合は無効
    if domain.is_empty() {
        return None;
    }

    Some(domain)
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
    fn test_is_valid_parser_type_hobbysearch_change_yoyaku() {
        assert!(is_valid_parser_type("hobbysearch_change_yoyaku"));
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

        let candidates =
            get_candidate_parsers("shop@example.com", Some("注文確認メール"), &settings);

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

        let candidates =
            get_candidate_parsers("shop@example.com", Some("注文確認メール"), &settings);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], "hobbysearch_confirm");
    }

    #[test]
    fn test_get_candidate_parsers_with_empty_subject_filter() {
        // 空のフィルターリスト（"[]"）は「フィルターなし＝全許可」と同じ扱い
        let settings = vec![(
            "shop@example.com".to_string(),
            "hobbysearch_confirm".to_string(),
            Some(r#"[]"#.to_string()), // 空のフィルターリスト
        )];

        let candidates = get_candidate_parsers("shop@example.com", Some("任意の件名"), &settings);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], "hobbysearch_confirm");
    }

    #[test]
    fn test_get_candidate_parsers_with_empty_subject_filter_no_subject() {
        // 空のフィルターリストは件名がなくてもマッチ
        let settings = vec![(
            "shop@example.com".to_string(),
            "hobbysearch_confirm".to_string(),
            Some(r#"[]"#.to_string()),
        )];

        let candidates = get_candidate_parsers("shop@example.com", None, &settings);

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
        // "@" のみの場合、ドメイン部が空なのでNoneを返す
        assert_eq!(extract_domain("@"), None);
    }

    #[test]
    fn test_extract_domain_multiple_at() {
        // @ が複数ある場合は無効
        assert_eq!(extract_domain("a@b@c"), None);
    }

    #[test]
    fn test_extract_domain_trailing_at() {
        // "user@" の場合、ドメイン部が空なのでNone
        assert_eq!(extract_domain("user@"), None);
    }
}
