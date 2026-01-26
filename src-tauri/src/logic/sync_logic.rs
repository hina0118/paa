//! Gmail同期関連のビジネスロジック
//!
//! このモジュールはGmail同期に関する純粋関数を提供します。
//! 外部依存（Gmail API, DB）を持たないため、テストが容易です。

use crate::gmail::{GmailMessage, ShopSettings};
use crate::gmail_client::GmailClientTrait;

/// Gmail検索クエリを構築する
///
/// # Arguments
/// * `sender_addresses` - 検索対象の送信者アドレスリスト
/// * `oldest_date` - 検索の終了日（RFC3339形式）。この日付より前のメールを検索
///
/// # Returns
/// Gmailの検索クエリ文字列
///
/// # Examples
/// ```
/// use paa_lib::logic::sync_logic::build_sync_query;
///
/// let query = build_sync_query(&["shop@example.com".to_string()], &None);
/// assert_eq!(query, "from:shop@example.com");
///
/// let query = build_sync_query(
///     &["shop@example.com".to_string()],
///     &Some("2024-01-15T00:00:00Z".to_string()),
/// );
/// assert!(query.contains("before:2024/01/15"));
/// ```
pub fn build_sync_query(sender_addresses: &[String], oldest_date: &Option<String>) -> String {
    // Build query based on sender addresses
    let base_query = if sender_addresses.is_empty() {
        // Fallback to keyword search if no sender addresses configured
        log::warn!("No enabled shop settings found, falling back to keyword search");
        r"subject:(注文 OR 予約 OR ありがとうございます)".to_string()
    } else {
        // Build "from:addr1 OR from:addr2 OR ..." query
        let from_clauses: Vec<String> = sender_addresses
            .iter()
            .map(|addr| format!("from:{addr}"))
            .collect();
        from_clauses.join(" OR ")
    };

    if let Some(date) = oldest_date {
        // Parse and format for Gmail query (YYYY/MM/DD).
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date) {
            let before_date = dt.format("%Y/%m/%d");
            return format!("({base_query}) before:{before_date}");
        }
        // If parsing fails, log warning and use base query without date filter
        log::warn!("Invalid date format in oldest_date, ignoring date constraint: {date}");
    }

    base_query
}

/// "From"ヘッダーからメールアドレスを抽出する
///
/// # Arguments
/// * `from_header` - "From"ヘッダーの値
///
/// # Returns
/// 抽出されたメールアドレス（小文字）、抽出できない場合はNone
///
/// # Examples
/// ```
/// use paa_lib::logic::sync_logic::extract_email_address;
///
/// assert_eq!(
///     extract_email_address("John Doe <john@example.com>"),
///     Some("john@example.com".to_string())
/// );
/// assert_eq!(
///     extract_email_address("john@example.com"),
///     Some("john@example.com".to_string())
/// );
/// assert_eq!(extract_email_address("invalid"), None);
/// assert_eq!(extract_email_address("user@"), None);
/// assert_eq!(extract_email_address("a@b@c"), None);
/// ```
pub fn extract_email_address(from_header: &str) -> Option<String> {
    // Try to extract email from "Name <email@domain>" format
    if let Some(start) = from_header.find('<') {
        if let Some(end) = from_header.find('>') {
            if start < end {
                let candidate = from_header[start + 1..end].trim();
                if is_valid_simple_email(candidate) {
                    return Some(candidate.to_lowercase());
                }
            }
        }
    }

    // If no angle brackets, assume the whole string is an email candidate
    let trimmed = from_header.trim();
    if is_valid_simple_email(trimmed) {
        return Some(trimmed.to_lowercase());
    }

    None
}

/// 最低限の形式チェックを行うシンプルなメールアドレスバリデーション
/// - 全体をtrim
/// - '@'で分割して2要素のみ
/// - ローカル部・ドメイン部がともに非空
/// - 明らかな不正を避けるため、空白文字を含まない
fn is_valid_simple_email(email: &str) -> bool {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return false;
    }

    let parts: Vec<&str> = trimmed.split('@').collect();
    if parts.len() != 2 {
        return false;
    }

    let local = parts[0];
    let domain = parts[1];

    if local.is_empty() || domain.is_empty() {
        return false;
    }

    if local.contains(char::is_whitespace) || domain.contains(char::is_whitespace) {
        return false;
    }

    true
}

/// メッセージをショップ設定と件名フィルターに基づいて保存すべきかを判定する
///
/// # Arguments
/// * `msg` - 判定対象のGmailメッセージ
/// * `shop_settings` - 有効なショップ設定のリスト
///
/// # Returns
/// メッセージを保存すべき場合はtrue、そうでない場合はfalse
pub fn should_save_message(msg: &GmailMessage, shop_settings: &[ShopSettings]) -> bool {
    // Extract sender email address
    let sender_email = match &msg.from_address {
        Some(addr) => match extract_email_address(addr) {
            Some(email) => email,
            None => return false,
        },
        None => return false,
    };

    // Find matching shop setting
    for shop in shop_settings {
        if shop.sender_address.to_lowercase() == sender_email {
            // If no subject filter is set, allow the message
            if shop.subject_filters.is_none() {
                return true;
            }

            // If subject filters are set, check if message subject matches any filter
            let filters = shop.get_subject_filters();
            if filters.is_empty() {
                return true;
            }

            // Check if subject matches any filter
            if let Some(subject) = &msg.subject {
                for filter in &filters {
                    if subject.contains(filter) {
                        return true;
                    }
                }
            }

            // Subject doesn't match any filter
            return false;
        }
    }

    // No matching shop setting found
    false
}

/// タイムスタンプをRFC3339形式の文字列に変換する
///
/// # Arguments
/// * `internal_date` - Gmailのinternal_date（ミリ秒単位のUnixタイムスタンプ）
///
/// # Returns
/// RFC3339形式の日時文字列。無効なタイムスタンプの場合は空文字列
pub fn format_timestamp(internal_date: i64) -> String {
    chrono::DateTime::from_timestamp_millis(internal_date)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| {
            log::warn!(
                "Invalid internal_date '{internal_date}' encountered when formatting timestamp"
            );
            String::new()
        })
}

/// ショップ設定からメールアドレスのリストを抽出する
///
/// # Arguments
/// * `shop_settings` - ショップ設定のリスト
///
/// # Returns
/// 送信者アドレスのリスト
pub fn extract_sender_addresses(shop_settings: &[ShopSettings]) -> Vec<String> {
    shop_settings
        .iter()
        .map(|s| s.sender_address.clone())
        .collect()
}

/// メッセージをショップ設定でフィルタリングする
///
/// # Arguments
/// * `messages` - フィルタリング対象のメッセージリスト
/// * `shop_settings` - 有効なショップ設定のリスト
///
/// # Returns
/// (保存すべきメッセージ, フィルタで除外されたメッセージ数)
pub fn filter_messages_by_shop_settings(
    messages: &[GmailMessage],
    shop_settings: &[ShopSettings],
) -> (Vec<GmailMessage>, usize) {
    let mut filtered_messages = Vec::new();
    let mut filtered_out_count = 0;

    for msg in messages {
        if should_save_message(msg, shop_settings) {
            filtered_messages.push(msg.clone());
        } else {
            filtered_out_count += 1;
        }
    }

    (filtered_messages, filtered_out_count)
}

/// GmailClientTraitを使用してメッセージのバッチを取得する
///
/// # Arguments
/// * `client` - GmailClientTraitを実装したクライアント
/// * `query` - Gmail検索クエリ
/// * `max_results` - 最大取得数
///
/// # Returns
/// 取得したGmailMessageのVec、またはエラー
pub async fn fetch_batch_with_client(
    client: &dyn GmailClientTrait,
    query: &str,
    max_results: usize,
) -> Result<Vec<GmailMessage>, String> {
    // メッセージIDリストを取得
    let message_ids = client.list_message_ids(query, max_results as u32).await?;

    let mut messages = Vec::new();
    for id in message_ids {
        match client.get_message(&id).await {
            Ok(msg) => messages.push(msg),
            Err(e) => log::warn!("Failed to fetch message {id}: {e}"),
        }
    }

    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== build_sync_query Tests ====================

    #[test]
    fn test_build_sync_query_single_address() {
        let query = build_sync_query(&["shop@example.com".to_string()], &None);
        assert_eq!(query, "from:shop@example.com");
    }

    #[test]
    fn test_build_sync_query_multiple_addresses() {
        let addresses = vec![
            "shop1@example.com".to_string(),
            "shop2@example.com".to_string(),
        ];
        let query = build_sync_query(&addresses, &None);
        assert_eq!(query, "from:shop1@example.com OR from:shop2@example.com");
    }

    #[test]
    fn test_build_sync_query_empty_addresses() {
        let query = build_sync_query(&[], &None);
        assert!(query.contains("subject:"));
    }

    #[test]
    fn test_build_sync_query_with_valid_date() {
        let query = build_sync_query(
            &["shop@example.com".to_string()],
            &Some("2024-01-15T10:30:00+09:00".to_string()),
        );
        assert!(query.contains("from:shop@example.com"));
        assert!(query.contains("before:2024/01/15"));
    }

    #[test]
    fn test_build_sync_query_with_invalid_date() {
        let query = build_sync_query(
            &["shop@example.com".to_string()],
            &Some("invalid-date".to_string()),
        );
        // Invalid date should be ignored
        assert_eq!(query, "from:shop@example.com");
    }

    #[test]
    fn test_build_sync_query_with_utc_date() {
        let query = build_sync_query(
            &["shop@example.com".to_string()],
            &Some("2024-06-01T00:00:00Z".to_string()),
        );
        assert!(query.contains("before:2024/06/01"));
    }

    // ==================== extract_email_address Tests ====================

    #[test]
    fn test_extract_email_address_with_name() {
        let result = extract_email_address("John Doe <john@example.com>");
        assert_eq!(result, Some("john@example.com".to_string()));
    }

    #[test]
    fn test_extract_email_address_plain() {
        let result = extract_email_address("john@example.com");
        assert_eq!(result, Some("john@example.com".to_string()));
    }

    #[test]
    fn test_extract_email_address_with_spaces() {
        let result = extract_email_address("  john@example.com  ");
        assert_eq!(result, Some("john@example.com".to_string()));
    }

    #[test]
    fn test_extract_email_address_uppercase() {
        let result = extract_email_address("John@EXAMPLE.COM");
        assert_eq!(result, Some("john@example.com".to_string()));
    }

    #[test]
    fn test_extract_email_address_invalid() {
        let result = extract_email_address("not-an-email");
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_email_address_empty() {
        let result = extract_email_address("");
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_email_address_japanese_name() {
        let result = extract_email_address("山田太郎 <yamada@example.co.jp>");
        assert_eq!(result, Some("yamada@example.co.jp".to_string()));
    }

    // ==================== should_save_message Tests ====================

    fn create_test_message(from: Option<&str>, subject: Option<&str>) -> GmailMessage {
        GmailMessage {
            message_id: "test123".to_string(),
            snippet: "test snippet".to_string(),
            subject: subject.map(String::from),
            body_plain: None,
            body_html: None,
            internal_date: 1704067200000,
            from_address: from.map(String::from),
        }
    }

    fn create_shop_setting(address: &str, filters: Option<Vec<String>>) -> ShopSettings {
        ShopSettings {
            id: 1,
            shop_name: "Test Shop".to_string(),
            sender_address: address.to_string(),
            parser_type: "hobbysearch_confirm".to_string(),
            is_enabled: true,
            subject_filters: filters.map(|f| serde_json::to_string(&f).unwrap()),
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
        }
    }

    #[test]
    fn test_should_save_message_matching_address_no_filter() {
        let msg = create_test_message(Some("shop@example.com"), Some("注文確認"));
        let settings = vec![create_shop_setting("shop@example.com", None)];

        assert!(should_save_message(&msg, &settings));
    }

    #[test]
    fn test_should_save_message_matching_address_matching_filter() {
        let msg = create_test_message(Some("shop@example.com"), Some("注文確認メール"));
        let settings = vec![create_shop_setting(
            "shop@example.com",
            Some(vec!["注文確認".to_string()]),
        )];

        assert!(should_save_message(&msg, &settings));
    }

    #[test]
    fn test_should_save_message_matching_address_non_matching_filter() {
        let msg = create_test_message(Some("shop@example.com"), Some("広告メール"));
        let settings = vec![create_shop_setting(
            "shop@example.com",
            Some(vec!["注文確認".to_string()]),
        )];

        assert!(!should_save_message(&msg, &settings));
    }

    #[test]
    fn test_should_save_message_non_matching_address() {
        let msg = create_test_message(Some("other@example.com"), Some("注文確認"));
        let settings = vec![create_shop_setting("shop@example.com", None)];

        assert!(!should_save_message(&msg, &settings));
    }

    #[test]
    fn test_should_save_message_no_from_address() {
        let msg = create_test_message(None, Some("注文確認"));
        let settings = vec![create_shop_setting("shop@example.com", None)];

        assert!(!should_save_message(&msg, &settings));
    }

    #[test]
    fn test_should_save_message_empty_settings() {
        let msg = create_test_message(Some("shop@example.com"), Some("注文確認"));
        let settings: Vec<ShopSettings> = vec![];

        assert!(!should_save_message(&msg, &settings));
    }

    // ==================== format_timestamp Tests ====================

    #[test]
    fn test_format_timestamp_valid() {
        // 2024-01-01 00:00:00 UTC in milliseconds
        let result = format_timestamp(1704067200000);
        assert!(result.contains("2024-01-01"));
    }

    #[test]
    fn test_format_timestamp_zero() {
        // Unix epoch
        let result = format_timestamp(0);
        assert!(result.contains("1970-01-01"));
    }

    #[test]
    fn test_format_timestamp_invalid_negative() {
        // Very large negative number that can't be represented
        let result = format_timestamp(i64::MIN);
        assert!(result.is_empty());
    }

    // ==================== extract_sender_addresses Tests ====================

    #[test]
    fn test_extract_sender_addresses() {
        let settings = vec![
            create_shop_setting("shop1@example.com", None),
            create_shop_setting("shop2@example.com", None),
        ];

        let addresses = extract_sender_addresses(&settings);

        assert_eq!(addresses.len(), 2);
        assert!(addresses.contains(&"shop1@example.com".to_string()));
        assert!(addresses.contains(&"shop2@example.com".to_string()));
    }

    #[test]
    fn test_extract_sender_addresses_empty() {
        let settings: Vec<ShopSettings> = vec![];
        let addresses = extract_sender_addresses(&settings);
        assert!(addresses.is_empty());
    }

    // ==================== filter_messages_by_shop_settings Tests ====================

    #[test]
    fn test_filter_messages_by_shop_settings_all_pass() {
        let messages = vec![
            create_test_message(Some("shop@example.com"), Some("注文確認メール")),
            create_test_message(Some("shop@example.com"), Some("注文受付完了")),
        ];
        let settings = vec![create_shop_setting(
            "shop@example.com",
            Some(vec!["注文".to_string()]),
        )];

        let (filtered, filtered_out) =
            super::filter_messages_by_shop_settings(&messages, &settings);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered_out, 0);
    }

    #[test]
    fn test_filter_messages_by_shop_settings_some_filtered() {
        let messages = vec![
            create_test_message(Some("shop@example.com"), Some("注文確認メール")),
            create_test_message(Some("shop@example.com"), Some("キャンペーンのお知らせ")),
        ];
        let settings = vec![create_shop_setting(
            "shop@example.com",
            Some(vec!["注文".to_string()]),
        )];

        let (filtered, filtered_out) =
            super::filter_messages_by_shop_settings(&messages, &settings);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered_out, 1);
        assert!(filtered[0].subject.as_ref().unwrap().contains("注文"));
    }

    #[test]
    fn test_filter_messages_by_shop_settings_all_filtered() {
        let messages = vec![
            create_test_message(Some("other@example.com"), Some("注文確認メール")),
            create_test_message(Some("shop@example.com"), Some("広告メール")),
        ];
        let settings = vec![create_shop_setting(
            "shop@example.com",
            Some(vec!["注文".to_string()]),
        )];

        let (filtered, filtered_out) =
            super::filter_messages_by_shop_settings(&messages, &settings);

        assert_eq!(filtered.len(), 0);
        assert_eq!(filtered_out, 2);
    }

    #[test]
    fn test_filter_messages_by_shop_settings_empty_messages() {
        let messages: Vec<GmailMessage> = vec![];
        let settings = vec![create_shop_setting("shop@example.com", None)];

        let (filtered, filtered_out) =
            super::filter_messages_by_shop_settings(&messages, &settings);

        assert!(filtered.is_empty());
        assert_eq!(filtered_out, 0);
    }

    // ==================== fetch_batch_with_client Tests ====================

    use crate::gmail_client::MockGmailClientTrait;

    #[tokio::test]
    async fn test_fetch_batch_with_client_success() {
        let mut mock = MockGmailClientTrait::new();

        // list_message_idsのモック設定
        mock.expect_list_message_ids()
            .withf(|query, max_results| query == "from:shop@example.com" && *max_results == 10)
            .returning(|_, _| Ok(vec!["msg1".to_string(), "msg2".to_string()]));

        // get_messageのモック設定
        mock.expect_get_message()
            .withf(|id| id == "msg1")
            .returning(|_| {
                Ok(GmailMessage {
                    message_id: "msg1".to_string(),
                    snippet: "Snippet 1".to_string(),
                    subject: Some("Subject 1".to_string()),
                    body_plain: Some("Body 1".to_string()),
                    body_html: None,
                    internal_date: 1704067200000,
                    from_address: Some("shop@example.com".to_string()),
                })
            });

        mock.expect_get_message()
            .withf(|id| id == "msg2")
            .returning(|_| {
                Ok(GmailMessage {
                    message_id: "msg2".to_string(),
                    snippet: "Snippet 2".to_string(),
                    subject: Some("Subject 2".to_string()),
                    body_plain: Some("Body 2".to_string()),
                    body_html: None,
                    internal_date: 1704153600000,
                    from_address: Some("shop@example.com".to_string()),
                })
            });

        let result = super::fetch_batch_with_client(&mock, "from:shop@example.com", 10).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].message_id, "msg1");
        assert_eq!(messages[1].message_id, "msg2");
    }

    #[tokio::test]
    async fn test_fetch_batch_with_client_list_error() {
        let mut mock = MockGmailClientTrait::new();

        mock.expect_list_message_ids()
            .returning(|_, _| Err("API error".to_string()));

        let result = super::fetch_batch_with_client(&mock, "query", 10).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "API error");
    }

    #[tokio::test]
    async fn test_fetch_batch_with_client_partial_fetch_error() {
        let mut mock = MockGmailClientTrait::new();

        mock.expect_list_message_ids()
            .returning(|_, _| Ok(vec!["msg1".to_string(), "msg2".to_string()]));

        // msg1は成功、msg2は失敗
        mock.expect_get_message()
            .withf(|id| id == "msg1")
            .returning(|_| {
                Ok(GmailMessage {
                    message_id: "msg1".to_string(),
                    snippet: "Snippet".to_string(),
                    subject: Some("Subject".to_string()),
                    body_plain: None,
                    body_html: None,
                    internal_date: 1704067200000,
                    from_address: None,
                })
            });

        mock.expect_get_message()
            .withf(|id| id == "msg2")
            .returning(|_| Err("Fetch error".to_string()));

        let result = super::fetch_batch_with_client(&mock, "query", 10).await;

        // 部分的な失敗はワーニングログのみで、成功したメッセージは返される
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message_id, "msg1");
    }

    #[tokio::test]
    async fn test_fetch_batch_with_client_empty_result() {
        let mut mock = MockGmailClientTrait::new();

        mock.expect_list_message_ids().returning(|_, _| Ok(vec![]));

        let result = super::fetch_batch_with_client(&mock, "query", 10).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
