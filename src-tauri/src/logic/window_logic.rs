//! ウィンドウ設定関連のビジネスロジック
//!
//! Tauriコマンドからウィンドウ設定の取得・保存ロジックを分離します。

use crate::repository::WindowSettingsRepository;

/// ウィンドウ設定を取得する
pub async fn get_window_settings<R>(repo: &R) -> Result<WindowSettings, String>
where
    R: WindowSettingsRepository,
{
    repo.get_window_settings().await
}

/// ウィンドウ設定を保存する（バリデーション含む）
pub async fn save_window_settings<R>(
    repo: &R,
    width: i64,
    height: i64,
    x: Option<i64>,
    y: Option<i64>,
    maximized: bool,
) -> Result<(), String>
where
    R: WindowSettingsRepository,
{
    // Validate window size (minimum 200, maximum 10000)
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

    repo.save_window_settings(width, height, x, y, maximized)
        .await
}

/// ウィンドウ設定の構造体（lib.rsから移動）
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct WindowSettings {
    pub width: i64,
    pub height: i64,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub maximized: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::MockWindowSettingsRepository;

    #[tokio::test]
    async fn test_get_window_settings_delegates_to_repo() {
        let mut mock = MockWindowSettingsRepository::new();
        let expected = WindowSettings {
            width: 800,
            height: 600,
            x: Some(100),
            y: Some(200),
            maximized: false,
        };

        mock.expect_get_window_settings()
            .returning(move || Ok(expected.clone()));

        let result = get_window_settings(&mock).await.unwrap();
        assert_eq!(result.width, 800);
        assert_eq!(result.height, 600);
        assert_eq!(result.x, Some(100));
        assert_eq!(result.y, Some(200));
        assert!(!result.maximized);
    }

    #[tokio::test]
    async fn test_save_window_settings_rejects_too_small() {
        let mock = MockWindowSettingsRepository::new();

        let result = save_window_settings(&mock, 100, 300, None, None, false).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("ウィンドウの幅は"));
    }

    #[tokio::test]
    async fn test_save_window_settings_rejects_too_large() {
        let mock = MockWindowSettingsRepository::new();

        let result = save_window_settings(&mock, 800, 20000, None, None, false).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("ウィンドウの高さは"));
    }

    #[tokio::test]
    async fn test_save_window_settings_accepts_valid_range() {
        let mut mock = MockWindowSettingsRepository::new();
        mock.expect_save_window_settings()
            .withf(|w, h, _x, _y, max| *w == 800 && *h == 600 && !*max)
            .returning(|_, _, _, _, _| Ok(()));

        let result = save_window_settings(&mock, 800, 600, Some(0), Some(0), false).await;
        assert!(result.is_ok());
    }
}
