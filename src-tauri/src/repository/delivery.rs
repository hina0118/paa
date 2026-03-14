//! 配送情報リポジトリ

use sqlx::sqlite::SqlitePool;

/// 配送確認処理で使用する配送レコード
#[derive(Debug)]
pub struct PendingDelivery {
    pub id: i64,
    pub tracking_number: String,
    pub carrier: String,
}

pub struct SqliteDeliveryRepository {
    pool: SqlitePool,
}

impl SqliteDeliveryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// tracking_check_logs の終端ステータスを deliveries テーブルに同期する。
    /// stats 等が deliveries.delivery_status を直接参照するため、
    /// スクレイピング前に DB 上で一致させておく。
    pub async fn sync_terminal_statuses(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE deliveries
            SET delivery_status = (
                    SELECT tcl.delivery_status
                    FROM tracking_check_logs tcl
                    WHERE tcl.tracking_number = deliveries.tracking_number
                      AND tcl.delivery_status IN ('delivered', 'cancelled', 'returned')
                ),
                last_checked_at = (
                    SELECT tcl.checked_at
                    FROM tracking_check_logs tcl
                    WHERE tcl.tracking_number = deliveries.tracking_number
                ),
                updated_at = CURRENT_TIMESTAMP
            WHERE EXISTS (
                SELECT 1
                FROM tracking_check_logs tcl
                WHERE tcl.tracking_number = deliveries.tracking_number
                  AND tcl.delivery_status IN ('delivered', 'cancelled', 'returned')
            )
              AND delivery_status NOT IN ('delivered', 'cancelled', 'returned')
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// HTTP スクレイピング対象となる未配達の配送レコードを返す。
    /// tracking_check_logs に終端ステータスが記録済みのものはスキップする。
    pub async fn get_pending_deliveries(&self) -> Result<Vec<PendingDelivery>, sqlx::Error> {
        let rows: Vec<(i64, String, String)> = sqlx::query_as(
            r#"
            SELECT d.id, d.tracking_number, d.carrier
            FROM deliveries d
            LEFT JOIN tracking_check_logs tcl ON d.tracking_number = tcl.tracking_number
            WHERE d.delivery_status NOT IN ('delivered', 'cancelled', 'returned')
              AND d.tracking_number IS NOT NULL
              AND TRIM(d.tracking_number) != ''
              AND d.carrier IS NOT NULL
              AND TRIM(d.carrier) != ''
              AND COALESCE(tcl.delivery_status, '') NOT IN ('delivered', 'cancelled', 'returned')
            ORDER BY d.updated_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, tracking_number, carrier)| PendingDelivery {
                id,
                tracking_number,
                carrier,
            })
            .collect())
    }
}
