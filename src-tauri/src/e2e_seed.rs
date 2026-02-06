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
