//! ファイル安全性チェック（パストラバーサル対策・復元ポイント管理）

use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub(super) const RESTORE_POINT_FILE_NAME: &str = "paa_restore_point.zip";

/// file_name が安全な単一ファイル名か検証（パストラバーサル対策）
pub(super) fn is_safe_file_name(file_name: &str) -> bool {
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

pub(super) fn get_restore_point_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    Ok(app_data_dir.join(RESTORE_POINT_FILE_NAME))
}

pub(super) fn copy_restore_point_zip(
    src_zip_path: &Path,
    restore_point_path: &Path,
) -> (bool, Option<String>) {
    // source と destination が同一ならコピー不要（成功扱い）
    // - destination が未作成でも判定できるよう、canonicalize 失敗も考慮する
    match (
        src_zip_path.canonicalize(),
        restore_point_path.canonicalize(),
    ) {
        (Ok(src_canonical), Ok(dest_canonical)) => {
            // 両方存在 → シンボリックリンクも考慮して比較
            if src_canonical == dest_canonical {
                return (true, None);
            }
        }
        (Ok(src_canonical), Err(_)) => {
            // destination が未作成 → parent の canonical + filename で比較
            if let Some(dest_parent) = restore_point_path.parent() {
                if let Ok(dest_parent_canonical) = dest_parent.canonicalize() {
                    if let Some(dest_filename) = restore_point_path.file_name() {
                        let expected_dest = dest_parent_canonical.join(dest_filename);
                        if src_canonical == expected_dest {
                            return (true, None);
                        }
                    }
                }
            }
        }
        _ => {
            // source が存在しない等 → fs::copy に任せてエラーにする
        }
    }

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

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::copy_restore_point_zip;

    #[test]
    fn test_copy_restore_point_zip_success() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src.zip");
        std::fs::write(&src, b"dummy zip bytes").unwrap();

        let restore_dir = tmp.path().join("app_data");
        let restore_path = restore_dir.join("paa_restore_point.zip");

        let (saved, err) = copy_restore_point_zip(&src, &restore_path);
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

        let (saved, err) = copy_restore_point_zip(&src, &restore_path);
        assert!(!saved);
        assert!(err.is_some());
        assert!(!restore_path.exists());
    }

    #[test]
    fn test_copy_restore_point_zip_same_path_succeeds() {
        let tmp = TempDir::new().unwrap();
        let same_file = tmp.path().join("paa_restore_point.zip");
        std::fs::write(&same_file, b"original content").unwrap();

        // 同じパスを src と dest に指定した場合、コピーをスキップして成功を返す
        let (saved, err) = copy_restore_point_zip(&same_file, &same_file);
        assert!(saved);
        assert!(err.is_none());

        // ファイル内容が変更されていないことを確認
        let content = std::fs::read(&same_file).unwrap();
        assert_eq!(content, b"original content");
    }
}
