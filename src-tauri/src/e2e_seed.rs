//! E2E テスト用 DB シード
//!
//! PAA_E2E_MOCK=1 のとき、マイグレーション後にテストデータを投入する。
//! ダッシュボード統計・Tables 画面・Orders 画面が正常に表示されるようにする。
//!
//! 注: tauri-plugin-sql のマイグレーションはフロントエンド接続時に実行される。
//! シードはテーブル存在チェックでエラーならスキップ（初回はフロントエンド接続後に
//! 再度アプリが使われるため、2回目以降のセットアップでシードされる想定）。
//! 確実にシードするには、wdio の beforeSession 待機時間を十分に取る。

use sqlx::SqlitePool;

/// E2E モード時に DB が空の場合にシードデータを投入する
pub async fn seed_if_e2e_and_empty(pool: &SqlitePool) {
    if !crate::e2e_mocks::is_e2e_mock_mode() {
        return;
    }

    // orders テーブルが存在するか確認（マイグレーション未実行ならスキップ）
    let count: Result<(i64,), _> = sqlx::query_as("SELECT COUNT(*) FROM orders")
        .fetch_one(pool)
        .await;

    let count = match count {
        Ok((n,)) => n,
        Err(_) => {
            log::info!("[E2E Seed] Tables not ready yet (migrations may run on first frontend load), skipping seed");
            return;
        }
    };

    if count > 0 {
        log::info!("[E2E Seed] DB already has data, skipping seed");
        return;
    }

    log::info!("[E2E Seed] Seeding test database...");
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));

    if count.0 > 0 {
        log::info!("[E2E Seed] DB already has data, skipping seed");
        return;
    }

    log::info!("[E2E Seed] Seeding test database...");

    // orders
    sqlx::query(
        r#"
        INSERT INTO orders (id, shop_domain, shop_name, order_number, order_date, created_at, updated_at)
        VALUES (1, 'example.com', 'Example Shop', 'ORD-E2E-001', '2024-01-15 12:00:00', '2024-01-15 12:00:00', '2024-01-15 12:00:00')
        "#,
    )
    .execute(pool)
    .await
    .expect("E2E seed: insert orders");

    // items
    sqlx::query(
        r#"
        INSERT INTO items (id, order_id, item_name, item_name_normalized, price, quantity, created_at, updated_at)
        VALUES (1, 1, 'E2Eテスト商品', 'e2eテスト商品', 1500, 1, '2024-01-15 12:00:00', '2024-01-15 12:00:00')
        "#,
    )
    .execute(pool)
    .await
    .expect("E2E seed: insert items");

    // deliveries
    sqlx::query(
        r#"
        INSERT INTO deliveries (id, order_id, tracking_number, carrier, delivery_status, created_at, updated_at)
        VALUES (1, 1, 'TRK-E2E-001', 'yamato', 'delivered', '2024-01-15 12:00:00', '2024-01-15 12:00:00')
        "#,
    )
    .execute(pool)
    .await
    .expect("E2E seed: insert deliveries");

    // emails（ダッシュボードの email stats 用）
    sqlx::query(
        r#"
        INSERT INTO emails (id, message_id, body_plain, analysis_status, created_at, updated_at, from_address, subject)
        VALUES (1, 'msg-e2e-001', 'test body', 'completed', '2024-01-15 12:00:00', '2024-01-15 12:00:00', 'shop@example.com', 'E2E Test')
        "#,
    )
    .execute(pool)
    .await
    .expect("E2E seed: insert emails");

    log::info!("[E2E Seed] Test data seeded successfully");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;

    async fn create_pool() -> SqlitePool {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .create_if_missing(true);
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap()
    }

    async fn create_tables(pool: &SqlitePool) {
        // seed が参照・挿入するカラムのみ用意
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS orders (
              id INTEGER PRIMARY KEY,
              shop_domain TEXT,
              shop_name TEXT,
              order_number TEXT,
              order_date TEXT,
              created_at TEXT,
              updated_at TEXT
            );
            CREATE TABLE IF NOT EXISTS items (
              id INTEGER PRIMARY KEY,
              order_id INTEGER,
              item_name TEXT,
              item_name_normalized TEXT,
              price INTEGER,
              quantity INTEGER,
              created_at TEXT,
              updated_at TEXT
            );
            CREATE TABLE IF NOT EXISTS deliveries (
              id INTEGER PRIMARY KEY,
              order_id INTEGER,
              tracking_number TEXT,
              carrier TEXT,
              delivery_status TEXT,
              created_at TEXT,
              updated_at TEXT
            );
            CREATE TABLE IF NOT EXISTS emails (
              id INTEGER PRIMARY KEY,
              message_id TEXT,
              body_plain TEXT,
              analysis_status TEXT,
              created_at TEXT,
              updated_at TEXT,
              from_address TEXT,
              subject TEXT
            );
            "#,
        )
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn seed_noop_when_not_e2e_mode() {
        std::env::remove_var("PAA_E2E_MOCK");
        let pool = create_pool().await;
        seed_if_e2e_and_empty(&pool).await;
    }

    #[tokio::test]
    #[serial]
    async fn seed_skips_when_tables_not_ready() {
        std::env::set_var("PAA_E2E_MOCK", "1");
        let pool = create_pool().await;

        // テーブル未作成のため、COUNT がエラーになりスキップされる
        seed_if_e2e_and_empty(&pool).await;
    }

    #[tokio::test]
    #[serial]
    async fn seed_inserts_when_e2e_and_orders_empty() {
        std::env::set_var("PAA_E2E_MOCK", "1");
        let pool = create_pool().await;
        create_tables(&pool).await;

        seed_if_e2e_and_empty(&pool).await;

        let (orders,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders")
            .fetch_one(&pool)
            .await
            .unwrap();
        let (items,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items")
            .fetch_one(&pool)
            .await
            .unwrap();
        let (deliveries,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM deliveries")
            .fetch_one(&pool)
            .await
            .unwrap();
        let (emails,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM emails")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(orders, 1);
        assert_eq!(items, 1);
        assert_eq!(deliveries, 1);
        assert_eq!(emails, 1);
    }

    #[tokio::test]
    #[serial]
    async fn seed_skips_when_orders_already_has_data() {
        std::env::set_var("PAA_E2E_MOCK", "1");
        let pool = create_pool().await;
        create_tables(&pool).await;

        sqlx::query("INSERT INTO orders (id) VALUES (1)")
            .execute(&pool)
            .await
            .unwrap();

        seed_if_e2e_and_empty(&pool).await;

        let (orders,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(orders, 1);
    }
}
