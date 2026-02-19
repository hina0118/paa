use sqlx::sqlite::SqlitePool;

use crate::metadata;

/// メタデータ（images, shop_settings, product_master）と画像ファイルをZIPにエクスポート
#[tauri::command]
pub async fn export_metadata(
    app: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    save_path: String,
) -> Result<metadata::ExportResult, String> {
    metadata::export_metadata(&app, pool.inner(), std::path::Path::new(&save_path)).await
}

/// ZIPからメタデータをインポート（INSERT OR IGNORE でマージ）
#[tauri::command]
pub async fn import_metadata(
    app: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    zip_path: String,
) -> Result<metadata::ImportResult, String> {
    metadata::import_metadata(&app, pool.inner(), std::path::Path::new(&zip_path)).await
}

/// app_data_dir 直下の復元ポイントZIPから復元する
#[tauri::command]
pub async fn restore_metadata(
    app: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<metadata::ImportResult, String> {
    metadata::restore_metadata(&app, pool.inner()).await
}
