//! メタデータのエクスポート/インポート（Issue #40）
//!
//! images, shop_settings, product_master, emails と画像ファイルに加え、
//! item_overrides, order_overrides, excluded_items, excluded_orders を
//! ZIP 形式でバックアップ・復元する。

mod export;
mod file_safety;
pub(crate) mod import;
mod manifest;
mod restore;
mod table_converters;

pub use export::export_metadata;
pub use import::import_metadata;
pub use restore::restore_metadata;
pub use table_converters::{ExportResult, ImportResult};
