//! メタデータのエクスポート/インポート（Issue #40）
//!
//! images, shop_settings, product_master テーブルと画像ファイルを
//! ZIP 形式でバックアップ・復元する。

use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::fs::{self, File};
use std::io::{Read, Seek, Write};
use std::path::Path;
use tauri::{AppHandle, Manager};
use zip::write::FileOptions;
use zip::ZipArchive;

const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportResult {
    pub images_count: usize,
    pub shop_settings_count: usize,
    pub product_master_count: usize,
    pub image_files_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub images_inserted: usize,
    pub shop_settings_inserted: usize,
    pub product_master_inserted: usize,
    pub image_files_copied: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    exported_at: String,
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

    // 1. テーブルデータを取得
    let images_rows: Vec<(i64, String, Option<String>, String)> = sqlx::query_as(
        "SELECT id, item_name_normalized, file_name, created_at FROM images",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch images: {e}"))?;

    let shop_settings_rows: Vec<(
        i64,
        String,
        String,
        String,
        i32,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        r#"
        SELECT id, shop_name, sender_address, parser_type, is_enabled,
               subject_filters, created_at, updated_at FROM shop_settings
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch shop_settings: {e}"))?;

    let product_master_rows: Vec<(
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
    )> = sqlx::query_as(
        r#"
        SELECT id, raw_name, normalized_name, maker, series, product_name, scale,
               is_reissue, platform_hint, created_at, updated_at FROM product_master
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch product_master: {e}"))?;

    // 2. JSON にシリアライズ
    let images_json = serde_json::to_string_pretty(&images_rows)
        .map_err(|e| format!("Failed to serialize images: {e}"))?;
    let shop_settings_json = serde_json::to_string_pretty(&shop_settings_rows)
        .map_err(|e| format!("Failed to serialize shop_settings: {e}"))?;
    let product_master_json = serde_json::to_string_pretty(&product_master_rows)
        .map_err(|e| format!("Failed to serialize product_master: {e}"))?;

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
    let file = File::create(save_path).map_err(|e| format!("Failed to create file: {e}"))?;
    let mut zip_writer = zip::ZipWriter::new(file);
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

    let mut image_files_count = 0usize;
    for (_, _norm, file_name_opt, _) in &images_rows {
        if let Some(ref file_name) = file_name_opt {
            let src = images_dir.join(file_name);
            if src.exists() {
                let data = fs::read(&src).map_err(|e| format!("Failed to read image {}: {e}", file_name))?;
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
    }

    zip_writer
        .finish()
        .map_err(|e| format!("Failed to finish zip: {e}"))?;

    Ok(ExportResult {
        images_count: images_rows.len(),
        shop_settings_count: shop_settings_rows.len(),
        product_master_count: product_master_rows.len(),
        image_files_count,
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
    let mut zip_archive = ZipArchive::new(file).map_err(|e| format!("Failed to read zip: {e}"))?;

    // images.json
    let images_json = read_zip_entry(&mut zip_archive, "images.json")?;
    let images_rows: Vec<JsonImageRow> =
        serde_json::from_str(&images_json).map_err(|e| format!("Failed to parse images.json: {e}"))?;

    // shop_settings.json
    let shop_settings_json = read_zip_entry(&mut zip_archive, "shop_settings.json")?;
    let shop_settings_rows: Vec<JsonShopSettingsRow> = serde_json::from_str(&shop_settings_json)
        .map_err(|e| format!("Failed to parse shop_settings.json: {e}"))?;

    // product_master.json
    let product_master_json = read_zip_entry(&mut zip_archive, "product_master.json")?;
    let product_master_rows: Vec<JsonProductMasterRow> = serde_json::from_str(&product_master_json)
        .map_err(|e| format!("Failed to parse product_master.json: {e}"))?;

    let mut images_inserted = 0usize;
    for row in &images_rows {
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO images (item_name_normalized, file_name, created_at)
            VALUES (?, ?, COALESCE(?, CURRENT_TIMESTAMP))
            "#,
        )
        .bind(&row.1)
        .bind(&row.2)
        .bind(&row.3)
        .execute(pool)
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
        .execute(pool)
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
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to insert product_master: {e}"))?;
        if result.rows_affected() > 0 {
            product_master_inserted += 1;
        }
    }

    let mut image_files_copied = 0usize;
    for i in 0..zip_archive.len() {
        let mut entry = zip_archive.by_index(i).map_err(|e| format!("Failed to read zip entry: {e}"))?;
        let name = entry.name().to_string();
        if name.starts_with("images/") && !name.ends_with('/') {
            let file_name = name.trim_start_matches("images/");
            let dest = images_dir.join(file_name);
            if dest.exists() {
                continue; // 既存を維持（スキップ）
            }
            let mut data = Vec::new();
            entry.read_to_end(&mut data).map_err(|e| format!("Failed to read image {}: {e}", name))?;
            fs::write(&dest, &data).map_err(|e| format!("Failed to write image {}: {e}", dest.display()))?;
            image_files_copied += 1;
        }
    }

    Ok(ImportResult {
        images_inserted,
        shop_settings_inserted,
        product_master_inserted,
        image_files_copied,
    })
}

fn read_zip_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<String, String> {
    let mut entry = archive
        .by_name(name)
        .map_err(|e| format!("Missing {} in zip: {e}", name))?;
    let mut s = String::new();
    entry
        .read_to_string(&mut s)
        .map_err(|e| format!("Failed to read {}: {e}", name))?;
    Ok(s)
}

/// JSON デシリアライズ用（タプル形式、id 除く）
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JsonImageRow(
    i64,              // id (未使用)
    String,           // item_name_normalized
    Option<String>,   // file_name
    Option<String>,   // created_at
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JsonShopSettingsRow(
    i64,              // id (未使用)
    String,           // shop_name
    String,           // sender_address
    String,           // parser_type
    i32,              // is_enabled
    Option<String>,   // subject_filters
    Option<String>,   // created_at (未使用)
    Option<String>,   // updated_at (未使用)
);

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JsonProductMasterRow(
    i64,              // id (未使用)
    String,           // raw_name
    String,           // normalized_name
    Option<String>,   // maker
    Option<String>,   // series
    Option<String>,   // product_name
    Option<String>,   // scale
    i32,              // is_reissue
    Option<String>,   // platform_hint
    Option<String>,   // created_at (未使用)
    Option<String>,   // updated_at (未使用)
);
