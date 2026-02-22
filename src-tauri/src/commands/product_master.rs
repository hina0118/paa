use crate::gemini::ParsedProduct;
use crate::repository::{
    ProductMaster, ProductMasterFilter, ProductMasterRepository, SqliteProductMasterRepository,
};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

const DEFAULT_LIMIT: i64 = 50;

/// `get_product_master_list` のレスポンス
#[derive(Debug, Serialize)]
pub struct ProductMasterListResponse {
    pub items: Vec<ProductMaster>,
    pub total: i64,
}

/// `get_product_master_list` のフィルターパラメーター
#[derive(Debug, Deserialize)]
pub struct ProductMasterFilterParams {
    pub raw_name: Option<String>,
    pub maker: Option<String>,
    pub series: Option<String>,
    pub product_name: Option<String>,
    pub scale: Option<String>,
    pub is_reissue: Option<bool>,
}

impl From<ProductMasterFilterParams> for ProductMasterFilter {
    fn from(p: ProductMasterFilterParams) -> Self {
        Self {
            raw_name: p.raw_name,
            maker: p.maker,
            series: p.series,
            product_name: p.product_name,
            scale: p.scale,
            is_reissue: p.is_reissue,
        }
    }
}

/// 商品マスタ一覧取得（フィルター・ページネーション付き）
#[tauri::command]
pub async fn get_product_master_list(
    pool: tauri::State<'_, SqlitePool>,
    filter: Option<ProductMasterFilterParams>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<ProductMasterListResponse, String> {
    let repo = SqliteProductMasterRepository::new(pool.inner().clone());
    let filter: ProductMasterFilter = filter.map(Into::into).unwrap_or_default();
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    let offset = offset.unwrap_or(0);

    let (items, total) = tokio::try_join!(
        repo.find_filtered(&filter, limit, offset),
        repo.count_filtered(&filter),
    )?;

    Ok(ProductMasterListResponse { items, total })
}

/// 商品マスタ手動更新
#[tauri::command]
pub async fn update_product_master(
    pool: tauri::State<'_, SqlitePool>,
    id: i64,
    maker: Option<String>,
    series: Option<String>,
    product_name: String,
    scale: Option<String>,
    is_reissue: bool,
) -> Result<(), String> {
    let repo = SqliteProductMasterRepository::new(pool.inner().clone());
    let parsed = ParsedProduct {
        maker,
        series,
        name: product_name,
        scale,
        is_reissue,
    };
    repo.update(id, &parsed).await
}
