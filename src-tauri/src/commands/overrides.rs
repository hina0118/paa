use sqlx::sqlite::SqlitePool;

use crate::repository;

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn save_item_override(
    pool: tauri::State<'_, SqlitePool>,
    shop_domain: String,
    order_number: String,
    original_item_name: String,
    original_brand: String,
    item_name: Option<String>,
    price: Option<i64>,
    quantity: Option<i64>,
    brand: Option<String>,
    category: Option<String>,
) -> Result<i64, String> {
    let repo = repository::SqliteOverrideRepository::new(pool.inner().clone());
    repo.save_item_override(repository::SaveItemOverride {
        shop_domain,
        order_number,
        original_item_name,
        original_brand,
        item_name,
        price,
        quantity,
        brand,
        category,
    })
    .await
}

#[tauri::command]
pub async fn save_order_override(
    pool: tauri::State<'_, SqlitePool>,
    shop_domain: String,
    order_number: String,
    new_order_number: Option<String>,
    order_date: Option<String>,
    shop_name: Option<String>,
) -> Result<i64, String> {
    let repo = repository::SqliteOverrideRepository::new(pool.inner().clone());
    repo.save_order_override(repository::SaveOrderOverride {
        shop_domain,
        order_number,
        new_order_number,
        order_date,
        shop_name,
    })
    .await
}

#[tauri::command]
pub async fn delete_item_override(pool: tauri::State<'_, SqlitePool>, id: i64) -> Result<(), String> {
    let repo = repository::SqliteOverrideRepository::new(pool.inner().clone());
    repo.delete_item_override(id).await
}

#[tauri::command]
pub async fn delete_item_override_by_key(
    pool: tauri::State<'_, SqlitePool>,
    shop_domain: String,
    order_number: String,
    original_item_name: String,
    original_brand: String,
) -> Result<(), String> {
    let repo = repository::SqliteOverrideRepository::new(pool.inner().clone());
    repo.delete_item_override_by_key(
        &shop_domain,
        &order_number,
        &original_item_name,
        &original_brand,
    )
    .await
}

#[tauri::command]
pub async fn delete_order_override(pool: tauri::State<'_, SqlitePool>, id: i64) -> Result<(), String> {
    let repo = repository::SqliteOverrideRepository::new(pool.inner().clone());
    repo.delete_order_override(id).await
}

#[tauri::command]
pub async fn delete_order_override_by_key(
    pool: tauri::State<'_, SqlitePool>,
    shop_domain: String,
    order_number: String,
) -> Result<(), String> {
    let repo = repository::SqliteOverrideRepository::new(pool.inner().clone());
    repo.delete_order_override_by_key(&shop_domain, &order_number)
        .await
}

#[tauri::command]
pub async fn exclude_item(
    pool: tauri::State<'_, SqlitePool>,
    shop_domain: String,
    order_number: String,
    item_name: String,
    brand: String,
    reason: Option<String>,
) -> Result<i64, String> {
    let repo = repository::SqliteOverrideRepository::new(pool.inner().clone());
    repo.exclude_item(repository::ExcludeItemParams {
        shop_domain,
        order_number,
        item_name,
        brand,
        reason,
    })
    .await
}

#[tauri::command]
pub async fn exclude_order(
    pool: tauri::State<'_, SqlitePool>,
    shop_domain: String,
    order_number: String,
    reason: Option<String>,
) -> Result<i64, String> {
    let repo = repository::SqliteOverrideRepository::new(pool.inner().clone());
    repo.exclude_order(repository::ExcludeOrderParams {
        shop_domain,
        order_number,
        reason,
    })
    .await
}

#[tauri::command]
pub async fn restore_excluded_item(pool: tauri::State<'_, SqlitePool>, id: i64) -> Result<(), String> {
    let repo = repository::SqliteOverrideRepository::new(pool.inner().clone());
    repo.restore_excluded_item(id).await
}

#[tauri::command]
pub async fn restore_excluded_order(pool: tauri::State<'_, SqlitePool>, id: i64) -> Result<(), String> {
    let repo = repository::SqliteOverrideRepository::new(pool.inner().clone());
    repo.restore_excluded_order(id).await
}

#[tauri::command]
pub async fn get_all_excluded_items(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<repository::ExcludedItem>, String> {
    let repo = repository::SqliteOverrideRepository::new(pool.inner().clone());
    repo.get_all_excluded_items().await
}

#[tauri::command]
pub async fn get_all_excluded_orders(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<repository::ExcludedOrder>, String> {
    let repo = repository::SqliteOverrideRepository::new(pool.inner().clone());
    repo.get_all_excluded_orders().await
}
