use sqlx::sqlite::SqlitePool;

use crate::repository;

#[tauri::command]
pub async fn list_exclusion_patterns(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<repository::ExclusionPattern>, String> {
    let repo = repository::SqliteExclusionPatternRepository::new(pool.inner().clone());
    repo.get_all().await
}

#[tauri::command]
pub async fn add_exclusion_pattern(
    pool: tauri::State<'_, SqlitePool>,
    shop_domain: Option<String>,
    keyword: String,
    match_type: String,
    note: Option<String>,
) -> Result<i64, String> {
    let repo = repository::SqliteExclusionPatternRepository::new(pool.inner().clone());
    repo.add(shop_domain, keyword, match_type, note).await
}

#[tauri::command]
pub async fn delete_exclusion_pattern(
    pool: tauri::State<'_, SqlitePool>,
    id: i64,
) -> Result<(), String> {
    let repo = repository::SqliteExclusionPatternRepository::new(pool.inner().clone());
    repo.delete(id).await
}
