//! メタデータのエクスポート処理

use futures::StreamExt;
use sqlx::sqlite::SqlitePool;
use std::fs::{self, File};
use std::io::{Seek, Write as IoWrite};
use std::path::Path;
use tauri::{AppHandle, Manager};
use zip::write::FileOptions;

use super::file_safety::{copy_restore_point_zip, is_safe_file_name, RESTORE_POINT_FILE_NAME};
use super::manifest::{Manifest, MANIFEST_VERSION, MAX_IMAGE_ENTRY_SIZE, MAX_NDJSON_LINE_SIZE};
use super::table_converters::{
    EmailRow, ExcludedItemRow, ExcludedOrderRow, ExportResult, ItemOverrideRow, OrderOverrideRow,
    ProductMasterRow, ShopSettingsRow, TrackingCheckLogRow,
};

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

    // 復元ポイントの保存（app_data_dir は既に取得済みなので再利用）
    let restore_point_path = app_data_dir.join(RESTORE_POINT_FILE_NAME);
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
    W: IoWrite + Seek,
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

    let tracking_check_logs_rows: Vec<TrackingCheckLogRow> = sqlx::query_as(
        r#"
        SELECT id, delivery_id, checked_at, check_status, delivery_status,
               description, location, error_message, created_at
        FROM tracking_check_logs
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch tracking_check_logs: {e}"))?;

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
    let tracking_check_logs_json = serde_json::to_string_pretty(&tracking_check_logs_rows)
        .map_err(|e| format!("Failed to serialize tracking_check_logs: {e}"))?;

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

    zip_writer
        .start_file("tracking_check_logs.json", options)
        .map_err(|e| format!("Failed to add tracking_check_logs.json: {e}"))?;
    zip_writer
        .write_all(tracking_check_logs_json.as_bytes())
        .map_err(|e| format!("Failed to write tracking_check_logs: {e}"))?;

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
        tracking_check_logs_count: tracking_check_logs_rows.len(),
        image_files_count,
        images_skipped,
        restore_point_saved: false,
        restore_point_path: None,
        restore_point_error: None,
    })
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;
    use tempfile::TempDir;
    use zip::ZipArchive;

    use super::export_metadata_to_writer;
    use crate::metadata::import::import_metadata_from_reader;

    async fn create_test_pool() -> sqlx::sqlite::SqlitePool {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .unwrap();
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS images (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                item_name_normalized TEXT NOT NULL,
                file_name TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (item_name_normalized)
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"
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
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"
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
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"
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
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"
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
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"
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
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS excluded_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                shop_domain TEXT NOT NULL,
                order_number TEXT NOT NULL,
                item_name TEXT NOT NULL,
                brand TEXT NOT NULL,
                reason TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (shop_domain, order_number, item_name, brand)
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS excluded_orders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                shop_domain TEXT NOT NULL,
                order_number TEXT NOT NULL,
                reason TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (shop_domain, order_number)
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS deliveries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tracking_check_logs (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                delivery_id     INTEGER NOT NULL,
                checked_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                check_status    TEXT NOT NULL DEFAULT 'success'
                                CHECK(check_status IN ('success', 'failed', 'not_found')),
                delivery_status TEXT
                                CHECK(delivery_status IS NULL OR delivery_status IN (
                                    'not_shipped', 'preparing', 'shipped', 'in_transit',
                                    'out_for_delivery', 'delivered', 'failed', 'returned', 'cancelled'
                                )),
                description     TEXT,
                location        TEXT,
                error_message   TEXT,
                created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (delivery_id) REFERENCES deliveries(id) ON DELETE CASCADE
            );",
        )
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
        sqlx::query(
            r"INSERT INTO deliveries (id, order_id) VALUES (1, 99)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"INSERT INTO tracking_check_logs
              (delivery_id, checked_at, check_status, delivery_status, description, location)
              VALUES (1, '2024-01-01 12:00:00', 'success', 'in_transit', '品川営業所に到着', '品川営業所')",
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
        assert_eq!(export_result.tracking_check_logs_count, 1);
        assert_eq!(export_result.image_files_count, 0); // img1.png は存在しない
        assert_eq!(export_result.images_skipped, 1); // img1.png が存在しないためスキップ

        // インポート先の新規 DB
        let pool2 = create_test_pool().await;
        // tracking_check_logs の FK を満たすため配送レコードを事前挿入
        sqlx::query("INSERT INTO deliveries (id, order_id) VALUES (1, 99)")
            .execute(&pool2)
            .await
            .unwrap();
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
        assert_eq!(import_result.tracking_check_logs_inserted, 1);

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
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tracking_check_logs")
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
        assert!(
            names.contains(&"tracking_check_logs.json".to_string()),
            "tracking_check_logs.json should exist, got: {:?}",
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
}
