use crate::gmail::GmailMessage;
use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

/// メール統計情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailStats {
    pub total_emails: i64,
    pub with_body_plain: i64,
    pub with_body_html: i64,
    pub without_body: i64,
    pub avg_plain_length: f64,
    pub avg_html_length: f64,
}

/// メール関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait EmailRepository: Send + Sync {
    /// メッセージをDBに保存
    ///
    /// # Returns
    /// `(saved, skipped)` - saved: rows_affected>0（INSERT または ON CONFLICT DO UPDATE）、
    /// skipped: rows_affected=0。
    ///
    /// # セマンティクス（ON CONFLICT DO UPDATE 使用時）
    /// 重複（既存 message_id）の場合も UPDATE が実行されるため、saved にカウントされる。
    /// skipped は rows_affected=0 のときのみ（SQLite の changes() は UPDATE でマッチ行数を返すため、通常は 0）。
    /// ログ／メトリクスで saved を「新規のみ」と解釈しないこと。
    async fn save_messages(&self, messages: &[GmailMessage]) -> Result<(usize, usize), String>;

    /// 既存のメッセージIDを取得
    async fn get_existing_message_ids(&self) -> Result<Vec<String>, String>;

    /// 指定されたメッセージIDのうち、emails テーブルに存在しないもののみを返す。
    /// メモリ効率のため、SQL の NOT IN を使用して DB 側でフィルタリングする。
    async fn filter_new_message_ids(&self, message_ids: &[String]) -> Result<Vec<String>, String>;

    /// メッセージ数を取得
    async fn get_message_count(&self) -> Result<i64, String>;

    /// DB内のメールの最新 internal_date（ミリ秒Unix時刻）を取得する。
    /// メールが存在しない場合は None を返す。
    async fn get_latest_internal_date(&self) -> Result<Option<i64>, String>;
}

/// メール統計関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait EmailStatsRepository: Send + Sync {
    /// メール統計情報を取得
    async fn get_email_stats(&self) -> Result<EmailStats, String>;
}

/// SQLiteを使用したEmailRepositoryの実装
pub struct SqliteEmailRepository {
    pool: SqlitePool,
}

impl SqliteEmailRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

/// SQLiteを使用したEmailStatsRepositoryの実装
pub struct SqliteEmailStatsRepository {
    pool: SqlitePool,
}

impl SqliteEmailStatsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EmailStatsRepository for SqliteEmailStatsRepository {
    async fn get_email_stats(&self) -> Result<EmailStats, String> {
        let stats: (i64, i64, i64, i64, Option<f64>, Option<f64>) = sqlx::query_as(
            r#"
            WITH email_lengths AS (
                SELECT
                    body_plain,
                    body_html,
                    CASE WHEN body_plain IS NOT NULL THEN LENGTH(body_plain) ELSE 0 END AS plain_length,
                    CASE WHEN body_html IS NOT NULL THEN LENGTH(body_html) ELSE 0 END AS html_length
                FROM emails
            )
            SELECT
                COUNT(*) AS total,
                COUNT(CASE WHEN body_plain IS NOT NULL AND plain_length > 0 THEN 1 END) AS with_plain,
                COUNT(CASE WHEN body_html IS NOT NULL AND html_length > 0 THEN 1 END) AS with_html,
                COUNT(CASE WHEN (body_plain IS NULL OR plain_length = 0) AND (body_html IS NULL OR html_length = 0) THEN 1 END) AS without_body,
                AVG(CASE WHEN body_plain IS NOT NULL AND plain_length > 0 THEN plain_length END) AS avg_plain,
                AVG(CASE WHEN body_html IS NOT NULL AND html_length > 0 THEN html_length END) AS avg_html
            FROM email_lengths
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch email stats: {e}"))?;

        Ok(EmailStats {
            total_emails: stats.0,
            with_body_plain: stats.1,
            with_body_html: stats.2,
            without_body: stats.3,
            avg_plain_length: stats.4.unwrap_or(0.0),
            avg_html_length: stats.5.unwrap_or(0.0),
        })
    }
}

#[async_trait]
impl EmailRepository for SqliteEmailRepository {
    async fn save_messages(&self, messages: &[GmailMessage]) -> Result<(usize, usize), String> {
        let mut saved = 0;
        let mut skipped = 0;

        // rows_affected の解釈: SQLite の changes() は「UPDATE でマッチした行数」ではなく
        // 「直近のステートメントによって実際に変更された行数」を返す。
        // このクエリでは ON CONFLICT DO UPDATE が発生した場合、既存行に対して UPDATE が実行され、
        // COALESCE により既存値がそのまま再代入されるケースでも、その行は更新されたものとしてカウントされるため
        // rows_affected は 1 となる想定である。
        // saved は「新規のみ」ではなく「INSERT/UPDATE が行われた件数」。FetchResult/SyncProgressEvent の
        // saved_count/newly_saved に伝播する。
        // 参考: https://www.sqlite.org/c3ref/changes.html

        // トランザクションを使用してバッチ処理
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin transaction: {e}"))?;

        for message in messages {
            // ON CONFLICT で既存の場合は body を補完（初回同期時に body_html 等が取れなかった場合の再取得で更新）
            let result = sqlx::query(
                r#"
                INSERT INTO emails (message_id, body_plain, body_html, internal_date, from_address, subject)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(message_id) DO UPDATE SET
                    body_plain = COALESCE(excluded.body_plain, body_plain),
                    body_html = COALESCE(excluded.body_html, body_html),
                    internal_date = COALESCE(excluded.internal_date, internal_date),
                    from_address = COALESCE(excluded.from_address, from_address),
                    subject = COALESCE(excluded.subject, subject)
                "#,
            )
            .bind(&message.message_id)
            .bind(&message.body_plain)
            .bind(&message.body_html)
            .bind(message.internal_date)
            .bind(&message.from_address)
            .bind(&message.subject)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert message {}: {}", message.message_id, e))?;

            if result.rows_affected() > 0 {
                saved += 1;
            } else {
                skipped += 1;
            }
        }

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {e}"))?;

        Ok((saved, skipped))
    }

    async fn get_existing_message_ids(&self) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT message_id FROM emails")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("Failed to get existing message IDs: {e}"))?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    async fn filter_new_message_ids(&self, message_ids: &[String]) -> Result<Vec<String>, String> {
        if message_ids.is_empty() {
            return Ok(Vec::new());
        }

        // トランザクション内で実行（TEMP テーブルは接続ごとのため、プールでは同一接続を保証する必要がある）
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin transaction: {e}"))?;

        // 一時テーブルを作成
        sqlx::query(
            "CREATE TEMP TABLE IF NOT EXISTS temp_filter_ids (message_id TEXT PRIMARY KEY)",
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to create temp table: {e}"))?;

        // SQLite の SQLITE_MAX_VARIABLE_NUMBER (999) を考慮してチャンク処理
        const CHUNK_SIZE: usize = 900;
        let mut new_ids = Vec::new();

        for chunk in message_ids.chunks(CHUNK_SIZE) {
            sqlx::query("DELETE FROM temp_filter_ids")
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Failed to clear temp table: {e}"))?;

            // チャンクを一時テーブルに INSERT
            let mut q = sqlx::QueryBuilder::new("INSERT INTO temp_filter_ids (message_id) ");
            q.push_values(chunk, |mut b, id| {
                b.push_bind(id);
            });
            q.build()
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Failed to insert into temp table: {e}"))?;

            // NOT IN でフィルタリング
            let rows: Vec<(String,)> = sqlx::query_as(
                "SELECT message_id FROM temp_filter_ids WHERE message_id NOT IN (SELECT message_id FROM emails)",
            )
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to filter new IDs: {e}"))?;

            new_ids.extend(rows.into_iter().map(|(id,)| id));
        }

        sqlx::query("DROP TABLE IF EXISTS temp_filter_ids")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to drop temp table: {e}"))?;

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {e}"))?;

        Ok(new_ids)
    }

    async fn get_message_count(&self) -> Result<i64, String> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM emails")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| format!("Failed to get message count: {e}"))?;

        Ok(count.0)
    }

    async fn get_latest_internal_date(&self) -> Result<Option<i64>, String> {
        let row: (Option<i64>,) =
            sqlx::query_as("SELECT MAX(internal_date) FROM emails WHERE internal_date > 0")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| format!("Failed to get latest internal_date: {e}"))?;

        Ok(row.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS emails (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id TEXT UNIQUE NOT NULL,
                body_plain TEXT,
                body_html TEXT,
                analysis_status TEXT NOT NULL DEFAULT 'pending' CHECK(analysis_status IN ('pending', 'completed')),
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                internal_date INTEGER,
                from_address TEXT,
                subject TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create emails table");

        pool
    }

    #[tokio::test]
    async fn test_email_repository_save_and_get() {
        let pool = setup_test_db().await;
        let repo = SqliteEmailRepository::new(pool);

        let messages = vec![
            GmailMessage {
                message_id: "test123".to_string(),
                snippet: "Test snippet".to_string(),
                subject: Some("Test subject".to_string()),
                body_plain: Some("Plain body".to_string()),
                body_html: Some("<p>HTML body</p>".to_string()),
                internal_date: 1704067200000,
                from_address: Some("test@example.com".to_string()),
            },
            GmailMessage {
                message_id: "test456".to_string(),
                snippet: "Another snippet".to_string(),
                subject: Some("Another subject".to_string()),
                body_plain: None,
                body_html: None,
                internal_date: 1704153600000,
                from_address: None,
            },
        ];

        // 保存
        let (saved, skipped) = repo.save_messages(&messages).await.unwrap();
        assert_eq!(saved, 2);
        assert_eq!(skipped, 0);

        // 重複保存: ON CONFLICT DO UPDATE により UPDATE が実行され、saved としてカウントされる
        let (saved, skipped) = repo.save_messages(&messages).await.unwrap();
        assert_eq!(saved, 2);
        assert_eq!(skipped, 0);

        // カウント確認
        let count = repo.get_message_count().await.unwrap();
        assert_eq!(count, 2);

        // 既存ID取得
        let ids = repo.get_existing_message_ids().await.unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"test123".to_string()));
        assert!(ids.contains(&"test456".to_string()));

        // filter_new_message_ids: 既存IDは除外され、新規IDのみ返る
        let input_ids = vec![
            "test123".to_string(),
            "test456".to_string(),
            "new_id_1".to_string(),
            "new_id_2".to_string(),
        ];
        let new_ids = repo.filter_new_message_ids(&input_ids).await.unwrap();
        assert_eq!(new_ids.len(), 2);
        assert!(new_ids.contains(&"new_id_1".to_string()));
        assert!(new_ids.contains(&"new_id_2".to_string()));
        assert!(!new_ids.contains(&"test123".to_string()));
        assert!(!new_ids.contains(&"test456".to_string()));

        // 空配列の場合は空を返す
        let empty = repo.filter_new_message_ids(&[]).await.unwrap();
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn test_filter_new_message_ids_chunk_boundaries() {
        // CHUNK_SIZE (900) を超える件数、境界値、全既存、全新規のエッジケース
        let pool = setup_test_db().await;
        let repo = SqliteEmailRepository::new(pool);

        // 既存メッセージを3件保存
        let existing = vec![
            GmailMessage {
                message_id: "existing_1".to_string(),
                snippet: "".to_string(),
                subject: None,
                body_plain: None,
                body_html: None,
                internal_date: 0,
                from_address: None,
            },
            GmailMessage {
                message_id: "existing_2".to_string(),
                snippet: "".to_string(),
                subject: None,
                body_plain: None,
                body_html: None,
                internal_date: 0,
                from_address: None,
            },
            GmailMessage {
                message_id: "existing_3".to_string(),
                snippet: "".to_string(),
                subject: None,
                body_plain: None,
                body_html: None,
                internal_date: 0,
                from_address: None,
            },
        ];
        repo.save_messages(&existing).await.unwrap();

        // 全既存ID → 空を返す
        let all_existing = vec![
            "existing_1".to_string(),
            "existing_2".to_string(),
            "existing_3".to_string(),
        ];
        let result = repo.filter_new_message_ids(&all_existing).await.unwrap();
        assert!(result.is_empty(), "all existing should return empty");

        // 全新規ID → 全て返す
        let all_new: Vec<String> = (0..5).map(|i| format!("new_only_{}", i)).collect();
        let result = repo.filter_new_message_ids(&all_new).await.unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(result, all_new);

        // CHUNK_SIZE ちょうど (900件): 既存3 + 新規897
        let mut ids_900: Vec<String> = vec![
            "existing_1".into(),
            "existing_2".into(),
            "existing_3".into(),
        ];
        ids_900.extend((0..897).map(|i| format!("chunk_900_{}", i)));
        let result = repo.filter_new_message_ids(&ids_900).await.unwrap();
        assert_eq!(result.len(), 897);

        // CHUNK_SIZE 超え (1000件): 既存3 + 新規997
        let mut ids_1000: Vec<String> = vec![
            "existing_1".into(),
            "existing_2".into(),
            "existing_3".into(),
        ];
        ids_1000.extend((0..997).map(|i| format!("chunk_1000_{}", i)));
        let result = repo.filter_new_message_ids(&ids_1000).await.unwrap();
        assert_eq!(result.len(), 997);

        // 2000件超: 既存3 + 新規2000
        let mut ids_2000: Vec<String> = vec![
            "existing_1".into(),
            "existing_2".into(),
            "existing_3".into(),
        ];
        ids_2000.extend((0..2000).map(|i| format!("chunk_2000_{}", i)));
        let result = repo.filter_new_message_ids(&ids_2000).await.unwrap();
        assert_eq!(result.len(), 2000);
    }

    #[tokio::test]
    async fn test_get_latest_internal_date_empty_db() {
        let pool = setup_test_db().await;
        let repo = SqliteEmailRepository::new(pool);

        let result = repo.get_latest_internal_date().await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_get_latest_internal_date_returns_max() {
        let pool = setup_test_db().await;
        let repo = SqliteEmailRepository::new(pool);

        let messages = vec![
            GmailMessage {
                message_id: "old".to_string(),
                snippet: "".to_string(),
                subject: None,
                body_plain: None,
                body_html: None,
                internal_date: 1704067200000, // 2024-01-01
                from_address: None,
            },
            GmailMessage {
                message_id: "new".to_string(),
                snippet: "".to_string(),
                subject: None,
                body_plain: None,
                body_html: None,
                internal_date: 1704153600000, // 2024-01-02
                from_address: None,
            },
        ];
        repo.save_messages(&messages).await.unwrap();

        let result = repo.get_latest_internal_date().await.unwrap();
        assert_eq!(result, Some(1704153600000));
    }

    #[tokio::test]
    async fn test_get_latest_internal_date_all_zero() {
        let pool = setup_test_db().await;
        let repo = SqliteEmailRepository::new(pool);

        let messages = vec![GmailMessage {
            message_id: "zero_date".to_string(),
            snippet: "".to_string(),
            subject: None,
            body_plain: None,
            body_html: None,
            internal_date: 0,
            from_address: None,
        }];
        repo.save_messages(&messages).await.unwrap();

        // internal_date = 0 のみの場合は None を返す（フル同期へフォールバック）
        let result = repo.get_latest_internal_date().await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_email_stats_repository_get_stats() {
        let pool = setup_test_db().await;
        let repo = SqliteEmailStatsRepository::new(pool.clone());

        // 空の状態での統計
        let stats = repo.get_email_stats().await.unwrap();
        assert_eq!(stats.total_emails, 0);
        assert_eq!(stats.with_body_plain, 0);
        assert_eq!(stats.with_body_html, 0);
        assert_eq!(stats.without_body, 0);
        assert_eq!(stats.avg_plain_length, 0.0);
        assert_eq!(stats.avg_html_length, 0.0);

        // テストデータを追加
        sqlx::query(
            r#"
            INSERT INTO emails (message_id, body_plain, body_html, from_address, subject)
            VALUES
                ('msg1', 'Plain text body', NULL, 'test1@example.com', 'Subject 1'),
                ('msg2', NULL, '<p>HTML body</p>', 'test2@example.com', 'Subject 2'),
                ('msg3', 'Another plain', '<p>Another HTML</p>', 'test3@example.com', 'Subject 3'),
                ('msg4', NULL, NULL, 'test4@example.com', 'Subject 4')
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to insert test emails");

        // 統計を再取得
        let stats = repo.get_email_stats().await.unwrap();
        assert_eq!(stats.total_emails, 4);
        assert_eq!(stats.with_body_plain, 2); // msg1, msg3
        assert_eq!(stats.with_body_html, 2); // msg2, msg3
        assert_eq!(stats.without_body, 1); // msg4
        assert!(stats.avg_plain_length > 0.0);
        assert!(stats.avg_html_length > 0.0);
    }
}
