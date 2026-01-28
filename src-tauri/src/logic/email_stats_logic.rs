//! メール統計関連のビジネスロジック
//!
//! Tauriコマンドからメール統計取得ロジックを分離します。

use crate::repository::EmailStatsRepository;

/// メール統計情報を取得する
pub async fn get_email_stats<R>(repo: &R) -> Result<EmailStats, String>
where
    R: EmailStatsRepository,
{
    repo.get_email_stats().await
}

/// メール統計情報の構造体（lib.rsから移動）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailStats {
    pub total_emails: i64,
    pub with_body_plain: i64,
    pub with_body_html: i64,
    pub without_body: i64,
    pub avg_plain_length: f64,
    pub avg_html_length: f64,
}

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::MockEmailStatsRepository;

    #[tokio::test]
    async fn test_get_email_stats_delegates_to_repo() {
        let mut mock = MockEmailStatsRepository::new();
        let expected = EmailStats {
            total_emails: 100,
            with_body_plain: 80,
            with_body_html: 70,
            without_body: 20,
            avg_plain_length: 123.4,
            avg_html_length: 567.8,
        };

        mock.expect_get_email_stats()
            .returning(move || Ok(expected.clone()));

        let stats = get_email_stats(&mock).await.unwrap();
        assert_eq!(stats.total_emails, 100);
        assert_eq!(stats.with_body_plain, 80);
        assert_eq!(stats.with_body_html, 70);
        assert_eq!(stats.without_body, 20);
        assert!((stats.avg_plain_length - 123.4).abs() < 1e-6);
        assert!((stats.avg_html_length - 567.8).abs() < 1e-6);
    }
}
