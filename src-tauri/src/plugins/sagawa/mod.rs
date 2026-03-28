//! 佐川急便プラグイン
//!
//! 配達完了通知メール（配達完了通知サービス）をパースし、
//! `tracking_check_logs` と `deliveries` を更新する。

pub mod parsers;

use async_trait::async_trait;

use crate::parsers::EmailParser;

use super::{DefaultShopSetting, DispatchError, DispatchOutcome, PluginRegistration, VendorPlugin};

pub struct SagawaPlugin;

#[async_trait]
impl VendorPlugin for SagawaPlugin {
    fn parser_types(&self) -> &[&str] {
        &["sagawa_delivery_complete"]
    }

    fn priority(&self) -> i32 {
        10
    }

    fn get_parser(&self, _parser_type: &str) -> Option<Box<dyn EmailParser>> {
        // dispatch() 内で直接処理するため EmailParser は返さない
        None
    }

    fn shop_name(&self) -> &str {
        "佐川急便"
    }

    fn default_shop_settings(&self) -> Vec<DefaultShopSetting> {
        vec![DefaultShopSetting {
            shop_name: "佐川急便".to_string(),
            sender_address: "info-nimotsu@sagawa-exp.co.jp".to_string(),
            parser_type: "sagawa_delivery_complete".to_string(),
            // 同送信元から「不在通知」等も届くため、配達完了のみに絞る
            subject_filters: Some(vec!["佐川急便配達完了通知サービス".to_string()]),
        }]
    }

    #[allow(clippy::too_many_arguments)]
    async fn dispatch(
        &self,
        parser_type: &str,
        email_id: i64,
        _from_address: Option<&str>,
        _shop_name: &str,
        _internal_date: Option<i64>,
        body: &str,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    ) -> Result<DispatchOutcome, DispatchError> {
        if parser_type != "sagawa_delivery_complete" {
            return Err(DispatchError::ParseFailed(format!(
                "Unknown parser_type: {parser_type}"
            )));
        }

        // 1. メール本文をパース
        let info = parsers::delivery_complete::parse(body).map_err(DispatchError::ParseFailed)?;

        log::debug!(
            "[sagawa_delivery_complete] email_id={} tracking_number={}",
            email_id,
            info.tracking_number
        );

        // 2. tracking_number で delivery を検索
        let delivery: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM deliveries WHERE tracking_number = ? LIMIT 1")
                .bind(&info.tracking_number)
                .fetch_optional(tx.as_mut())
                .await
                .map_err(|e| DispatchError::SaveFailed(format!("DB error: {e}")))?;

        // 3. tracking_check_logs を更新（メイン操作）
        //    delivery の有無にかかわらず記録する
        sqlx::query(
            r#"
            INSERT INTO tracking_check_logs
                (tracking_number, checked_at, check_status, delivery_status, description)
            VALUES
                (?, CURRENT_TIMESTAMP, 'success', 'delivered', '配達完了メールより確認')
            ON CONFLICT(tracking_number) DO UPDATE SET
                checked_at      = excluded.checked_at,
                check_status    = excluded.check_status,
                delivery_status = excluded.delivery_status,
                description     = excluded.description
            "#,
        )
        .bind(&info.tracking_number)
        .execute(tx.as_mut())
        .await
        .map_err(|e| {
            DispatchError::SaveFailed(format!("Failed to upsert tracking_check_logs: {e}"))
        })?;

        // 4. deliveries が存在すれば更新
        if let Some((delivery_id,)) = delivery {
            sqlx::query(
                r#"
                UPDATE deliveries
                SET delivery_status  = 'delivered',
                    actual_delivery  = ?,
                    last_checked_at  = CURRENT_TIMESTAMP,
                    updated_at       = CURRENT_TIMESTAMP
                WHERE id = ?
                "#,
            )
            .bind(info.delivered_at.as_deref())
            .bind(delivery_id)
            .execute(tx.as_mut())
            .await
            .map_err(|e| DispatchError::SaveFailed(format!("Failed to update deliveries: {e}")))?;

            log::info!(
                "[sagawa_delivery_complete] delivered: tracking_number={} delivery_id={}",
                info.tracking_number,
                delivery_id
            );
        } else {
            log::warn!(
                "[sagawa_delivery_complete] tracking_check_logs に記録済み（deliveries 未登録）: tracking_number={}",
                info.tracking_number
            );
        }

        Ok(DispatchOutcome::DeliveryCompleted {
            tracking_number: info.tracking_number,
        })
    }
}

inventory::submit!(PluginRegistration {
    factory: || Box::new(SagawaPlugin),
});

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn create_pool() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE deliveries (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id        INTEGER NOT NULL,
                tracking_number TEXT,
                carrier         TEXT,
                delivery_status TEXT NOT NULL DEFAULT 'not_shipped',
                estimated_delivery DATETIME,
                actual_delivery DATETIME,
                last_checked_at DATETIME,
                created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE orders (
                id       INTEGER PRIMARY KEY AUTOINCREMENT,
                order_number TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE tracking_check_logs (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                tracking_number TEXT NOT NULL,
                checked_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                check_status    TEXT NOT NULL DEFAULT 'success',
                delivery_status TEXT,
                description     TEXT,
                location        TEXT,
                error_message   TEXT,
                created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (tracking_number)
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    async fn insert_test_delivery(pool: &sqlx::SqlitePool, tracking_number: &str) -> i64 {
        sqlx::query("INSERT INTO orders (order_number) VALUES ('TEST-001')")
            .execute(pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO deliveries (order_id, tracking_number, carrier, delivery_status) VALUES (1, ?, '佐川急便', 'shipped')",
        )
        .bind(tracking_number)
        .execute(pool)
        .await
        .unwrap();

        let (id,): (i64,) = sqlx::query_as("SELECT id FROM deliveries WHERE tracking_number = ?")
            .bind(tracking_number)
            .fetch_one(pool)
            .await
            .unwrap();
        id
    }

    const SAMPLE_BODY: &str = "\
山田太郎 様

◆お問い合わせ送り状No.
470551104391

◆お届け完了日時
2026/03/04（水） 11:18
";

    #[tokio::test]
    async fn test_dispatch_updates_tracking_check_logs() {
        let pool = create_pool().await;
        let delivery_id = insert_test_delivery(&pool, "470551104391").await;
        let _ = delivery_id;

        let plugin = SagawaPlugin;
        let mut tx = pool.begin().await.unwrap();
        let result = plugin
            .dispatch(
                "sagawa_delivery_complete",
                1,
                None,
                "佐川急便",
                None,
                SAMPLE_BODY,
                &mut tx,
            )
            .await;
        tx.commit().await.unwrap();

        assert!(result.is_ok(), "{:?}", result.err());
        assert!(matches!(
            result.unwrap(),
            DispatchOutcome::DeliveryCompleted { .. }
        ));

        let log: (String, String, Option<String>) = sqlx::query_as(
            "SELECT check_status, delivery_status, description FROM tracking_check_logs WHERE tracking_number = ?",
        )
        .bind("470551104391")
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(log.0, "success");
        assert_eq!(log.1, "delivered");
        assert_eq!(log.2.as_deref(), Some("配達完了メールより確認"));
    }

    #[tokio::test]
    async fn test_dispatch_updates_deliveries() {
        let pool = create_pool().await;
        let delivery_id = insert_test_delivery(&pool, "470551104391").await;

        let plugin = SagawaPlugin;
        let mut tx = pool.begin().await.unwrap();
        plugin
            .dispatch(
                "sagawa_delivery_complete",
                1,
                None,
                "佐川急便",
                None,
                SAMPLE_BODY,
                &mut tx,
            )
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let row: (String, Option<String>) =
            sqlx::query_as("SELECT delivery_status, actual_delivery FROM deliveries WHERE id = ?")
                .bind(delivery_id)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(row.0, "delivered");
        assert_eq!(row.1, Some("2026-03-04 11:18:00".to_string()));
    }

    /// deliveries が未登録でも tracking_check_logs に記録して DeliveryCompleted を返す
    #[tokio::test]
    async fn test_dispatch_succeeds_when_no_delivery() {
        let pool = create_pool().await;

        let plugin = SagawaPlugin;
        let mut tx = pool.begin().await.unwrap();
        let result = plugin
            .dispatch(
                "sagawa_delivery_complete",
                1,
                None,
                "佐川急便",
                None,
                SAMPLE_BODY,
                &mut tx,
            )
            .await;
        tx.commit().await.unwrap();

        // delivery 未登録でも成功扱い
        assert!(
            matches!(result, Ok(DispatchOutcome::DeliveryCompleted { .. })),
            "{:?}",
            result.err()
        );

        // tracking_check_logs には記録されている
        let log: (String,) = sqlx::query_as(
            "SELECT delivery_status FROM tracking_check_logs WHERE tracking_number = '470551104391'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(log.0, "delivered");
    }

    #[tokio::test]
    async fn test_dispatch_idempotent() {
        let pool = create_pool().await;
        let _ = insert_test_delivery(&pool, "470551104391").await;

        let plugin = SagawaPlugin;

        for _ in 0..2 {
            let mut tx = pool.begin().await.unwrap();
            plugin
                .dispatch(
                    "sagawa_delivery_complete",
                    1,
                    None,
                    "佐川急便",
                    None,
                    SAMPLE_BODY,
                    &mut tx,
                )
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM tracking_check_logs WHERE tracking_number = '470551104391'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count.0, 1, "tracking_check_logs は1件のみ（冪等）");
    }
}
