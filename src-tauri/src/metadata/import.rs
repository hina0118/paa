//! メタデータのインポート処理

use sqlx::sqlite::SqlitePool;
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, Read, Seek};
use std::path::Path;
use tauri::{AppHandle, Manager};
use zip::ZipArchive;

use super::file_safety::{copy_restore_point_zip, is_safe_file_name, RESTORE_POINT_FILE_NAME};
use super::manifest::{
    read_zip_entry, Manifest, MANIFEST_VERSION, MAX_EMAILS_JSON_ENTRY_SIZE,
    MAX_EMAILS_NDJSON_ENTRY_SIZE, MAX_IMAGE_ENTRY_SIZE, MAX_NDJSON_LINE_SIZE,
};
use super::table_converters::{
    ImportResult, JsonEmailRow, JsonExcludedItemRow, JsonExcludedOrderRow, JsonHtmlsRow,
    JsonImageRow, JsonItemExclusionPatternRow, JsonItemOverrideRow, JsonNewsClipRow,
    JsonOrderOverrideRow, JsonProductMasterRow, JsonShopSettingsRow, JsonTrackingCheckLogRow,
};

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
    let file = std::fs::File::open(zip_path).map_err(|e| format!("Failed to open zip: {e}"))?;
    let mut result = import_metadata_from_reader(pool, &images_dir, file).await?;

    // 復元ポイントの更新（app_data_dir は既に取得済みなので再利用）
    let restore_point_path = app_data_dir.join(RESTORE_POINT_FILE_NAME);
    let (updated, err) = copy_restore_point_zip(zip_path, &restore_point_path);
    result.restore_point_updated = Some(updated);
    result.restore_point_path = Some(restore_point_path.display().to_string());
    result.restore_point_error = err;

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
    // 旧バックアップ互換: ファイルが無ければスキップする
    let tracking_check_logs_rows: Vec<JsonTrackingCheckLogRow> = if zip_archive
        .file_names()
        .any(|n| n == "tracking_check_logs.json")
    {
        let json = read_zip_entry(&mut zip_archive, "tracking_check_logs.json")?;
        serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse tracking_check_logs.json: {e}"))?
    } else {
        Vec::new()
    };
    let htmls_rows: Vec<JsonHtmlsRow> =
        if zip_archive.file_names().any(|n| n == "htmls.json") {
            let json = read_zip_entry(&mut zip_archive, "htmls.json")?;
            serde_json::from_str(&json)
                .map_err(|e| format!("Failed to parse htmls.json: {e}"))?
        } else {
            Vec::new()
        };
    let news_clips_rows: Vec<JsonNewsClipRow> =
        if zip_archive.file_names().any(|n| n == "news_clips.json") {
            let json = read_zip_entry(&mut zip_archive, "news_clips.json")?;
            serde_json::from_str(&json)
                .map_err(|e| format!("Failed to parse news_clips.json: {e}"))?
        } else {
            Vec::new()
        };
    let item_exclusion_patterns_rows: Vec<JsonItemExclusionPatternRow> = if zip_archive
        .file_names()
        .any(|n| n == "item_exclusion_patterns.json")
    {
        let json = read_zip_entry(&mut zip_archive, "item_exclusion_patterns.json")?;
        serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse item_exclusion_patterns.json: {e}"))?
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

    // tracking_check_logs: tracking_number をキーに INSERT OR REPLACE（既存行は削除→再挿入）
    let mut tracking_check_logs_inserted = 0usize;
    for row in &tracking_check_logs_rows {
        let result = sqlx::query(
            r#"
            INSERT OR REPLACE INTO tracking_check_logs (
                tracking_number, checked_at, check_status, delivery_status,
                description, location, error_message, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, COALESCE(?, CURRENT_TIMESTAMP))
            "#,
        )
        .bind(&row.1)
        .bind(&row.2)
        .bind(&row.3)
        .bind(&row.4)
        .bind(&row.5)
        .bind(&row.6)
        .bind(&row.7)
        .bind(&row.8)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert tracking_check_log: {e}"))?;
        if result.rows_affected() > 0 {
            tracking_check_logs_inserted += 1;
        }
    }

    let mut htmls_inserted = 0usize;
    for row in &htmls_rows {
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO htmls (url, html_content, analysis_status)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&row.1)
        .bind(&row.2)
        .bind(&row.3)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert html: {e}"))?;
        if result.rows_affected() > 0 {
            htmls_inserted += 1;
        }
    }

    let mut news_clips_inserted = 0usize;
    for row in &news_clips_rows {
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO news_clips (title, url, source_name, published_at, summary, tags, clipped_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&row.1)
        .bind(&row.2)
        .bind(&row.3)
        .bind(&row.4)
        .bind(&row.5)
        .bind(&row.6)
        .bind(&row.7)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert news_clip: {e}"))?;
        if result.rows_affected() > 0 {
            news_clips_inserted += 1;
        }
    }

    // item_exclusion_patterns はスキーマ上 UNIQUE 制約なし。
    // 既存行と重複しないよう (shop_domain, keyword, match_type) の組み合わせで存在確認してからINSERT。
    let mut item_exclusion_patterns_inserted = 0usize;
    for row in &item_exclusion_patterns_rows {
        let exists: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM item_exclusion_patterns
            WHERE (shop_domain IS ? OR (shop_domain IS NULL AND ? IS NULL))
              AND keyword = ?
              AND match_type = ?
            "#,
        )
        .bind(&row.1)
        .bind(&row.1)
        .bind(&row.2)
        .bind(&row.3)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("Failed to check item_exclusion_pattern: {e}"))?;
        if exists.0 == 0 {
            sqlx::query(
                r#"
                INSERT INTO item_exclusion_patterns (shop_domain, keyword, match_type, note, created_at)
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
            .map_err(|e| format!("Failed to insert item_exclusion_pattern: {e}"))?;
            item_exclusion_patterns_inserted += 1;
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
        tracking_check_logs_inserted,
        htmls_inserted,
        news_clips_inserted,
        item_exclusion_patterns_inserted,
        image_files_copied,
        restore_point_updated: None,
        restore_point_path: None,
        restore_point_error: None,
    })
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;
    use tempfile::TempDir;

    use super::import_metadata_from_reader;

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
            CREATE TABLE IF NOT EXISTS tracking_check_logs (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                tracking_number TEXT NOT NULL,
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
                UNIQUE (tracking_number)
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS htmls (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT UNIQUE NOT NULL,
                html_content TEXT,
                analysis_status TEXT NOT NULL DEFAULT 'pending' CHECK(analysis_status IN ('pending', 'completed')),
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS news_clips (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                url TEXT NOT NULL,
                source_name TEXT NOT NULL,
                published_at TEXT,
                summary TEXT,
                tags TEXT NOT NULL DEFAULT '[]',
                clipped_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (url)
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS item_exclusion_patterns (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                shop_domain TEXT,
                keyword TEXT NOT NULL,
                match_type TEXT NOT NULL DEFAULT 'contains' CHECK(match_type IN ('contains', 'starts_with', 'exact')),
                note TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
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
        assert_eq!(r.tracking_check_logs_inserted, 0);
    }
}
