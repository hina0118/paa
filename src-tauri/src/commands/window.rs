use crate::config;
use tauri::Manager;

/// ウィンドウサイズのバリデーション（最小200、最大10000）
pub fn validate_window_size(width: i64, height: i64) -> Result<(), String> {
    const MIN_SIZE: i64 = 200;
    const MAX_SIZE: i64 = 10000;

    if !(MIN_SIZE..=MAX_SIZE).contains(&width) {
        return Err(format!(
            "ウィンドウの幅は{MIN_SIZE}〜{MAX_SIZE}の範囲である必要があります"
        ));
    }
    if !(MIN_SIZE..=MAX_SIZE).contains(&height) {
        return Err(format!(
            "ウィンドウの高さは{MIN_SIZE}〜{MAX_SIZE}の範囲である必要があります"
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn save_window_settings(
    app_handle: tauri::AppHandle,
    width: i64,
    height: i64,
    x: Option<i64>,
    y: Option<i64>,
    maximized: bool,
) -> Result<(), String> {
    validate_window_size(width, height)?;

    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.window = config::WindowConfig {
        width,
        height,
        x,
        y,
        maximized,
    };
    config::save(&app_config_dir, &config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_window_size_valid() {
        assert!(validate_window_size(200, 200).is_ok());
        assert!(validate_window_size(1000, 800).is_ok());
        assert!(validate_window_size(10000, 10000).is_ok());
    }

    #[test]
    fn test_validate_window_size_width_too_small() {
        let result = validate_window_size(199, 500);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("幅"));
    }

    #[test]
    fn test_validate_window_size_width_too_large() {
        let result = validate_window_size(10001, 500);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("幅"));
    }

    #[test]
    fn test_validate_window_size_height_too_small() {
        let result = validate_window_size(500, 199);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("高さ"));
    }

    #[test]
    fn test_validate_window_size_height_too_large() {
        let result = validate_window_size(500, 10001);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("高さ"));
    }
}
