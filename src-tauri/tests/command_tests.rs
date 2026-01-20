use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;

// テスト用のデータベースプールを作成
async fn create_test_pool() -> sqlx::SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .unwrap();

    // sync_metadataテーブルを作成
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS sync_metadata (
            id INTEGER PRIMARY KEY,
            sync_status TEXT NOT NULL DEFAULT 'idle',
            oldest_fetched_date TEXT,
            total_synced_count INTEGER NOT NULL DEFAULT 0,
            batch_size INTEGER NOT NULL DEFAULT 50,
            last_sync_started_at TEXT,
            last_sync_completed_at TEXT,
            last_error_message TEXT
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    // 初期データを挿入
    sqlx::query(
        r"
        INSERT OR REPLACE INTO sync_metadata
        (id, sync_status, total_synced_count, batch_size)
        VALUES (1, 'idle', 0, 50)
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

#[cfg(test)]
mod command_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_sync_status_success() {
        let pool = create_test_pool().await;

        // sync_statusを取得
        let row: (String, Option<String>, i64, i64, Option<String>, Option<String>) = sqlx::query_as(
            "SELECT sync_status, oldest_fetched_date, total_synced_count, batch_size, last_sync_started_at, last_sync_completed_at FROM sync_metadata WHERE id = 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "idle");
        assert_eq!(row.1, None);
        assert_eq!(row.2, 0);
        assert_eq!(row.3, 50);
        assert_eq!(row.4, None);
        assert_eq!(row.5, None);
    }

    #[tokio::test]
    async fn test_get_sync_status_with_data() {
        let pool = create_test_pool().await;

        // データを更新
        sqlx::query(
            r"
            UPDATE sync_metadata
            SET sync_status = 'syncing',
                oldest_fetched_date = '2024-01-01',
                total_synced_count = 100,
                last_sync_started_at = '2024-01-15T10:00:00Z'
            WHERE id = 1
            ",
        )
        .execute(&pool)
        .await
        .unwrap();

        // sync_statusを取得
        let row: (String, Option<String>, i64, i64, Option<String>, Option<String>) = sqlx::query_as(
            "SELECT sync_status, oldest_fetched_date, total_synced_count, batch_size, last_sync_started_at, last_sync_completed_at FROM sync_metadata WHERE id = 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "syncing");
        assert_eq!(row.1, Some("2024-01-01".to_string()));
        assert_eq!(row.2, 100);
        assert_eq!(row.3, 50);
        assert_eq!(row.4, Some("2024-01-15T10:00:00Z".to_string()));
        assert_eq!(row.5, None);
    }

    #[tokio::test]
    async fn test_reset_sync_status_when_syncing() {
        let pool = create_test_pool().await;

        // sync_statusを'syncing'に設定
        sqlx::query("UPDATE sync_metadata SET sync_status = 'syncing' WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();

        // reset_sync_statusの処理を実行
        sqlx::query(
            "UPDATE sync_metadata
             SET sync_status = 'idle'
             WHERE id = 1 AND sync_status = 'syncing'"
        )
        .execute(&pool)
        .await
        .unwrap();

        // 検証
        let status: (String,) = sqlx::query_as(
            "SELECT sync_status FROM sync_metadata WHERE id = 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(status.0, "idle");
    }

    #[tokio::test]
    async fn test_reset_sync_status_when_idle() {
        let pool = create_test_pool().await;

        // sync_statusは既に'idle'
        let status_before: (String,) = sqlx::query_as(
            "SELECT sync_status FROM sync_metadata WHERE id = 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(status_before.0, "idle");

        // reset_sync_statusの処理を実行（条件に合わないので更新されない）
        let result = sqlx::query(
            "UPDATE sync_metadata
             SET sync_status = 'idle'
             WHERE id = 1 AND sync_status = 'syncing'"
        )
        .execute(&pool)
        .await
        .unwrap();

        // 影響を受けた行数は0
        assert_eq!(result.rows_affected(), 0);

        // statusは変わらず'idle'
        let status_after: (String,) = sqlx::query_as(
            "SELECT sync_status FROM sync_metadata WHERE id = 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(status_after.0, "idle");
    }

    #[tokio::test]
    async fn test_update_batch_size_success() {
        let pool = create_test_pool().await;

        // batch_sizeを更新
        let new_batch_size = 100i64;
        sqlx::query("UPDATE sync_metadata SET batch_size = ?1 WHERE id = 1")
            .bind(new_batch_size)
            .execute(&pool)
            .await
            .unwrap();

        // 検証
        let batch_size: (i64,) = sqlx::query_as(
            "SELECT batch_size FROM sync_metadata WHERE id = 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(batch_size.0, 100);
    }

    #[tokio::test]
    async fn test_update_batch_size_zero() {
        let pool = create_test_pool().await;

        // batch_sizeを0に更新（境界値テスト）
        let new_batch_size = 0i64;
        sqlx::query("UPDATE sync_metadata SET batch_size = ?1 WHERE id = 1")
            .bind(new_batch_size)
            .execute(&pool)
            .await
            .unwrap();

        // 検証
        let batch_size: (i64,) = sqlx::query_as(
            "SELECT batch_size FROM sync_metadata WHERE id = 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(batch_size.0, 0);
    }

    #[tokio::test]
    async fn test_update_batch_size_large_value() {
        let pool = create_test_pool().await;

        // 大きなbatch_sizeを設定（境界値テスト）
        let new_batch_size = 1000i64;
        sqlx::query("UPDATE sync_metadata SET batch_size = ?1 WHERE id = 1")
            .bind(new_batch_size)
            .execute(&pool)
            .await
            .unwrap();

        // 検証
        let batch_size: (i64,) = sqlx::query_as(
            "SELECT batch_size FROM sync_metadata WHERE id = 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(batch_size.0, 1000);
    }

    #[tokio::test]
    async fn test_get_sync_status_nonexistent_record() {
        let pool = create_test_pool().await;

        // id=1のレコードを削除
        sqlx::query("DELETE FROM sync_metadata WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();

        // 存在しないレコードを取得しようとする
        let result: Result<(String, Option<String>, i64, i64, Option<String>, Option<String>), _> = sqlx::query_as(
            "SELECT sync_status, oldest_fetched_date, total_synced_count, batch_size, last_sync_started_at, last_sync_completed_at FROM sync_metadata WHERE id = 1"
        )
        .fetch_one(&pool)
        .await;

        // エラーが返されることを確認
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sync_status_transitions() {
        let pool = create_test_pool().await;

        // idle -> syncing
        sqlx::query("UPDATE sync_metadata SET sync_status = 'syncing', last_sync_started_at = '2024-01-15T10:00:00Z' WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();

        let status: (String,) = sqlx::query_as("SELECT sync_status FROM sync_metadata WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status.0, "syncing");

        // syncing -> complete
        sqlx::query("UPDATE sync_metadata SET sync_status = 'complete', last_sync_completed_at = '2024-01-15T10:05:00Z' WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();

        let status: (String,) = sqlx::query_as("SELECT sync_status FROM sync_metadata WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status.0, "complete");

        // complete -> idle (reset)
        sqlx::query("UPDATE sync_metadata SET sync_status = 'idle' WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();

        let status: (String,) = sqlx::query_as("SELECT sync_status FROM sync_metadata WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status.0, "idle");
    }

    #[tokio::test]
    async fn test_sync_error_handling() {
        let pool = create_test_pool().await;

        // エラー状態を設定
        let error_message = "Test error: API rate limit exceeded";
        sqlx::query(
            "UPDATE sync_metadata SET sync_status = 'error', last_error_message = ?1 WHERE id = 1"
        )
        .bind(error_message)
        .execute(&pool)
        .await
        .unwrap();

        // 検証
        let result: (String, Option<String>) = sqlx::query_as(
            "SELECT sync_status, last_error_message FROM sync_metadata WHERE id = 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(result.0, "error");
        assert_eq!(result.1, Some(error_message.to_string()));
    }

    #[tokio::test]
    async fn test_batch_size_boundary_values() {
        let pool = create_test_pool().await;

        // 境界値テスト: 1
        sqlx::query("UPDATE sync_metadata SET batch_size = 1 WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();

        let size: (i64,) = sqlx::query_as("SELECT batch_size FROM sync_metadata WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(size.0, 1);

        // 境界値テスト: 500
        sqlx::query("UPDATE sync_metadata SET batch_size = 500 WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();

        let size: (i64,) = sqlx::query_as("SELECT batch_size FROM sync_metadata WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(size.0, 500);
    }

    #[tokio::test]
    async fn test_oldest_fetched_date_update() {
        let pool = create_test_pool().await;

        let date = "2024-01-15T10:00:00Z";
        sqlx::query("UPDATE sync_metadata SET oldest_fetched_date = ?1 WHERE id = 1")
            .bind(date)
            .execute(&pool)
            .await
            .unwrap();

        let result: (Option<String>,) = sqlx::query_as(
            "SELECT oldest_fetched_date FROM sync_metadata WHERE id = 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(result.0, Some(date.to_string()));
    }

    #[tokio::test]
    async fn test_total_synced_count_increment() {
        let pool = create_test_pool().await;

        // 初期値は0
        let initial: (i64,) = sqlx::query_as(
            "SELECT total_synced_count FROM sync_metadata WHERE id = 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(initial.0, 0);

        // インクリメント
        sqlx::query("UPDATE sync_metadata SET total_synced_count = total_synced_count + 50 WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();

        let after_first: (i64,) = sqlx::query_as(
            "SELECT total_synced_count FROM sync_metadata WHERE id = 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(after_first.0, 50);

        // さらにインクリメント
        sqlx::query("UPDATE sync_metadata SET total_synced_count = total_synced_count + 100 WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();

        let after_second: (i64,) = sqlx::query_as(
            "SELECT total_synced_count FROM sync_metadata WHERE id = 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(after_second.0, 150);
    }

    #[tokio::test]
    async fn test_sync_timestamps() {
        let pool = create_test_pool().await;

        let started_at = "2024-01-15T10:00:00Z";
        let completed_at = "2024-01-15T10:30:00Z";

        sqlx::query(
            "UPDATE sync_metadata
             SET last_sync_started_at = ?1, last_sync_completed_at = ?2
             WHERE id = 1"
        )
        .bind(started_at)
        .bind(completed_at)
        .execute(&pool)
        .await
        .unwrap();

        let result: (Option<String>, Option<String>) = sqlx::query_as(
            "SELECT last_sync_started_at, last_sync_completed_at FROM sync_metadata WHERE id = 1"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(result.0, Some(started_at.to_string()));
        assert_eq!(result.1, Some(completed_at.to_string()));
    }

    #[tokio::test]
    async fn test_reset_sync_status_multiple_times() {
        let pool = create_test_pool().await;

        // 1回目: syncing -> idle
        sqlx::query("UPDATE sync_metadata SET sync_status = 'syncing' WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "UPDATE sync_metadata SET sync_status = 'idle' WHERE id = 1 AND sync_status = 'syncing'"
        )
        .execute(&pool)
        .await
        .unwrap();

        let status: (String,) = sqlx::query_as("SELECT sync_status FROM sync_metadata WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status.0, "idle");

        // 2回目: syncing -> idle
        sqlx::query("UPDATE sync_metadata SET sync_status = 'syncing' WHERE id = 1")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "UPDATE sync_metadata SET sync_status = 'idle' WHERE id = 1 AND sync_status = 'syncing'"
        )
        .execute(&pool)
        .await
        .unwrap();

        let status: (String,) = sqlx::query_as("SELECT sync_status FROM sync_metadata WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status.0, "idle");
    }
}
