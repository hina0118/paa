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
    /// page_token に前回の nextPageToken を渡すと次のページを取得
    /// Returns (message_ids, next_page_token)
    async fn list_message_ids(
        &self,
        query: &str,
        max_results: u32,
        page_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), String>;

    /// 単一メッセージを取得
    async fn get_message(&self, message_id: &str) -> Result<GmailMessage, String>;

    /// メッセージのメタデータのみ取得（From, Subject等のヘッダー情報）
    ///
    /// `format("metadata")` を使用して本文(body)を含まない軽量なレスポンスを返す。
    /// 返される `GmailMessage` の `body_plain`, `body_html` は常に `None`。
    /// フィルタリング判定（送信者・件名チェック）に必要な情報のみ取得する。
    async fn get_message_metadata(&self, message_id: &str) -> Result<GmailMessage, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_gmail_client_list_message_ids() {
        let mut mock = MockGmailClientTrait::new();

        mock.expect_list_message_ids()
            .withf(|query, max_results, page_token| {
                query == "from:test@example.com" && *max_results == 10 && page_token.is_none()
            })
            .returning(|_, _, _| Ok((vec!["msg1".to_string(), "msg2".to_string()], None)));

        let result = mock
            .list_message_ids("from:test@example.com", 10, None)
            .await
            .unwrap();
        assert_eq!(result.0.len(), 2);
        assert_eq!(result.0[0], "msg1");
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
            .returning(|_, _, _| Err("API error".to_string()));

        let result = mock.list_message_ids("query", 10, None::<String>).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "API error");
    }
}
