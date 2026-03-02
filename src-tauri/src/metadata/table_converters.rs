//! テーブル行の型定義（エクスポート用タプル型・インポート用デシリアライズ構造体）

use serde::{Deserialize, Serialize};

/// shop_settings テーブル行 (id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at)
pub(super) type ShopSettingsRow = (
    i64,
    String,
    String,
    String,
    i32,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// product_master テーブル行 (id, raw_name, normalized_name, maker, series, product_name, scale, is_reissue, platform_hint, created_at, updated_at)
pub(super) type ProductMasterRow = (
    i64,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    i32,
    Option<String>,
    String,
    String,
);

/// emails テーブル行 (id, message_id, body_plain, body_html, analysis_status, created_at, updated_at, internal_date, from_address, subject)
pub(super) type EmailRow = (
    i64,
    String,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<String>,
    Option<String>,
);

/// item_overrides テーブル行
/// (id, shop_domain, order_number, original_item_name, original_brand, item_name, price, quantity, brand, category, created_at, updated_at)
pub(super) type ItemOverrideRow = (
    i64,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<i64>,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// order_overrides テーブル行
/// (id, shop_domain, order_number, new_order_number, order_date, shop_name, created_at, updated_at)
pub(super) type OrderOverrideRow = (
    i64,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// excluded_items テーブル行
/// (id, shop_domain, order_number, item_name, brand, reason, created_at)
pub(super) type ExcludedItemRow = (
    i64,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
);

/// excluded_orders テーブル行
/// (id, shop_domain, order_number, reason, created_at)
pub(super) type ExcludedOrderRow = (i64, String, String, Option<String>, Option<String>);

/// tracking_check_logs テーブル行
/// (id, tracking_number, checked_at, check_status, delivery_status, description, location, error_message, created_at)
pub(super) type TrackingCheckLogRow = (
    i64,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
);

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportResult {
    pub images_count: usize,
    pub shop_settings_count: usize,
    pub product_master_count: usize,
    pub emails_count: usize,
    pub item_overrides_count: usize,
    pub order_overrides_count: usize,
    pub excluded_items_count: usize,
    pub excluded_orders_count: usize,
    pub tracking_check_logs_count: usize,
    pub image_files_count: usize,
    /// スキップした画像数（不正な file_name、サイズ超過、ファイル不存在）
    pub images_skipped: usize,
    /// app_data_dir 直下に復元ポイントZIPを保存できたか
    pub restore_point_saved: bool,
    /// 復元ポイントZIPのパス（保存先）
    pub restore_point_path: Option<String>,
    /// 復元ポイントZIP保存に失敗した場合のエラー
    pub restore_point_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub images_inserted: usize,
    pub shop_settings_inserted: usize,
    pub product_master_inserted: usize,
    pub emails_inserted: usize,
    pub item_overrides_inserted: usize,
    pub order_overrides_inserted: usize,
    pub excluded_items_inserted: usize,
    pub excluded_orders_inserted: usize,
    pub tracking_check_logs_inserted: usize,
    pub image_files_copied: usize,
    /// app_data_dir 直下の復元ポイントZIPを更新できたか（インポート時）
    /// Some(true): 更新成功, Some(false): 更新失敗, None: 更新不要（restore_metadata）
    pub restore_point_updated: Option<bool>,
    /// 復元ポイントZIPのパス（保存先）
    pub restore_point_path: Option<String>,
    /// 復元ポイントZIP更新に失敗した場合のエラー
    pub restore_point_error: Option<String>,
}

/// JSON デシリアライズ用（タプル形式、id を含むがインポート時は未使用）
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(super) struct JsonImageRow(
    pub(super) i64,            // id (未使用)
    pub(super) String,         // item_name_normalized
    pub(super) Option<String>, // file_name
    pub(super) Option<String>, // created_at
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(super) struct JsonShopSettingsRow(
    pub(super) i64,            // id (未使用)
    pub(super) String,         // shop_name
    pub(super) String,         // sender_address
    pub(super) String,         // parser_type
    pub(super) i32,            // is_enabled
    pub(super) Option<String>, // subject_filters
    pub(super) Option<String>, // created_at (未使用)
    pub(super) Option<String>, // updated_at (未使用)
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(super) struct JsonProductMasterRow(
    pub(super) i64,            // id (未使用)
    pub(super) String,         // raw_name
    pub(super) String,         // normalized_name
    pub(super) Option<String>, // maker
    pub(super) Option<String>, // series
    pub(super) Option<String>, // product_name
    pub(super) Option<String>, // scale
    pub(super) i32,            // is_reissue
    pub(super) Option<String>, // platform_hint
    pub(super) Option<String>, // created_at (未使用)
    pub(super) Option<String>, // updated_at (未使用)
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(super) struct JsonEmailRow(
    pub(super) i64,            // id (未使用)
    pub(super) String,         // message_id
    pub(super) Option<String>, // body_plain
    pub(super) Option<String>, // body_html
    pub(super) String,         // analysis_status
    pub(super) Option<String>, // created_at
    pub(super) Option<String>, // updated_at
    pub(super) Option<i64>,    // internal_date
    pub(super) Option<String>, // from_address
    pub(super) Option<String>, // subject
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(super) struct JsonItemOverrideRow(
    pub(super) i64,            // id (未使用)
    pub(super) String,         // shop_domain
    pub(super) String,         // order_number
    pub(super) String,         // original_item_name
    pub(super) String,         // original_brand
    pub(super) Option<String>, // item_name
    pub(super) Option<i64>,    // price
    pub(super) Option<i64>,    // quantity
    pub(super) Option<String>, // brand
    pub(super) Option<String>, // category
    pub(super) Option<String>, // created_at (未使用)
    pub(super) Option<String>, // updated_at (未使用)
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(super) struct JsonOrderOverrideRow(
    pub(super) i64,            // id (未使用)
    pub(super) String,         // shop_domain
    pub(super) String,         // order_number
    pub(super) Option<String>, // new_order_number
    pub(super) Option<String>, // order_date
    pub(super) Option<String>, // shop_name
    pub(super) Option<String>, // created_at (未使用)
    pub(super) Option<String>, // updated_at (未使用)
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(super) struct JsonExcludedItemRow(
    pub(super) i64,            // id (未使用)
    pub(super) String,         // shop_domain
    pub(super) String,         // order_number
    pub(super) String,         // item_name
    pub(super) String,         // brand
    pub(super) Option<String>, // reason
    pub(super) Option<String>, // created_at (未使用)
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(super) struct JsonExcludedOrderRow(
    pub(super) i64,            // id (未使用)
    pub(super) String,         // shop_domain
    pub(super) String,         // order_number
    pub(super) Option<String>, // reason
    pub(super) Option<String>, // created_at (未使用)
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(super) struct JsonTrackingCheckLogRow(
    pub(super) i64,            // id (未使用)
    pub(super) String,         // tracking_number
    pub(super) String,         // checked_at
    pub(super) String,         // check_status
    pub(super) Option<String>, // delivery_status
    pub(super) Option<String>, // description
    pub(super) Option<String>, // location
    pub(super) Option<String>, // error_message
    pub(super) Option<String>, // created_at (インポート時に COALESCE(?, CURRENT_TIMESTAMP) で使用)
);
