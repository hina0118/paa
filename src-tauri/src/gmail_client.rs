//! Gmail API クライアントのトレイト定義とモック対応
//!
//! このモジュールは Gmail API 操作を抽象化し、テスト時にモック可能にします。

use crate::gmail::GmailMessage;
use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;

/// Gmail API の操作を抽象化するトレイト
///
/// このトレイトを実装することで、本番環境では実際の Gmail API を使用し、
/// テスト環境ではモックを使用してテストできます。
#[cfg_attr(test, automock)]
#[async_trait]
pub trait GmailClientTrait: Send + Sync {
    /// 指定されたクエリに基づいてメッセージIDリストを取得
    async fn list_message_ids(&self, query: &str, max_results: u32) -> Result<Vec<String>, String>;

    /// 単一メッセージを取得
    async fn get_message(&self, message_id: &str) -> Result<GmailMessage, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_gmail_client_list_message_ids() {
        let mut mock = MockGmailClientTrait::new();

        mock.expect_list_message_ids()
            .withf(|query, max_results| query == "from:test@example.com" && *max_results == 10)
            .returning(|_, _| Ok(vec!["msg1".to_string(), "msg2".to_string()]));

        let result = mock
            .list_message_ids("from:test@example.com", 10)
            .await
            .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "msg1");
    }

    #[tokio::test]
    async fn test_mock_gmail_client_get_message() {
        let mut mock = MockGmailClientTrait::new();

        mock.expect_get_message()
            .withf(|id| id == "msg123")
            .returning(|_| {
                Ok(GmailMessage {
                    message_id: "msg123".to_string(),
                    snippet: "Test snippet".to_string(),
                    subject: Some("Test subject".to_string()),
                    body_plain: Some("Test body".to_string()),
                    body_html: None,
                    internal_date: 1704067200000,
                    from_address: Some("sender@example.com".to_string()),
                })
            });

        let msg = mock.get_message("msg123").await.unwrap();
        assert_eq!(msg.message_id, "msg123");
        assert_eq!(msg.subject, Some("Test subject".to_string()));
    }

    #[tokio::test]
    async fn test_mock_gmail_client_error() {
        let mut mock = MockGmailClientTrait::new();

        mock.expect_list_message_ids()
            .returning(|_, _| Err("API error".to_string()));

        let result = mock.list_message_ids("query", 10).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "API error");
    }
}
