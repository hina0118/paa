//! パーサーの統合テスト
//!
//! サンプルメール形式のテストデータを使用してパーサーの動作を検証します。
//! 個人情報を含まないダミーデータを使用しています。

use paa_lib::logic::email_parser::{extract_domain, get_candidate_parsers, is_valid_parser_type};
use paa_lib::logic::sync_logic::{
    build_sync_query, extract_email_address, extract_sender_addresses, format_timestamp,
    should_save_message,
};

/// ホビーサーチ注文確認メールのサンプル（ダミーデータ）
const SAMPLE_HOBBYSEARCH_CONFIRM: &str = r#"
ホビーサーチ注文確認メール

このメールはお客様のご注文を確認するために送信しています。

[注文番号] 25-0101-1234

[お届け先情報]
〒100-0001
東京都千代田区千代田1-1-1
テスト 太郎 様

[ご購入内容]
バンダイ 1234567 テスト商品A (プラモデル) HGシリーズ
単価：1,000円 × 個数：2 = 2,000円
バンダイ 2345678 テスト商品B (プラモデル) MGシリーズ
単価：3,000円 × 個数：1 = 3,000円

小計：5,000円
送料：660円
合計：5,660円
"#;

/// ホビーサーチ発送通知メールのサンプル（ダミーデータ）
const SAMPLE_HOBBYSEARCH_SEND: &str = r#"
ホビーサーチ発送通知メール

ご注文の商品を発送いたしました。

[注文番号] 25-0101-1234

[配送情報]
配送業者：ヤマト運輸
伝票番号：1234-5678-9012

[お届け先情報]
〒100-0001
東京都千代田区千代田1-1-1
テスト 太郎 様

[発送内容]
バンダイ 1234567 テスト商品A (プラモデル) HGシリーズ
単価：1,000円 × 個数：2 = 2,000円

合計：2,660円
"#;

// ==================== sync_logic Tests ====================

#[test]
fn test_build_sync_query_integration() {
    // 実際のユースケース: 複数ショップからの検索クエリ構築
    let addresses = vec![
        "order@hobbysearch.co.jp".to_string(),
        "info@anotherstore.com".to_string(),
    ];

    let query = build_sync_query(&addresses, &None);
    assert!(query.contains("from:order@hobbysearch.co.jp"));
    assert!(query.contains("from:info@anotherstore.com"));
    assert!(query.contains(" OR "));

    // 日付フィルター付き
    let query_with_date = build_sync_query(
        &addresses,
        &Some("2024-06-15T12:00:00+09:00".to_string()),
    );
    assert!(query_with_date.contains("before:2024/06/15"));
}

#[test]
fn test_extract_email_address_integration() {
    // 実際のFromヘッダー形式
    let test_cases = vec![
        (
            "ホビーサーチ <order@hobbysearch.co.jp>",
            Some("order@hobbysearch.co.jp".to_string()),
        ),
        (
            "\"Amazon.co.jp\" <auto-confirm@amazon.co.jp>",
            Some("auto-confirm@amazon.co.jp".to_string()),
        ),
        (
            "noreply@example.com",
            Some("noreply@example.com".to_string()),
        ),
        (
            "  spaced@example.com  ",
            Some("spaced@example.com".to_string()),
        ),
    ];

    for (input, expected) in test_cases {
        assert_eq!(extract_email_address(input), expected, "Failed for: {input}");
    }
}

#[test]
fn test_format_timestamp_integration() {
    // 実際のGmail internal_date値（ミリ秒）
    let test_cases = vec![
        (1704067200000i64, "2024-01-01"), // 2024-01-01 00:00:00 UTC
        (1719792000000i64, "2024-07-01"), // 2024-07-01 00:00:00 UTC
        (0i64, "1970-01-01"),             // Unix epoch
    ];

    for (timestamp, expected_date) in test_cases {
        let result = format_timestamp(timestamp);
        assert!(
            result.contains(expected_date),
            "Expected {} to contain {}, got {}",
            timestamp,
            expected_date,
            result
        );
    }
}

// ==================== email_parser Tests ====================

#[test]
fn test_is_valid_parser_type_integration() {
    // 有効なパーサータイプ
    let valid_types = [
        "hobbysearch_confirm",
        "hobbysearch_confirm_yoyaku",
        "hobbysearch_change",
        "hobbysearch_send",
    ];

    for parser_type in valid_types {
        assert!(
            is_valid_parser_type(parser_type),
            "Expected {} to be valid",
            parser_type
        );
    }

    // 無効なパーサータイプ
    let invalid_types = ["amazon", "rakuten", "unknown", ""];

    for parser_type in invalid_types {
        assert!(
            !is_valid_parser_type(parser_type),
            "Expected {} to be invalid",
            parser_type
        );
    }
}

#[test]
fn test_get_candidate_parsers_integration() {
    // 実際のショップ設定を模倣
    let shop_settings = vec![
        (
            "hobbysearch.co.jp".to_string(),
            "hobbysearch_confirm".to_string(),
            Some(r#"["注文確認", "ご注文"]"#.to_string()),
        ),
        (
            "hobbysearch.co.jp".to_string(),
            "hobbysearch_send".to_string(),
            Some(r#"["発送", "出荷"]"#.to_string()),
        ),
        (
            "anotherstore.com".to_string(),
            "hobbysearch_confirm".to_string(),
            None,
        ),
    ];

    // 注文確認メールのケース
    let candidates = get_candidate_parsers(
        "order@hobbysearch.co.jp",
        Some("【ホビーサーチ】ご注文確認"),
        &shop_settings,
    );
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0], "hobbysearch_confirm");

    // 発送通知メールのケース
    let candidates = get_candidate_parsers(
        "order@hobbysearch.co.jp",
        Some("【ホビーサーチ】商品発送のお知らせ"),
        &shop_settings,
    );
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0], "hobbysearch_send");

    // マッチしないケース
    let candidates = get_candidate_parsers(
        "order@hobbysearch.co.jp",
        Some("キャンペーンのお知らせ"),
        &shop_settings,
    );
    assert!(candidates.is_empty());
}

#[test]
fn test_extract_domain_integration() {
    let test_cases = vec![
        ("order@hobbysearch.co.jp", Some("hobbysearch.co.jp")),
        ("noreply@amazon.co.jp", Some("amazon.co.jp")),
        ("info@mail.rakuten.co.jp", Some("mail.rakuten.co.jp")),
        ("invalid", None),
    ];

    for (email, expected) in test_cases {
        assert_eq!(
            extract_domain(email),
            expected,
            "Failed for: {}",
            email
        );
    }
}

// ==================== End-to-End Workflow Tests ====================

#[test]
fn test_email_processing_workflow() {
    // 1. ショップ設定からアドレスを抽出
    use paa_lib::gmail::ShopSettings;

    let shop_settings = vec![
        ShopSettings {
            id: 1,
            shop_name: "ホビーサーチ".to_string(),
            sender_address: "order@hobbysearch.co.jp".to_string(),
            parser_type: "hobbysearch_confirm".to_string(),
            is_enabled: true,
            subject_filters: Some(r#"["注文確認"]"#.to_string()),
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
        },
    ];

    let addresses = extract_sender_addresses(&shop_settings);
    assert_eq!(addresses.len(), 1);
    assert_eq!(addresses[0], "order@hobbysearch.co.jp");

    // 2. 検索クエリを構築
    let query = build_sync_query(&addresses, &None);
    assert!(query.contains("from:order@hobbysearch.co.jp"));

    // 3. メッセージのフィルタリング
    use paa_lib::gmail::GmailMessage;

    let msg = GmailMessage {
        message_id: "test123".to_string(),
        snippet: "ご注文ありがとうございます".to_string(),
        subject: Some("【ホビーサーチ】注文確認メール".to_string()),
        body_plain: Some(SAMPLE_HOBBYSEARCH_CONFIRM.to_string()),
        body_html: None,
        internal_date: 1704067200000,
        from_address: Some("order@hobbysearch.co.jp".to_string()),
    };

    let should_save = should_save_message(&msg, &shop_settings);
    assert!(should_save, "Message should be saved");

    // 4. ドメイン抽出
    let domain = extract_domain(&msg.from_address.as_ref().unwrap());
    assert_eq!(domain, Some("hobbysearch.co.jp"));
}

#[test]
fn test_message_filtering_with_various_formats() {
    use paa_lib::gmail::{GmailMessage, ShopSettings};

    let shop_settings = vec![ShopSettings {
        id: 1,
        shop_name: "Test Shop".to_string(),
        sender_address: "shop@example.com".to_string(),
        parser_type: "hobbysearch_confirm".to_string(),
        is_enabled: true,
        subject_filters: Some(r#"["注文", "確認"]"#.to_string()),
        created_at: "2024-01-01".to_string(),
        updated_at: "2024-01-01".to_string(),
    }];

    // 件名が「注文」を含む
    let msg1 = GmailMessage {
        message_id: "1".to_string(),
        snippet: "".to_string(),
        subject: Some("ご注文ありがとうございます".to_string()),
        body_plain: None,
        body_html: None,
        internal_date: 0,
        from_address: Some("shop@example.com".to_string()),
    };
    assert!(should_save_message(&msg1, &shop_settings));

    // 件名が「確認」を含む
    let msg2 = GmailMessage {
        message_id: "2".to_string(),
        snippet: "".to_string(),
        subject: Some("注文確認のお知らせ".to_string()),
        body_plain: None,
        body_html: None,
        internal_date: 0,
        from_address: Some("shop@example.com".to_string()),
    };
    assert!(should_save_message(&msg2, &shop_settings));

    // 件名がフィルターに一致しない
    let msg3 = GmailMessage {
        message_id: "3".to_string(),
        snippet: "".to_string(),
        subject: Some("セールのお知らせ".to_string()),
        body_plain: None,
        body_html: None,
        internal_date: 0,
        from_address: Some("shop@example.com".to_string()),
    };
    assert!(!should_save_message(&msg3, &shop_settings));

    // 送信者が異なる
    let msg4 = GmailMessage {
        message_id: "4".to_string(),
        snippet: "".to_string(),
        subject: Some("ご注文ありがとうございます".to_string()),
        body_plain: None,
        body_html: None,
        internal_date: 0,
        from_address: Some("other@example.com".to_string()),
    };
    assert!(!should_save_message(&msg4, &shop_settings));
}
