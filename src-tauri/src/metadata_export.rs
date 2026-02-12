//! メタデータのエクスポート/インポート（Issue #40）
//!
//! images, shop_settings, product_master, emails と画像ファイルに加え、
//! item_overrides, order_overrides, excluded_items, excluded_orders を
//! ZIP 形式でバックアップ・復元する。

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Seek, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use zip::write::FileOptions;
use zip::ZipArchive;

const MANIFEST_VERSION: u32 = 1;
const RESTORE_POINT_FILE_NAME: &str = "paa_restore_point.zip";

/// 画像ファイル1件あたりの最大サイズ（バイト）。巨大エントリによるメモリ消費を防ぐ。
const MAX_IMAGE_ENTRY_SIZE: u64 = 10 * 1024 * 1024; // 10MB

/// JSON エントリ1件あたりの最大サイズ（バイト）。巨大 ZIP による DoS を防ぐ。
const MAX_JSON_ENTRY_SIZE: u64 = 10 * 1024 * 1024; // 10MB

/// NDJSON 1行あたりの最大サイズ。メール本文（最大1MB級）を含むため余裕を持たせる。
const MAX_NDJSON_LINE_SIZE: usize = 2 * 1024 * 1024; // 2MB

/// レガシー emails.json の最大サイズ。本文を含むため 10MB を超えやすいので緩和。
const MAX_EMAILS_JSON_ENTRY_SIZE: u64 = 50 * 1024 * 1024; // 50MB

/// emails.ndjson 全体の最大サイズ。OOM 対策。
const MAX_EMAILS_NDJSON_ENTRY_SIZE: u64 = 100 * 1024 * 1024; // 100MB

/// file_name が安全な単一ファイル名か検証（パストラバーサル対策）
fn is_safe_file_name(file_name: &str) -> bool {
    !file_name.is_empty()
        && !file_name.contains('/')
        && !file_name.contains('\\')
        && !file_name.contains("..")
        && Path::new(file_name)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s == file_name)
            .unwrap_or(false)
}

/// shop_settings テーブル行 (id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at)
type ShopSettingsRow = (
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
type ProductMasterRow = (
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
type EmailRow = (
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
type ItemOverrideRow = (
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
type OrderOverrideRow = (
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
type ExcludedItemRow = (
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
type ExcludedOrderRow = (i64, String, String, Option<String>, Option<String>);

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
    pub image_files_copied: usize,
    /// app_data_dir 直下の復元ポイントZIPを更新できたか（インポート時）
    pub restore_point_updated: bool,
    /// 復元ポイントZIPのパス（保存先）
    pub restore_point_path: Option<String>,
    /// 復元ポイントZIP更新に失敗した場合のエラー
    pub restore_point_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    exported_at: String,
}

fn get_restore_point_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    Ok(app_data_dir.join(RESTORE_POINT_FILE_NAME))
}

fn copy_restore_point_zip(src_zip_path: &Path, restore_point_path: &Path) -> (bool, Option<String>) {
    if let Some(parent) = restore_point_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return (
                false,
                Some(format!(
                    "Failed to create restore point directory {}: {e}",
                    parent.display()
                )),
            );
        }
    }

    match fs::copy(src_zip_path, restore_point_path) {
        Ok(_) => (true, None),
        Err(e) => (
            false,
            Some(format!(
                "Failed to save restore point zip to {}: {e}",
                restore_point_path.display()
            )),
        ),
    }
}

/// メタデータをZIPにエクスポート
pub async fn export_metadata(
    app: &AppHandle,
    pool: &SqlitePool,
    save_path: &Path,
) -> Result<ExportResult, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    let images_dir = app_data_dir.join("images");
    let file = File::create(save_path).map_err(|e| format!("Failed to create file: {e}"))?;
    let mut result = export_metadata_to_writer(pool, &images_dir, file).await?;

    let restore_point_path = get_restore_point_path(app)?;
    let (saved, err) = copy_restore_point_zip(save_path, &restore_point_path);
    result.restore_point_saved = saved;
    result.restore_point_path = Some(restore_point_path.display().to_string());
    result.restore_point_error = err;

    Ok(result)
}

/// エクスポート処理本体（テスト可能）。writer に ZIP を書き込む。
pub(crate) async fn export_metadata_to_writer<W>(
    pool: &SqlitePool,
    images_dir: &Path,
    writer: W,
) -> Result<ExportResult, String>
where
    W: Write + Seek,
{
    // 1. テーブルデータを取得
    let images_rows: Vec<(i64, String, Option<String>, String)> =
        sqlx::query_as("SELECT id, item_name_normalized, file_name, created_at FROM images")
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Failed to fetch images: {e}"))?;

    let shop_settings_rows: Vec<ShopSettingsRow> = sqlx::query_as(
        r#"
        SELECT id, shop_name, sender_address, parser_type, is_enabled,
               subject_filters, created_at, updated_at FROM shop_settings
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch shop_settings: {e}"))?;

    let product_master_rows: Vec<ProductMasterRow> = sqlx::query_as(
        r#"
        SELECT id, raw_name, normalized_name, maker, series, product_name, scale,
               is_reissue, platform_hint, created_at, updated_at FROM product_master
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch product_master: {e}"))?;

    let item_overrides_rows: Vec<ItemOverrideRow> = sqlx::query_as(
        r#"
        SELECT id, shop_domain, order_number, original_item_name, original_brand,
               item_name, price, quantity, brand, category, created_at, updated_at
        FROM item_overrides
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch item_overrides: {e}"))?;

    let order_overrides_rows: Vec<OrderOverrideRow> = sqlx::query_as(
        r#"
        SELECT id, shop_domain, order_number, new_order_number, order_date, shop_name,
               created_at, updated_at
        FROM order_overrides
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch order_overrides: {e}"))?;

    let excluded_items_rows: Vec<ExcludedItemRow> = sqlx::query_as(
        r#"
        SELECT id, shop_domain, order_number, item_name, brand, reason, created_at
        FROM excluded_items
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch excluded_items: {e}"))?;

    let excluded_orders_rows: Vec<ExcludedOrderRow> = sqlx::query_as(
        r#"
        SELECT id, shop_domain, order_number, reason, created_at
        FROM excluded_orders
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch excluded_orders: {e}"))?;

    // 2. JSON にシリアライズ（emails は後でストリーミング出力するため除外）
    let images_json = serde_json::to_string_pretty(&images_rows)
        .map_err(|e| format!("Failed to serialize images: {e}"))?;
    let shop_settings_json = serde_json::to_string_pretty(&shop_settings_rows)
        .map_err(|e| format!("Failed to serialize shop_settings: {e}"))?;
    let product_master_json = serde_json::to_string_pretty(&product_master_rows)
        .map_err(|e| format!("Failed to serialize product_master: {e}"))?;
    let item_overrides_json = serde_json::to_string_pretty(&item_overrides_rows)
        .map_err(|e| format!("Failed to serialize item_overrides: {e}"))?;
    let order_overrides_json = serde_json::to_string_pretty(&order_overrides_rows)
        .map_err(|e| format!("Failed to serialize order_overrides: {e}"))?;
    let excluded_items_json = serde_json::to_string_pretty(&excluded_items_rows)
        .map_err(|e| format!("Failed to serialize excluded_items: {e}"))?;
    let excluded_orders_json = serde_json::to_string_pretty(&excluded_orders_rows)
        .map_err(|e| format!("Failed to serialize excluded_orders: {e}"))?;

    let manifest = Manifest {
        version: MANIFEST_VERSION,
        exported_at: chrono::Utc::now()
            .with_timezone(&chrono_tz::Asia::Tokyo)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {e}"))?;

    // 3. ZIP に書き込み
    let mut zip_writer = zip::ZipWriter::new(writer);
    let options: zip::write::FileOptions<()> = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    zip_writer
        .start_file("manifest.json", options)
        .map_err(|e| format!("Failed to add manifest: {e}"))?;
    zip_writer
        .write_all(manifest_json.as_bytes())
        .map_err(|e| format!("Failed to write manifest: {e}"))?;

    zip_writer
        .start_file("images.json", options)
        .map_err(|e| format!("Failed to add images.json: {e}"))?;
    zip_writer
        .write_all(images_json.as_bytes())
        .map_err(|e| format!("Failed to write images: {e}"))?;

    zip_writer
        .start_file("shop_settings.json", options)
        .map_err(|e| format!("Failed to add shop_settings.json: {e}"))?;
    zip_writer
        .write_all(shop_settings_json.as_bytes())
        .map_err(|e| format!("Failed to write shop_settings: {e}"))?;

    zip_writer
        .start_file("product_master.json", options)
        .map_err(|e| format!("Failed to add product_master.json: {e}"))?;
    zip_writer
        .write_all(product_master_json.as_bytes())
        .map_err(|e| format!("Failed to write product_master: {e}"))?;

    zip_writer
        .start_file("item_overrides.json", options)
        .map_err(|e| format!("Failed to add item_overrides.json: {e}"))?;
    zip_writer
        .write_all(item_overrides_json.as_bytes())
        .map_err(|e| format!("Failed to write item_overrides: {e}"))?;

    zip_writer
        .start_file("order_overrides.json", options)
        .map_err(|e| format!("Failed to add order_overrides.json: {e}"))?;
    zip_writer
        .write_all(order_overrides_json.as_bytes())
        .map_err(|e| format!("Failed to write order_overrides: {e}"))?;

    zip_writer
        .start_file("excluded_items.json", options)
        .map_err(|e| format!("Failed to add excluded_items.json: {e}"))?;
    zip_writer
        .write_all(excluded_items_json.as_bytes())
        .map_err(|e| format!("Failed to write excluded_items: {e}"))?;

    zip_writer
        .start_file("excluded_orders.json", options)
        .map_err(|e| format!("Failed to add excluded_orders.json: {e}"))?;
    zip_writer
        .write_all(excluded_orders_json.as_bytes())
        .map_err(|e| format!("Failed to write excluded_orders: {e}"))?;

    // emails: ストリーミングで NDJSON 出力（OOM 回避）
    zip_writer
        .start_file("emails.ndjson", options)
        .map_err(|e| format!("Failed to add emails.ndjson: {e}"))?;
    let mut emails_count = 0usize;
    {
        let mut stream = sqlx::query_as::<_, EmailRow>(
            r#"
            SELECT id, message_id, body_plain, body_html, analysis_status,
                   created_at, updated_at, internal_date, from_address, subject FROM emails
            "#,
        )
        .fetch(pool);
        while let Some(row) = stream.next().await {
            let row = row.map_err(|e| format!("Failed to fetch emails: {e}"))?;
            let line = serde_json::to_string(&row)
                .map_err(|e| format!("Failed to serialize email: {e}"))?;
            if line.len() > MAX_NDJSON_LINE_SIZE {
                return Err(format!(
                    "Email row exceeds line size limit (max {} bytes)",
                    MAX_NDJSON_LINE_SIZE
                ));
            }
            zip_writer
                .write_all(line.as_bytes())
                .map_err(|e| format!("Failed to write email: {e}"))?;
            zip_writer
                .write_all(b"\n")
                .map_err(|e| format!("Failed to write newline: {e}"))?;
            emails_count += 1;
        }
    }

    let mut image_files_count = 0usize;
    let mut images_skipped = 0usize;
    for (_, _norm, file_name_opt, _) in &images_rows {
        if let Some(ref file_name) = file_name_opt {
            if !is_safe_file_name(file_name) {
                images_skipped += 1; // パストラバーサル対策: 不正な file_name はスキップ
                continue;
            }
            let src = images_dir.join(file_name);
            if !src.exists() {
                images_skipped += 1;
                continue;
            }
            let metadata = fs::metadata(&src).ok();
            if metadata
                .map(|m| m.len() > MAX_IMAGE_ENTRY_SIZE)
                .unwrap_or(true)
            {
                images_skipped += 1; // サイズ不明 or 超過はスキップ
                continue;
            }
            let data =
                fs::read(&src).map_err(|e| format!("Failed to read image {}: {e}", file_name))?;
            let zip_path = format!("images/{}", file_name);
            zip_writer
                .start_file(&zip_path, options)
                .map_err(|e| format!("Failed to add image {}: {e}", file_name))?;
            zip_writer
                .write_all(&data)
                .map_err(|e| format!("Failed to write image {}: {e}", file_name))?;
            image_files_count += 1;
        }
    }

    zip_writer
        .finish()
        .map_err(|e| format!("Failed to finish zip: {e}"))?;

    Ok(ExportResult {
        images_count: images_rows.len(),
        shop_settings_count: shop_settings_rows.len(),
        product_master_count: product_master_rows.len(),
        emails_count,
        item_overrides_count: item_overrides_rows.len(),
        order_overrides_count: order_overrides_rows.len(),
        excluded_items_count: excluded_items_rows.len(),
        excluded_orders_count: excluded_orders_rows.len(),
        image_files_count,
        images_skipped,
        restore_point_saved: false,
        restore_point_path: None,
        restore_point_error: None,
    })
}

/// ZIP からメタデータをインポート（INSERT OR IGNORE でマージ）
pub async fn import_metadata(
    app: &AppHandle,
    pool: &SqlitePool,
    zip_path: &Path,
) -> Result<ImportResult, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    let images_dir = app_data_dir.join("images");
    fs::create_dir_all(&images_dir).map_err(|e| format!("Failed to create images dir: {e}"))?;
    let file = File::open(zip_path).map_err(|e| format!("Failed to open zip: {e}"))?;
    let mut result = import_metadata_from_reader(pool, &images_dir, file).await?;

    let restore_point_path = get_restore_point_path(app)?;
    let (updated, err) = copy_restore_point_zip(zip_path, &restore_point_path);
    result.restore_point_updated = updated;
    result.restore_point_path = Some(restore_point_path.display().to_string());
    result.restore_point_error = err;

    Ok(result)
}

/// app_data_dir 直下に保存してある復元ポイントZIPから復元する
pub async fn restore_metadata(app: &AppHandle, pool: &SqlitePool) -> Result<ImportResult, String> {
    let restore_point_path = get_restore_point_path(app)?;
    let _metadata = match fs::metadata(&restore_point_path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(
                "復元ポイントが存在しません。先に「データのバックアップ」または「データのインポート」を実行してください。"
                    .to_string(),
            );
        }
        Err(e) => {
            return Err(format!("復元ポイントにアクセスできません: {e}"));
        }
    };

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    let images_dir = app_data_dir.join("images");
    fs::create_dir_all(&images_dir).map_err(|e| format!("Failed to create images dir: {e}"))?;

    let file = File::open(&restore_point_path)
        .map_err(|e| format!("Failed to open restore point zip: {e}"))?;
    let mut result = import_metadata_from_reader(pool, &images_dir, file).await?;

    // restore コマンドでは復元ポイント自体は更新しない（読み取り専用）
    result.restore_point_updated = false;
    result.restore_point_path = Some(restore_point_path.display().to_string());
    result.restore_point_error = None;

    Ok(result)
}

/// インポート処理本体（テスト可能）。reader から ZIP を読み込む。
pub(crate) async fn import_metadata_from_reader<R>(
    pool: &SqlitePool,
    images_dir: &Path,
    reader: R,
) -> Result<ImportResult, String>
where
    R: Read + Seek,
{
    fs::create_dir_all(images_dir).map_err(|e| format!("Failed to create images dir: {e}"))?;
    let mut zip_archive =
        ZipArchive::new(reader).map_err(|e| format!("Failed to read zip: {e}"))?;

    // manifest.json の読み取りとバージョン検証
    let manifest_json = read_zip_entry(&mut zip_archive, "manifest.json")?;
    let manifest: Manifest = serde_json::from_str(&manifest_json)
        .map_err(|e| format!("Failed to parse manifest.json: {e}"))?;
    if manifest.version != MANIFEST_VERSION {
        return Err(format!(
            "Unsupported backup version: expected {}, got {}",
            MANIFEST_VERSION, manifest.version
        ));
    }

    // images.json
    let images_json = read_zip_entry(&mut zip_archive, "images.json")?;
    let images_rows: Vec<JsonImageRow> = serde_json::from_str(&images_json)
        .map_err(|e| format!("Failed to parse images.json: {e}"))?;

    // shop_settings.json
    let shop_settings_json = read_zip_entry(&mut zip_archive, "shop_settings.json")?;
    let shop_settings_rows: Vec<JsonShopSettingsRow> = serde_json::from_str(&shop_settings_json)
        .map_err(|e| format!("Failed to parse shop_settings.json: {e}"))?;

    // product_master.json
    let product_master_json = read_zip_entry(&mut zip_archive, "product_master.json")?;
    let product_master_rows: Vec<JsonProductMasterRow> = serde_json::from_str(&product_master_json)
        .map_err(|e| format!("Failed to parse product_master.json: {e}"))?;

    // item_overrides.json / order_overrides.json / excluded_items.json / excluded_orders.json
    // 旧バックアップ互換: ファイルが無ければスキップする
    let item_overrides_rows: Vec<JsonItemOverrideRow> =
        if zip_archive.file_names().any(|n| n == "item_overrides.json") {
            let json = read_zip_entry(&mut zip_archive, "item_overrides.json")?;
            serde_json::from_str(&json)
                .map_err(|e| format!("Failed to parse item_overrides.json: {e}"))?
        } else {
            Vec::new()
        };
    let order_overrides_rows: Vec<JsonOrderOverrideRow> = if zip_archive
        .file_names()
        .any(|n| n == "order_overrides.json")
    {
        let json = read_zip_entry(&mut zip_archive, "order_overrides.json")?;
        serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse order_overrides.json: {e}"))?
    } else {
        Vec::new()
    };
    let excluded_items_rows: Vec<JsonExcludedItemRow> =
        if zip_archive.file_names().any(|n| n == "excluded_items.json") {
            let json = read_zip_entry(&mut zip_archive, "excluded_items.json")?;
            serde_json::from_str(&json)
                .map_err(|e| format!("Failed to parse excluded_items.json: {e}"))?
        } else {
            Vec::new()
        };
    let excluded_orders_rows: Vec<JsonExcludedOrderRow> = if zip_archive
        .file_names()
        .any(|n| n == "excluded_orders.json")
    {
        let json = read_zip_entry(&mut zip_archive, "excluded_orders.json")?;
        serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse excluded_orders.json: {e}"))?
    } else {
        Vec::new()
    };

    // images.json に登場する安全な file_name のみをコピー対象とする（DoS 対策）
    let allowed_image_files: HashSet<String> = images_rows
        .iter()
        .filter_map(|r| r.2.as_ref())
        .filter(|s| is_safe_file_name(s))
        .cloned()
        .collect();

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {e}"))?;

    let mut images_inserted = 0usize;
    for row in &images_rows {
        // パストラバーサル対策: 不正な file_name は None にして DB に保存しない
        let file_name_for_db = row
            .2
            .as_ref()
            .filter(|s| is_safe_file_name(s))
            .map(|s| s.as_str());
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO images (item_name_normalized, file_name, created_at)
            VALUES (?, ?, COALESCE(?, CURRENT_TIMESTAMP))
            "#,
        )
        .bind(&row.1)
        .bind(file_name_for_db)
        .bind(&row.3)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert image: {e}"))?;
        if result.rows_affected() > 0 {
            images_inserted += 1;
        }
    }

    let mut shop_settings_inserted = 0usize;
    for row in &shop_settings_rows {
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO shop_settings (shop_name, sender_address, parser_type, is_enabled, subject_filters)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&row.1)
        .bind(&row.2)
        .bind(&row.3)
        .bind(row.4)
        .bind(&row.5)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert shop_setting: {e}"))?;
        if result.rows_affected() > 0 {
            shop_settings_inserted += 1;
        }
    }

    let mut product_master_inserted = 0usize;
    for row in &product_master_rows {
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO product_master (raw_name, normalized_name, maker, series, product_name, scale, is_reissue, platform_hint)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&row.1)
        .bind(&row.2)
        .bind(&row.3)
        .bind(&row.4)
        .bind(&row.5)
        .bind(&row.6)
        .bind(row.7)
        .bind(&row.8)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert product_master: {e}"))?;
        if result.rows_affected() > 0 {
            product_master_inserted += 1;
        }
    }

    let mut emails_inserted = 0usize;
    if zip_archive.file_names().any(|n| n == "emails.ndjson") {
        // 新形式: NDJSON（read_until で行単位ストリーミング、長大行は確保前に拒否、OOM 回避）
        let rows: Vec<JsonEmailRow> = {
            let mut entry = zip_archive
                .by_name("emails.ndjson")
                .map_err(|e| format!("Failed to access emails.ndjson: {e}"))?;
            if entry.size() > MAX_EMAILS_NDJSON_ENTRY_SIZE {
                return Err(format!(
                    "emails.ndjson exceeds size limit (max {} bytes)",
                    MAX_EMAILS_NDJSON_ENTRY_SIZE
                ));
            }
            let mut reader = BufReader::new(&mut entry);
            let mut buf = Vec::with_capacity(4096);
            let mut vec = Vec::new();
            loop {
                buf.clear();
                let bytes_read = reader
                    .read_until(b'\n', &mut buf)
                    .map_err(|e| format!("Failed to read emails.ndjson: {e}"))?;
                if bytes_read == 0 {
                    break;
                }
                if buf.len() > MAX_NDJSON_LINE_SIZE + 1 {
                    return Err(format!(
                        "emails.ndjson line exceeds size limit (max {} bytes)",
                        MAX_NDJSON_LINE_SIZE
                    ));
                }
                if buf.last() == Some(&b'\n') {
                    buf.pop();
                }
                if buf.is_empty() {
                    continue;
                }
                let line = std::str::from_utf8(&buf)
                    .map_err(|e| format!("Failed to decode emails.ndjson as UTF-8: {e}"))?;
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let row: JsonEmailRow = serde_json::from_str(line)
                    .map_err(|e| format!("Failed to parse emails.ndjson line: {e}"))?;
                vec.push(row);
            }
            vec
        };
        for row in &rows {
            let result = sqlx::query(
                r#"
                INSERT OR IGNORE INTO emails (message_id, body_plain, body_html, analysis_status, created_at, updated_at, internal_date, from_address, subject)
                VALUES (?, ?, ?, ?, COALESCE(?, CURRENT_TIMESTAMP), COALESCE(?, CURRENT_TIMESTAMP), ?, ?, ?)
                "#,
            )
            .bind(&row.1)
            .bind(&row.2)
            .bind(&row.3)
            .bind(&row.4)
            .bind(&row.5)
            .bind(&row.6)
            .bind(row.7)
            .bind(&row.8)
            .bind(&row.9)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert email: {e}"))?;
            if result.rows_affected() > 0 {
                emails_inserted += 1;
            }
        }
    } else if zip_archive.file_names().any(|n| n == "emails.json") {
        // 旧形式: emails.json（レガシーバックアップ互換）
        // 1つの Deserializer で全体を読み取り、バッファ先読みとの不整合を防ぐ
        let emails_rows: Vec<JsonEmailRow> = {
            let mut entry = zip_archive
                .by_name("emails.json")
                .map_err(|e| format!("Failed to access emails.json: {e}"))?;
            if entry.size() > MAX_EMAILS_JSON_ENTRY_SIZE {
                return Err(format!(
                    "emails.json exceeds size limit (max {} bytes)",
                    MAX_EMAILS_JSON_ENTRY_SIZE
                ));
            }
            serde_json::from_reader(BufReader::new(&mut entry))
                .map_err(|e| format!("Failed to parse emails.json: {e}"))?
        };
        for row in &emails_rows {
            let result = sqlx::query(
                r#"
                INSERT OR IGNORE INTO emails (message_id, body_plain, body_html, analysis_status, created_at, updated_at, internal_date, from_address, subject)
                VALUES (?, ?, ?, ?, COALESCE(?, CURRENT_TIMESTAMP), COALESCE(?, CURRENT_TIMESTAMP), ?, ?, ?)
                "#,
            )
            .bind(&row.1)
            .bind(&row.2)
            .bind(&row.3)
            .bind(&row.4)
            .bind(&row.5)
            .bind(&row.6)
            .bind(row.7)
            .bind(&row.8)
            .bind(&row.9)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert email: {e}"))?;
            if result.rows_affected() > 0 {
                emails_inserted += 1;
            }
        }
    }

    let mut item_overrides_inserted = 0usize;
    for row in &item_overrides_rows {
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO item_overrides (
                shop_domain, order_number, original_item_name, original_brand,
                item_name, price, quantity, brand, category
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&row.1)
        .bind(&row.2)
        .bind(&row.3)
        .bind(&row.4)
        .bind(&row.5)
        .bind(row.6)
        .bind(row.7)
        .bind(&row.8)
        .bind(&row.9)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert item_override: {e}"))?;
        if result.rows_affected() > 0 {
            item_overrides_inserted += 1;
        }
    }

    let mut order_overrides_inserted = 0usize;
    for row in &order_overrides_rows {
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO order_overrides (
                shop_domain, order_number, new_order_number, order_date, shop_name
            )
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&row.1)
        .bind(&row.2)
        .bind(&row.3)
        .bind(&row.4)
        .bind(&row.5)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert order_override: {e}"))?;
        if result.rows_affected() > 0 {
            order_overrides_inserted += 1;
        }
    }

    let mut excluded_items_inserted = 0usize;
    for row in &excluded_items_rows {
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO excluded_items (shop_domain, order_number, item_name, brand, reason)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&row.1)
        .bind(&row.2)
        .bind(&row.3)
        .bind(&row.4)
        .bind(&row.5)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert excluded_item: {e}"))?;
        if result.rows_affected() > 0 {
            excluded_items_inserted += 1;
        }
    }

    let mut excluded_orders_inserted = 0usize;
    for row in &excluded_orders_rows {
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO excluded_orders (shop_domain, order_number, reason)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&row.1)
        .bind(&row.2)
        .bind(&row.3)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert excluded_order: {e}"))?;
        if result.rows_affected() > 0 {
            excluded_orders_inserted += 1;
        }
    }

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit transaction: {e}"))?;

    let mut image_files_copied = 0usize;
    for i in 0..zip_archive.len() {
        let mut entry = zip_archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {e}"))?;

        // Zip Slip 対策: enclosed_name() で正規化された相対パスのみを扱う
        let Some(enclosed_path) = entry.enclosed_name() else {
            continue; // パストラバーサルや絶対パスなど、不正なパスはスキップ
        };

        // 期待するパス構造: images/<file_name> （images 直下のみを許可）
        if enclosed_path.parent() != Some(Path::new("images")) {
            continue;
        }

        let Some(file_name) = enclosed_path.file_name() else {
            continue; // ディレクトリエントリ等はスキップ
        };

        // images.json に登場する file_name のみコピー（意図しない大量ファイルの DoS 対策）
        if !file_name
            .to_str()
            .is_some_and(|s| allowed_image_files.contains(s))
        {
            continue;
        }

        let dest = images_dir.join(file_name);
        if dest.exists() {
            continue; // 既存を維持（スキップ）
        }

        // 巨大エントリによるメモリ消費を防ぐ
        let size = entry.size();
        if size > MAX_IMAGE_ENTRY_SIZE {
            continue; // サイズ上限超過はスキップ
        }

        let mut data = Vec::new();
        entry
            .read_to_end(&mut data)
            .map_err(|e| format!("Failed to read image {:?}: {e}", enclosed_path))?;
        fs::write(&dest, &data)
            .map_err(|e| format!("Failed to write image {}: {e}", dest.display()))?;
        image_files_copied += 1;
    }

    Ok(ImportResult {
        images_inserted,
        shop_settings_inserted,
        product_master_inserted,
        emails_inserted,
        item_overrides_inserted,
        order_overrides_inserted,
        excluded_items_inserted,
        excluded_orders_inserted,
        image_files_copied,
        restore_point_updated: false,
        restore_point_path: None,
        restore_point_error: None,
    })
}

fn read_zip_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<String, String> {
    let mut entry = archive
        .by_name(name)
        .map_err(|e| format!("Missing {} in zip: {e}", name))?;
    if entry.size() > MAX_JSON_ENTRY_SIZE {
        return Err(format!(
            "{} exceeds size limit (max {} bytes)",
            name, MAX_JSON_ENTRY_SIZE
        ));
    }
    let mut s = String::new();
    entry
        .read_to_string(&mut s)
        .map_err(|e| format!("Failed to read {}: {e}", name))?;
    Ok(s)
}

/// JSON デシリアライズ用（タプル形式、id を含むがインポート時は未使用）
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JsonImageRow(
    i64,            // id (未使用)
    String,         // item_name_normalized
    Option<String>, // file_name
    Option<String>, // created_at
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JsonShopSettingsRow(
    i64,            // id (未使用)
    String,         // shop_name
    String,         // sender_address
    String,         // parser_type
    i32,            // is_enabled
    Option<String>, // subject_filters
    Option<String>, // created_at (未使用)
    Option<String>, // updated_at (未使用)
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JsonProductMasterRow(
    i64,            // id (未使用)
    String,         // raw_name
    String,         // normalized_name
    Option<String>, // maker
    Option<String>, // series
    Option<String>, // product_name
    Option<String>, // scale
    i32,            // is_reissue
    Option<String>, // platform_hint
    Option<String>, // created_at (未使用)
    Option<String>, // updated_at (未使用)
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JsonEmailRow(
    i64,            // id (未使用)
    String,         // message_id
    Option<String>, // body_plain
    Option<String>, // body_html
    String,         // analysis_status
    Option<String>, // created_at
    Option<String>, // updated_at
    Option<i64>,    // internal_date
    Option<String>, // from_address
    Option<String>, // subject
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JsonItemOverrideRow(
    i64,            // id (未使用)
    String,         // shop_domain
    String,         // order_number
    String,         // original_item_name
    String,         // original_brand
    Option<String>, // item_name
    Option<i64>,    // price
    Option<i64>,    // quantity
    Option<String>, // brand
    Option<String>, // category
    Option<String>, // created_at (未使用)
    Option<String>, // updated_at (未使用)
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JsonOrderOverrideRow(
    i64,            // id (未使用)
    String,         // shop_domain
    String,         // order_number
    Option<String>, // new_order_number
    Option<String>, // order_date
    Option<String>, // shop_name
    Option<String>, // created_at (未使用)
    Option<String>, // updated_at (未使用)
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JsonExcludedItemRow(
    i64,            // id (未使用)
    String,         // shop_domain
    String,         // order_number
    String,         // item_name
    String,         // brand
    Option<String>, // reason
    Option<String>, // created_at (未使用)
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JsonExcludedOrderRow(
    i64,            // id (未使用)
    String,         // shop_domain
    String,         // order_number
    Option<String>, // reason
    Option<String>, // created_at (未使用)
);

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::io::{Cursor, Write};
    use std::str::FromStr;
    use tempfile::TempDir;

    const SCHEMA_IMAGES: &str = r"
        CREATE TABLE IF NOT EXISTS images (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            item_name_normalized TEXT NOT NULL,
            file_name TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE (item_name_normalized)
        );
    ";
    const SCHEMA_SHOP_SETTINGS: &str = r"
        CREATE TABLE IF NOT EXISTS shop_settings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            shop_name TEXT NOT NULL,
            sender_address TEXT NOT NULL,
            parser_type TEXT NOT NULL,
            is_enabled INTEGER NOT NULL DEFAULT 1 CHECK(is_enabled IN (0, 1)),
            subject_filters TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE (sender_address, parser_type)
        );
    ";
    const SCHEMA_PRODUCT_MASTER: &str = r"
        CREATE TABLE IF NOT EXISTS product_master (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            raw_name TEXT UNIQUE NOT NULL,
            normalized_name TEXT NOT NULL,
            maker TEXT,
            series TEXT,
            product_name TEXT,
            scale TEXT,
            is_reissue INTEGER NOT NULL DEFAULT 0 CHECK(is_reissue IN (0, 1)),
            platform_hint TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
    ";
    const SCHEMA_EMAILS: &str = r"
        CREATE TABLE IF NOT EXISTS emails (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id TEXT UNIQUE NOT NULL,
            body_plain TEXT,
            body_html TEXT,
            analysis_status TEXT NOT NULL DEFAULT 'pending' CHECK(analysis_status IN ('pending', 'completed')),
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            internal_date INTEGER,
            from_address TEXT,
            subject TEXT
        );
    ";

    const SCHEMA_ITEM_OVERRIDES: &str = r"
        CREATE TABLE IF NOT EXISTS item_overrides (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            shop_domain TEXT NOT NULL,
            order_number TEXT NOT NULL,
            original_item_name TEXT NOT NULL,
            original_brand TEXT NOT NULL,
            item_name TEXT,
            price INTEGER,
            quantity INTEGER,
            brand TEXT,
            category TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE (shop_domain, order_number, original_item_name, original_brand)
        );
    ";

    const SCHEMA_ORDER_OVERRIDES: &str = r"
        CREATE TABLE IF NOT EXISTS order_overrides (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            shop_domain TEXT NOT NULL,
            order_number TEXT NOT NULL,
            new_order_number TEXT,
            order_date TEXT,
            shop_name TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE (shop_domain, order_number)
        );
    ";

    const SCHEMA_EXCLUDED_ITEMS: &str = r"
        CREATE TABLE IF NOT EXISTS excluded_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            shop_domain TEXT NOT NULL,
            order_number TEXT NOT NULL,
            item_name TEXT NOT NULL,
            brand TEXT NOT NULL,
            reason TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE (shop_domain, order_number, item_name, brand)
        );
    ";

    const SCHEMA_EXCLUDED_ORDERS: &str = r"
        CREATE TABLE IF NOT EXISTS excluded_orders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            shop_domain TEXT NOT NULL,
            order_number TEXT NOT NULL,
            reason TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE (shop_domain, order_number)
        );
    ";

    async fn create_test_pool() -> SqlitePool {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .unwrap();
        sqlx::query(SCHEMA_IMAGES).execute(&pool).await.unwrap();
        sqlx::query(SCHEMA_SHOP_SETTINGS)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(SCHEMA_PRODUCT_MASTER)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(SCHEMA_EMAILS).execute(&pool).await.unwrap();
        sqlx::query(SCHEMA_ITEM_OVERRIDES)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(SCHEMA_ORDER_OVERRIDES)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(SCHEMA_EXCLUDED_ITEMS)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(SCHEMA_EXCLUDED_ORDERS)
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_export_import_roundtrip() {
        let pool = create_test_pool().await;

        // テストデータを挿入
        sqlx::query(
            r"INSERT INTO images (item_name_normalized, file_name, created_at)
              VALUES ('item1', 'img1.png', '2024-01-01 00:00:00'), ('item2', NULL, '2024-01-02 00:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO shop_settings (shop_name, sender_address, parser_type, is_enabled, subject_filters) \
             VALUES ('ShopA', 'a@test.com', 'parser1', 1, '[\"filter1\"]')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"INSERT INTO product_master (raw_name, normalized_name, maker, is_reissue)
              VALUES ('RawProduct', 'NormProduct', 'Maker1', 0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"INSERT INTO emails (message_id, body_plain, analysis_status, from_address, subject)
              VALUES ('msg-001', 'body text', 'pending', 'a@test.com', 'Subject')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"INSERT INTO item_overrides (shop_domain, order_number, original_item_name, original_brand, item_name, price, quantity, brand, category)
              VALUES ('example.com', 'ORDER-1', 'orig-item', 'orig-brand', 'new-item', 1000, 2, 'new-brand', 'cat')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"INSERT INTO order_overrides (shop_domain, order_number, new_order_number, order_date, shop_name)
              VALUES ('example.com', 'ORDER-1', 'ORDER-NEW', '2024-01-01', 'ShopA')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"INSERT INTO excluded_items (shop_domain, order_number, item_name, brand, reason)
              VALUES ('example.com', 'ORDER-1', 'excluded-item', 'brand-x', 'test')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"INSERT INTO excluded_orders (shop_domain, order_number, reason)
              VALUES ('example.com', 'ORDER-2', 'spam')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let tmp = TempDir::new().unwrap();
        let images_dir = tmp.path().join("images");
        std::fs::create_dir_all(&images_dir).unwrap();

        // エクスポート（ZIP をメモリに書き込み）
        let mut buf = Cursor::new(Vec::new());
        let result = export_metadata_to_writer(&pool, &images_dir, &mut buf).await;
        assert!(result.is_ok(), "export failed: {:?}", result.err());
        let export_result = result.unwrap();
        assert_eq!(export_result.images_count, 2);
        assert_eq!(export_result.shop_settings_count, 1);
        assert_eq!(export_result.product_master_count, 1);
        assert_eq!(export_result.emails_count, 1);
        assert_eq!(export_result.item_overrides_count, 1);
        assert_eq!(export_result.order_overrides_count, 1);
        assert_eq!(export_result.excluded_items_count, 1);
        assert_eq!(export_result.excluded_orders_count, 1);
        assert_eq!(export_result.image_files_count, 0); // img1.png は存在しない
        assert_eq!(export_result.images_skipped, 1); // img1.png が存在しないためスキップ

        // インポート先の新規 DB
        let pool2 = create_test_pool().await;
        buf.set_position(0);

        let import_result = import_metadata_from_reader(&pool2, &images_dir, buf).await;
        assert!(
            import_result.is_ok(),
            "import failed: {:?}",
            import_result.err()
        );
        let import_result = import_result.unwrap();
        assert_eq!(import_result.images_inserted, 2);
        assert_eq!(import_result.shop_settings_inserted, 1);
        assert_eq!(import_result.product_master_inserted, 1);
        assert_eq!(import_result.emails_inserted, 1);
        assert_eq!(import_result.item_overrides_inserted, 1);
        assert_eq!(import_result.order_overrides_inserted, 1);
        assert_eq!(import_result.excluded_items_inserted, 1);
        assert_eq!(import_result.excluded_orders_inserted, 1);

        // データが正しく復元されているか確認
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM images")
            .fetch_one(&pool2)
            .await
            .unwrap();
        assert_eq!(count.0, 2);
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM shop_settings")
            .fetch_one(&pool2)
            .await
            .unwrap();
        assert_eq!(count.0, 1);
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM product_master")
            .fetch_one(&pool2)
            .await
            .unwrap();
        assert_eq!(count.0, 1);
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM emails")
            .fetch_one(&pool2)
            .await
            .unwrap();
        assert_eq!(count.0, 1);
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM item_overrides")
            .fetch_one(&pool2)
            .await
            .unwrap();
        assert_eq!(count.0, 1);
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM order_overrides")
            .fetch_one(&pool2)
            .await
            .unwrap();
        assert_eq!(count.0, 1);
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM excluded_items")
            .fetch_one(&pool2)
            .await
            .unwrap();
        assert_eq!(count.0, 1);
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM excluded_orders")
            .fetch_one(&pool2)
            .await
            .unwrap();
        assert_eq!(count.0, 1);
    }

    #[tokio::test]
    async fn test_import_insert_or_ignore_duplicate() {
        let pool = create_test_pool().await;

        // 初期データ
        sqlx::query(
            r"INSERT INTO images (item_name_normalized, file_name, created_at)
              VALUES ('dup_item', 'dup.png', '2024-01-01 00:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let tmp = TempDir::new().unwrap();
        let images_dir = tmp.path().join("images");
        std::fs::create_dir_all(&images_dir).unwrap();

        let mut buf = Cursor::new(Vec::new());
        export_metadata_to_writer(&pool, &images_dir, &mut buf)
            .await
            .unwrap();
        buf.set_position(0);

        // 同じ DB に再インポート → 重複は無視
        let import_result = import_metadata_from_reader(&pool, &images_dir, buf).await;
        assert!(import_result.is_ok());
        let r = import_result.unwrap();
        assert_eq!(r.images_inserted, 0, "duplicate should be ignored");
    }

    #[tokio::test]
    async fn test_export_zip_contents() {
        let pool = create_test_pool().await;
        sqlx::query(
            r"INSERT INTO images (item_name_normalized, file_name, created_at)
              VALUES ('item1', 'img1.png', '2024-01-01 00:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let tmp = TempDir::new().unwrap();
        let images_dir = tmp.path().join("images");
        std::fs::create_dir_all(&images_dir).unwrap();

        let mut buf = Cursor::new(Vec::new());
        let export_result = export_metadata_to_writer(&pool, &images_dir, &mut buf)
            .await
            .unwrap();
        assert_eq!(export_result.images_skipped, 1); // img1.png は存在しない
        buf.set_position(0);

        // ZIP 内容を検証
        let mut zip = ZipArchive::new(buf).unwrap();
        let names: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_string())
            .collect();

        assert!(
            names.contains(&"manifest.json".to_string()),
            "manifest.json should exist, got: {:?}",
            names
        );
        assert!(
            names.contains(&"images.json".to_string()),
            "images.json should exist, got: {:?}",
            names
        );
        assert!(
            names.contains(&"shop_settings.json".to_string()),
            "shop_settings.json should exist, got: {:?}",
            names
        );
        assert!(
            names.contains(&"product_master.json".to_string()),
            "product_master.json should exist, got: {:?}",
            names
        );
        assert!(
            names.contains(&"emails.ndjson".to_string()),
            "emails.ndjson should exist, got: {:?}",
            names
        );
        assert!(
            names.contains(&"item_overrides.json".to_string()),
            "item_overrides.json should exist, got: {:?}",
            names
        );
        assert!(
            names.contains(&"order_overrides.json".to_string()),
            "order_overrides.json should exist, got: {:?}",
            names
        );
        assert!(
            names.contains(&"excluded_items.json".to_string()),
            "excluded_items.json should exist, got: {:?}",
            names
        );
        assert!(
            names.contains(&"excluded_orders.json".to_string()),
            "excluded_orders.json should exist, got: {:?}",
            names
        );
    }

    #[tokio::test]
    async fn test_import_with_image_files() {
        let pool = create_test_pool().await;
        sqlx::query(
            r"INSERT INTO images (item_name_normalized, file_name, created_at)
              VALUES ('item_with_img', 'test_img.png', '2024-01-01 00:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let tmp = TempDir::new().unwrap();
        let images_dir = tmp.path().join("images");
        std::fs::create_dir_all(&images_dir).unwrap();
        // 実際の画像ファイルを作成
        let img_path = images_dir.join("test_img.png");
        std::fs::write(&img_path, b"fake png content").unwrap();

        let mut buf = Cursor::new(Vec::new());
        let export_result = export_metadata_to_writer(&pool, &images_dir, &mut buf)
            .await
            .unwrap();
        assert_eq!(export_result.image_files_count, 1);

        // 空の DB にインポート
        let pool2 = create_test_pool().await;
        let images_dir2 = tmp.path().join("images_import");
        std::fs::create_dir_all(&images_dir2).unwrap();
        buf.set_position(0);

        let import_result = import_metadata_from_reader(&pool2, &images_dir2, buf)
            .await
            .unwrap();
        assert_eq!(import_result.image_files_copied, 1);
        assert!(images_dir2.join("test_img.png").exists());
    }

    #[tokio::test]
    async fn test_export_skips_unsafe_file_name() {
        let pool = create_test_pool().await;
        sqlx::query(
            r"INSERT INTO images (item_name_normalized, file_name, created_at)
              VALUES ('item1', 'normal.png', '2024-01-01 00:00:00'),
                     ('item2', '../evil.png', '2024-01-02 00:00:00'),
                     ('item3', 'subdir/file.png', '2024-01-03 00:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let tmp = TempDir::new().unwrap();
        let images_dir = tmp.path().join("images");
        std::fs::create_dir_all(&images_dir).unwrap();
        std::fs::write(images_dir.join("normal.png"), b"ok").unwrap();

        let mut buf = Cursor::new(Vec::new());
        let result = export_metadata_to_writer(&pool, &images_dir, &mut buf)
            .await
            .unwrap();
        // 3件の images レコードはあるが、ZIP に含まれる画像は normal.png のみ（../evil.png, subdir/file.png はスキップ）
        assert_eq!(result.images_count, 3);
        assert_eq!(result.image_files_count, 1);
        assert_eq!(result.images_skipped, 2);
    }

    #[tokio::test]
    async fn test_import_sanitizes_unsafe_file_name() {
        let pool = create_test_pool().await;
        let tmp = TempDir::new().unwrap();
        let images_dir = tmp.path().join("images");
        std::fs::create_dir_all(&images_dir).unwrap();

        // 不正な file_name を含む images.json を持つ ZIP を作成
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let options: zip::write::FileOptions<()> = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("manifest.json", options).unwrap();
            zip.write_all(b"{\"version\": 1, \"exported_at\": \"2024-01-01 00:00:00\"}")
                .unwrap();
            zip.start_file("images.json", options).unwrap();
            zip.write_all(
                br#"[[1,"safe_item","safe.png","2024-01-01 00:00:00"],[2,"unsafe_item","../evil.png","2024-01-02 00:00:00"]]"#,
            )
            .unwrap();
            zip.start_file("shop_settings.json", options).unwrap();
            zip.write_all(b"[]").unwrap();
            zip.start_file("product_master.json", options).unwrap();
            zip.write_all(b"[]").unwrap();
            zip.finish().unwrap();
        }
        buf.set_position(0);

        let import_result = import_metadata_from_reader(&pool, &images_dir, buf).await;
        assert!(
            import_result.is_ok(),
            "import failed: {:?}",
            import_result.err()
        );
        let r = import_result.unwrap();
        assert_eq!(r.images_inserted, 2);

        // safe_item は file_name あり、unsafe_item は file_name が None で保存されている
        let rows: Vec<(String, Option<String>)> = sqlx::query_as(
            "SELECT item_name_normalized, file_name FROM images ORDER BY item_name_normalized",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0],
            ("safe_item".to_string(), Some("safe.png".to_string()))
        );
        assert_eq!(rows[1], ("unsafe_item".to_string(), None));
    }

    #[tokio::test]
    async fn test_import_rejects_wrong_manifest_version() {
        use zip::write::FileOptions;

        let pool = create_test_pool().await;
        let tmp = TempDir::new().unwrap();
        let images_dir = tmp.path().join("images");
        std::fs::create_dir_all(&images_dir).unwrap();

        // 不正なバージョンの manifest を含む ZIP を作成
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let options: zip::write::FileOptions<()> =
                FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("manifest.json", options).unwrap();
            zip.write_all(b"{\"version\": 999, \"exported_at\": \"2024-01-01 00:00:00\"}")
                .unwrap();
            zip.start_file("images.json", options).unwrap();
            zip.write_all(b"[]").unwrap();
            zip.start_file("shop_settings.json", options).unwrap();
            zip.write_all(b"[]").unwrap();
            zip.start_file("product_master.json", options).unwrap();
            zip.write_all(b"[]").unwrap();
            zip.finish().unwrap();
        }
        buf.set_position(0);

        let result = import_metadata_from_reader(&pool, &images_dir, buf).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported backup version"));
    }

    #[tokio::test]
    async fn test_import_legacy_emails_json() {
        use zip::write::FileOptions;

        let pool = create_test_pool().await;
        let tmp = TempDir::new().unwrap();
        let images_dir = tmp.path().join("images");
        std::fs::create_dir_all(&images_dir).unwrap();

        // emails.ndjson が無く emails.json（レガシー形式）のみを含む ZIP
        let emails_json = r#"[1,"msg-legacy-001","body plain","body html","pending","2024-01-01 00:00:00",null,null,"legacy@test.com","Legacy Subject"]"#;
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let options: zip::write::FileOptions<()> =
                FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("manifest.json", options).unwrap();
            zip.write_all(b"{\"version\": 1, \"exported_at\": \"2024-01-01 00:00:00\"}")
                .unwrap();
            zip.start_file("images.json", options).unwrap();
            zip.write_all(b"[]").unwrap();
            zip.start_file("shop_settings.json", options).unwrap();
            zip.write_all(b"[]").unwrap();
            zip.start_file("product_master.json", options).unwrap();
            zip.write_all(b"[]").unwrap();
            zip.start_file("emails.json", options).unwrap();
            zip.write_all(format!("[{}]", emails_json).as_bytes())
                .unwrap();
            zip.finish().unwrap();
        }
        buf.set_position(0);

        let import_result = import_metadata_from_reader(&pool, &images_dir, buf).await;
        assert!(
            import_result.is_ok(),
            "import failed: {:?}",
            import_result.err()
        );
        let r = import_result.unwrap();
        assert_eq!(r.emails_inserted, 1, "emails_inserted should be 1");

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM emails")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 1, "emails table should have 1 row");

        let row: (String, String, Option<String>) =
            sqlx::query_as("SELECT message_id, analysis_status, subject FROM emails LIMIT 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "msg-legacy-001");
        assert_eq!(row.1, "pending");
        assert_eq!(row.2.as_deref(), Some("Legacy Subject"));
    }

    #[tokio::test]
    async fn test_import_missing_override_and_exclusion_files_is_ok() {
        use zip::write::FileOptions;

        let pool = create_test_pool().await;
        let tmp = TempDir::new().unwrap();
        let images_dir = tmp.path().join("images");
        std::fs::create_dir_all(&images_dir).unwrap();

        // 旧バックアップ相当: overrides/excluded の JSON が無い ZIP を作成
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let options: zip::write::FileOptions<()> =
                FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("manifest.json", options).unwrap();
            zip.write_all(b"{\"version\": 1, \"exported_at\": \"2024-01-01 00:00:00\"}")
                .unwrap();
            zip.start_file("images.json", options).unwrap();
            zip.write_all(b"[]").unwrap();
            zip.start_file("shop_settings.json", options).unwrap();
            zip.write_all(b"[]").unwrap();
            zip.start_file("product_master.json", options).unwrap();
            zip.write_all(b"[]").unwrap();
            zip.start_file("emails.ndjson", options).unwrap();
            zip.finish().unwrap();
        }
        buf.set_position(0);

        let import_result = import_metadata_from_reader(&pool, &images_dir, buf).await;
        assert!(
            import_result.is_ok(),
            "import failed: {:?}",
            import_result.err()
        );
        let r = import_result.unwrap();
        assert_eq!(r.item_overrides_inserted, 0);
        assert_eq!(r.order_overrides_inserted, 0);
        assert_eq!(r.excluded_items_inserted, 0);
        assert_eq!(r.excluded_orders_inserted, 0);
    }

    #[test]
    fn test_copy_restore_point_zip_success() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src.zip");
        std::fs::write(&src, b"dummy zip bytes").unwrap();

        let restore_dir = tmp.path().join("app_data");
        let restore_path = restore_dir.join("paa_restore_point.zip");

        let (saved, err) = super::copy_restore_point_zip(&src, &restore_path);
        assert!(saved);
        assert!(err.is_none());
        assert!(restore_path.exists());
        let copied = std::fs::read(&restore_path).unwrap();
        assert_eq!(copied, b"dummy zip bytes");
    }

    #[test]
    fn test_copy_restore_point_zip_fails_when_parent_is_file() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src.zip");
        std::fs::write(&src, b"dummy zip bytes").unwrap();

        // parent をファイルにして create_dir_all を失敗させる
        let parent_as_file = tmp.path().join("not_a_dir");
        std::fs::write(&parent_as_file, b"x").unwrap();
        let restore_path = parent_as_file.join("paa_restore_point.zip");

        let (saved, err) = super::copy_restore_point_zip(&src, &restore_path);
        assert!(!saved);
        assert!(err.is_some());
        assert!(!restore_path.exists());
    }
}
