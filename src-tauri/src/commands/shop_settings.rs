use sqlx::sqlite::SqlitePool;

use crate::gmail;
use crate::plugins::{build_registry, ensure_default_settings};
use crate::repository::SqliteShopSettingsRepository;

#[tauri::command]
pub async fn get_all_shop_settings(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<gmail::ShopSettings>, String> {
    gmail::get_all_shop_settings(pool.inner()).await
}

#[tauri::command]
pub async fn create_shop_setting(
    pool: tauri::State<'_, SqlitePool>,
    shop_name: String,
    sender_address: String,
    parser_type: String,
    subject_filters: Option<Vec<String>>,
) -> Result<i64, String> {
    let settings = gmail::CreateShopSettings {
        shop_name,
        sender_address,
        parser_type,
        subject_filters,
    };
    gmail::create_shop_setting(pool.inner(), settings).await
}

#[tauri::command]
pub async fn update_shop_setting(
    pool: tauri::State<'_, SqlitePool>,
    id: i64,
    shop_name: Option<String>,
    sender_address: Option<String>,
    parser_type: Option<String>,
    is_enabled: Option<bool>,
    subject_filters: Option<Vec<String>>,
) -> Result<(), String> {
    let settings = gmail::UpdateShopSettings {
        shop_name,
        sender_address,
        parser_type,
        is_enabled,
        subject_filters,
    };
    gmail::update_shop_setting(pool.inner(), id, settings).await
}

#[tauri::command]
pub async fn delete_shop_setting(
    pool: tauri::State<'_, SqlitePool>,
    id: i64,
) -> Result<(), String> {
    gmail::delete_shop_setting(pool.inner(), id).await
}

#[tauri::command]
pub async fn toggle_shop_enabled(
    pool: tauri::State<'_, SqlitePool>,
    shop_name: String,
    is_enabled: bool,
) -> Result<(), String> {
    gmail::toggle_shop_enabled(pool.inner(), &shop_name, is_enabled).await
}

/// アプリ起動時（フロントエンドの DB init 完了後）に呼び出す。
/// 各プラグインのデフォルト shop_settings を INSERT OR IGNORE で投入する（冪等）。
#[tauri::command]
pub async fn init_default_shop_settings(pool: tauri::State<'_, SqlitePool>) -> Result<(), String> {
    let registry = build_registry();
    let repo = SqliteShopSettingsRepository::new(pool.inner().clone());
    ensure_default_settings(&registry, &repo).await
}
