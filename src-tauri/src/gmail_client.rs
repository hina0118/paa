//! Gmail API クライアントのトレイト定義とモック対応
//!
//! このモジュールは Gmail API 操作を抽象化し、テスト時にモック可能にします。

use crate::gmail::{GmailMessage, ShopSettings};
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
    /// 指定されたクエリに基づいてメッセージを取得
    async fn fetch_messages(&self, query: &str) -> Result<Vec<GmailMessage>, String>;

    /// バッチでメッセージIDリストからメッセージを取得
    async fn fetch_batch(
        &self,
        message_ids: Vec<String>,
        batch_size: usize,
    ) -> Result<Vec<GmailMessage>, String>;

    /// 単一メッセージを取得
    async fn get_message(&self, message_id: &str) -> Result<GmailMessage, String>;

    /// ショップ設定に基づいて検索クエリを構築
    /// before_date を Option<String> に変更してライフタイム問題を回避
    fn build_query(&self, shop_settings: &[ShopSettings], before_date: Option<String>) -> String;
}
