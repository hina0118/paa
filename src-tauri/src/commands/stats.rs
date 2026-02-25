use sqlx::sqlite::SqlitePool;

use crate::repository::{
    DeliveryStats, DeliveryStatsRepository, EmailStats, EmailStatsRepository, MiscStats,
    MiscStatsRepository, OrderStats, OrderStatsRepository, ProductMasterStats,
    ProductMasterStatsRepository, SqliteDeliveryStatsRepository, SqliteEmailStatsRepository,
    SqliteMiscStatsRepository, SqliteOrderStatsRepository, SqliteProductMasterStatsRepository,
};

/// E2E モード時に DB シードを実行。フロントエンドのマウント後に呼ぶ（マイグレーション完了後）
#[tauri::command]
pub async fn seed_e2e_db(pool: tauri::State<'_, SqlitePool>) -> Result<(), String> {
    crate::e2e_seed::seed_if_e2e_and_empty(pool.inner()).await;
    Ok(())
}

/// DB ファイル名を返す。E2E モード時は paa_e2e.db（開発用と分離）、通常時は paa_data.db
#[tauri::command]
pub fn get_db_filename() -> &'static str {
    if crate::e2e_mocks::is_e2e_mock_mode() {
        "paa_e2e.db"
    } else {
        "paa_data.db"
    }
}

/// メール統計情報を取得
#[tauri::command]
pub async fn get_email_stats(pool: tauri::State<'_, SqlitePool>) -> Result<EmailStats, String> {
    let repo = SqliteEmailStatsRepository::new(pool.inner().clone());
    repo.get_email_stats().await
}

/// 注文・商品サマリを取得
#[tauri::command]
pub async fn get_order_stats(pool: tauri::State<'_, SqlitePool>) -> Result<OrderStats, String> {
    let repo = SqliteOrderStatsRepository::new(pool.inner().clone());
    repo.get_order_stats().await
}

/// 配送状況サマリを取得
#[tauri::command]
pub async fn get_delivery_stats(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<DeliveryStats, String> {
    let repo = SqliteDeliveryStatsRepository::new(pool.inner().clone());
    repo.get_delivery_stats().await
}

/// 商品名解析進捗を取得
#[tauri::command]
pub async fn get_product_master_stats(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<ProductMasterStats, String> {
    let repo = SqliteProductMasterStatsRepository::new(pool.inner().clone());
    repo.get_product_master_stats().await
}

/// 店舗設定・画像サマリを取得
#[tauri::command]
pub async fn get_misc_stats(pool: tauri::State<'_, SqlitePool>) -> Result<MiscStats, String> {
    let repo = SqliteMiscStatsRepository::new(pool.inner().clone());
    repo.get_misc_stats().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_get_db_filename_switches_by_e2e_env() {
        let prev = std::env::var("PAA_E2E_MOCK").ok();

        std::env::remove_var("PAA_E2E_MOCK");
        assert_eq!(get_db_filename(), "paa_data.db");

        std::env::set_var("PAA_E2E_MOCK", "0");
        assert_eq!(get_db_filename(), "paa_data.db");

        std::env::set_var("PAA_E2E_MOCK", "1");
        assert_eq!(get_db_filename(), "paa_e2e.db");

        // restore
        match prev {
            Some(v) => std::env::set_var("PAA_E2E_MOCK", v),
            None => std::env::remove_var("PAA_E2E_MOCK"),
        }
    }
}
