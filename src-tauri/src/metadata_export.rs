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

/// 画像ファイル1件あたりの最大サイズ（バイト）。巨大エントリによるメモリ消費を防ぐ。
const MAX_IMAGE_ENTRY_SIZE: u64 = 10 * 1024 * 1024; // 10MB

/// JSON エントリ1件あたりの最大サイズ（バイト）。巨大 ZIP による DoS を防ぐ。
const MAX_JSON_ENTRY_SIZE: u64 = 10 * 1024 * 1024; // 10MB

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
    let file = File::create(save_path).map_err(|e| format!("Failed to create file: {e}"))?;
    export_metadata_to_writer(pool, &images_dir, file).await
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

    let mut image_files_count = 0usize;
    for (_, _norm, file_name_opt, _) in &images_rows {
        if let Some(ref file_name) = file_name_opt {
            if !is_safe_file_name(file_name) {
                continue; // パストラバーサル対策: 不正な file_name はスキップ
            }
            let src = images_dir.join(file_name);
            if src.exists() {
                let metadata = fs::metadata(&src).ok();
                if metadata
                    .map(|m| m.len() > MAX_IMAGE_ENTRY_SIZE)
                    .unwrap_or(true)
                {
                    continue; // サイズ不明 or 超過はスキップ
                }
                let data = fs::read(&src)
                    .map_err(|e| format!("Failed to read image {}: {e}", file_name))?;
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
    import_metadata_from_reader(pool, &images_dir, file).await
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
        assert_eq!(export_result.image_files_count, 0); // img1.png は存在しない

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
        export_metadata_to_writer(&pool, &images_dir, &mut buf)
            .await
            .unwrap();
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
}
