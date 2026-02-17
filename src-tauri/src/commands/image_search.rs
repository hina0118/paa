use sqlx::sqlite::SqlitePool;
use tauri::Manager;

use crate::e2e_mocks::{is_e2e_mock_mode, E2EMockImageSearchClient};
use crate::google_search;
use crate::image_utils;

/// 商品画像を検索（SerpApi）
#[tauri::command]
pub async fn search_product_images(
    app_handle: tauri::AppHandle,
    query: String,
    num_results: Option<u32>,
) -> Result<Vec<google_search::ImageSearchResult>, String> {
    use google_search::ImageSearchClientTrait;

    let num = num_results.unwrap_or(10);

    // E2Eモック時は外部APIを呼ばない
    if is_e2e_mock_mode() {
        log::info!("Using E2E mock image search");
        let client = E2EMockImageSearchClient;
        return client.search_images(&query, num).await;
    }

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    if !google_search::is_configured(&app_data_dir) {
        return Err(
            "SerpApiが設定されていません。設定画面でAPIキーを設定してください。".to_string(),
        );
    }

    let api_key = google_search::load_api_key(&app_data_dir)?;

    let client = google_search::SerpApiClient::new(api_key)?;
    client.search_images(&query, num).await
}

/// 画像URLから画像をダウンロードしてimagesテーブルに保存
#[tauri::command]
pub async fn save_image_from_url(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    item_id: i64,
    image_url: String,
) -> Result<String, String> {
    log::info!("Downloading image for item_id: {}", item_id);

    let item_name_normalized: Option<String> =
        sqlx::query_scalar("SELECT item_name_normalized FROM items WHERE id = ?")
            .bind(item_id)
            .fetch_optional(pool.inner())
            .await
            .map_err(|e| format!("Failed to get item_name_normalized: {e}"))?
            .flatten();

    let normalized = item_name_normalized.as_ref().ok_or_else(|| {
        "この商品は正規化できないため画像を登録できません。商品名に記号のみなどが含まれている可能性があります。".to_string()
    })?;

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    let images_dir = app_data_dir.join("images");

    image_utils::save_image_from_url_for_item(
        pool.inner(),
        &images_dir,
        normalized,
        &image_url,
        false, // UI手動保存: 既存があれば上書き
    )
    .await
}
