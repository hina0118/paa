//! 復元ポイントからのリストア処理

use sqlx::sqlite::SqlitePool;
use std::fs;
use tauri::{AppHandle, Manager};

use super::file_safety::get_restore_point_path;
use super::import::import_metadata_from_reader;
use super::table_converters::ImportResult;

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

    let file = std::fs::File::open(&restore_point_path)
        .map_err(|e| format!("Failed to open restore point zip: {e}"))?;
    let mut result = import_metadata_from_reader(pool, &images_dir, file).await?;

    // restore コマンドでは復元ポイント自体は更新しない（読み取り専用）
    result.restore_point_updated = None;
    result.restore_point_path = Some(restore_point_path.display().to_string());
    result.restore_point_error = None;

    Ok(result)
}
