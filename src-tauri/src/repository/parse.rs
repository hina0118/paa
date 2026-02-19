use crate::parsers::EmailRow;
use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use sqlx::sqlite::SqlitePool;

/// パース関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait ParseRepository: Send + Sync {
    /// 未パースのメールを取得（order_emails に存在しないメール）
    async fn get_unparsed_emails(&self, batch_size: usize) -> Result<Vec<EmailRow>, String>;

    /// 注文関連テーブルをクリア（order_emails, deliveries, items, orders）
    async fn clear_order_tables(&self) -> Result<(), String>;

    /// パース対象の全メール数を取得
    async fn get_total_email_count(&self) -> Result<i64, String>;
}

/// SQLiteを使用したParseRepositoryの実装
pub struct SqliteParseRepository {
    pool: SqlitePool,
}

impl SqliteParseRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ParseRepository for SqliteParseRepository {
    async fn get_unparsed_emails(&self, batch_size: usize) -> Result<Vec<EmailRow>, String> {
        let emails: Vec<EmailRow> = sqlx::query_as(
            r#"
            SELECT e.id, e.message_id, e.body_plain, e.body_html, e.from_address, e.subject, e.internal_date
            FROM emails e
            LEFT JOIN order_emails oe ON e.id = oe.email_id
            WHERE e.from_address IS NOT NULL
            AND oe.email_id IS NULL
            AND (
                (e.body_plain IS NOT NULL AND LENGTH(TRIM(e.body_plain)) > 0)
                OR (e.body_html IS NOT NULL AND LENGTH(TRIM(e.body_html)) > 0)
            )
            ORDER BY e.internal_date ASC
            LIMIT ?
            "#,
        )
        .bind(batch_size as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch unparsed emails: {e}"))?;

        Ok(emails)
    }

    async fn clear_order_tables(&self) -> Result<(), String> {
        // トランザクション内で全てのDELETE操作を実行してアトミック性を確保
        // 外部キー制約により、order_emails -> deliveries -> items -> orders の順でクリア
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin transaction: {e}"))?;

        sqlx::query("DELETE FROM order_emails")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to clear order_emails table: {e}"))?;

        sqlx::query("DELETE FROM deliveries")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to clear deliveries table: {e}"))?;

        sqlx::query("DELETE FROM items")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to clear items table: {e}"))?;

        sqlx::query("DELETE FROM orders")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to clear orders table: {e}"))?;

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {e}"))?;

        Ok(())
    }

    async fn get_total_email_count(&self) -> Result<i64, String> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM emails
            WHERE from_address IS NOT NULL
            AND (
                (body_plain IS NOT NULL AND LENGTH(TRIM(body_plain)) > 0)
                OR (body_html IS NOT NULL AND LENGTH(TRIM(body_html)) > 0)
            )
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to count emails: {e}"))?;

        Ok(count)
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

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS orders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                shop_domain TEXT,
                shop_name TEXT,
                order_number TEXT,
                order_date DATETIME,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create orders table");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL,
                item_name TEXT NOT NULL,
                item_name_normalized TEXT,
                price INTEGER NOT NULL DEFAULT 0,
                quantity INTEGER NOT NULL DEFAULT 1,
                category TEXT,
                brand TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create items table");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS deliveries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL,
                tracking_number TEXT,
                carrier TEXT,
                delivery_status TEXT NOT NULL DEFAULT 'not_shipped' CHECK(delivery_status IN ('not_shipped', 'preparing', 'shipped', 'in_transit', 'out_for_delivery', 'delivered', 'failed', 'returned', 'cancelled')),
                estimated_delivery DATETIME,
                actual_delivery DATETIME,
                last_checked_at DATETIME,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create deliveries table");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS order_emails (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL,
                email_id INTEGER NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
                FOREIGN KEY (email_id) REFERENCES emails(id) ON DELETE CASCADE,
                UNIQUE (order_id, email_id)
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create order_emails table");

        pool
    }

    #[tokio::test]
    async fn test_parse_repository_get_unparsed_emails_and_clear() {
        let pool = setup_test_db().await;
        let repo = SqliteParseRepository::new(pool.clone());

        // テスト用のメールを追加
        sqlx::query(
            r#"
            INSERT INTO emails (message_id, body_plain, from_address, subject, internal_date)
            VALUES
                ('email1', 'body1', 'test1@example.com', 'Subject 1', 1000),
                ('email2', 'body2', 'test2@example.com', 'Subject 2', 2000),
                ('email3', 'body3', 'test3@example.com', 'Subject 3', 3000)
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to insert test emails");

        // 未パースのメールを取得
        let emails = repo.get_unparsed_emails(10).await.unwrap();
        assert_eq!(emails.len(), 3);

        // 注文を作成してemail1をパース済みにする
        sqlx::query(
            "INSERT INTO orders (order_number, shop_domain) VALUES ('ORD-001', 'example.com')",
        )
        .execute(&pool)
        .await
        .expect("Failed to insert order");

        let order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = 'ORD-001'")
                .fetch_one(&pool)
                .await
                .expect("Failed to get order id");

        let email_id: (i64,) = sqlx::query_as("SELECT id FROM emails WHERE message_id = 'email1'")
            .fetch_one(&pool)
            .await
            .expect("Failed to get email id");

        sqlx::query("INSERT INTO order_emails (order_id, email_id) VALUES (?, ?)")
            .bind(order_id.0)
            .bind(email_id.0)
            .execute(&pool)
            .await
            .expect("Failed to link order to email");

        // 未パースのメールを再取得（email1は除外される）
        let emails = repo.get_unparsed_emails(10).await.unwrap();
        assert_eq!(emails.len(), 2);

        // 全メール数を取得
        let total = repo.get_total_email_count().await.unwrap();
        assert_eq!(total, 3);

        // テーブルをクリア
        repo.clear_order_tables().await.unwrap();

        // クリア後、未パースのメールは再び3件になる
        let emails = repo.get_unparsed_emails(10).await.unwrap();
        assert_eq!(emails.len(), 3);

        // body_html のみのメールも取得対象（HTML メールのフォールバック）
        sqlx::query(
            r#"
            INSERT INTO emails (message_id, body_html, from_address, subject, internal_date)
            VALUES ('email-html-only', '<p>注文番号:99999</p>', 'html@example.com', 'Subject', 4000)
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to insert HTML-only email");

        let emails = repo.get_unparsed_emails(10).await.unwrap();
        assert_eq!(emails.len(), 4, "HTML-only email should be included");
        let html_email = emails
            .iter()
            .find(|e| e.message_id == "email-html-only")
            .unwrap();
        assert!(html_email.body_plain.is_none());
        assert!(html_email
            .body_html
            .as_deref()
            .unwrap()
            .contains("注文番号:99999"));
    }
}
