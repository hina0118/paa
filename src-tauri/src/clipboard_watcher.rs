//! クリップボード監視（ポーリング）
//!
//! - OSイベントフックは使わず、一定間隔でクリップボード（テキスト）を取得して変化を検知する。
//! - URL（特に画像URL）を検知したらフロントへイベントで通知する。

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

pub const CLIPBOARD_URL_DETECTED_EVENT: &str = "clipboard-url-detected";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardUrlDetectedPayload {
    pub url: String,
    pub kind: ClipboardDetectedKind,
    pub source: String,
    pub detected_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClipboardDetectedKind {
    ImageUrl,
    Url,
}

#[derive(Debug, Clone)]
pub struct WatcherConfig {
    pub poll_interval_ms: u64,
    pub emit_non_image_url: bool,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            // 800ms: CPU使用率とレスポンス性のバランスを考慮した間隔
            // 経験的に、500ms以下ではCPU使用率が顕著に上がり、
            // 1000ms以上ではクリップボードコピー後の検知遅延が体感的に遅く感じられる。
            // 800msは両者のバランスが取れた妥当なデフォルト値。
            poll_interval_ms: 800,
            emit_non_image_url: false,
        }
    }
}

/// クリップボード監視を開始する（ブロッキング）。
///
/// - `AppHandle` を受け取り、検知時にイベントでフロントへ通知する。
/// - 例外が発生してもクラッシュせず継続する。
/// - `shutdown_signal` が true になると監視ループを終了する。
pub fn run_clipboard_watcher(
    app: tauri::AppHandle,
    config: WatcherConfig,
    shutdown_signal: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    let mut last_text: Option<String> = None;
    let mut consecutive_read_errors: u32 = 0;

    loop {
        // シャットダウンシグナルをチェック
        if shutdown_signal.load(std::sync::atomic::Ordering::Relaxed) {
            log::info!("Clipboard watcher received shutdown signal, exiting");
            break;
        }

        // Clipboard の初期化が失敗することがあるため、リトライ前提で外側ループにする
        let mut clipboard = match arboard::Clipboard::new() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to initialize clipboard: {}", e);
                std::thread::sleep(std::time::Duration::from_millis(config.poll_interval_ms));
                continue;
            }
        };

        loop {
            // 内側ループでもシャットダウンチェック
            if shutdown_signal.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }

            std::thread::sleep(std::time::Duration::from_millis(config.poll_interval_ms));

            let text = match clipboard.get_text() {
                Ok(t) => {
                    consecutive_read_errors = 0;
                    t
                }
                Err(_e) => {
                    // クリップボードがロックされている等で失敗することがあるため、ログはdebugに留める
                    //（頻繁に起きるとノイズになる）
                    consecutive_read_errors = consecutive_read_errors.saturating_add(1);
                    log::debug!("Failed to read clipboard text");
                    if consecutive_read_errors >= 10 {
                        // 連続失敗が続く場合は Clipboard を作り直す
                        consecutive_read_errors = 0;
                        break;
                    }
                    continue;
                }
            };

            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }

            // 変化がない場合は無視
            if last_text.as_deref() == Some(trimmed) {
                continue;
            }
            last_text = Some(trimmed.to_string());

            // URL抽出（最初のURLのみ）
            let Some(url) = extract_first_url(trimmed) else {
                continue;
            };

            let kind = if is_image_url(&url) {
                ClipboardDetectedKind::ImageUrl
            } else {
                ClipboardDetectedKind::Url
            };

            if matches!(kind, ClipboardDetectedKind::Url) && !config.emit_non_image_url {
                continue;
            }

            let payload = ClipboardUrlDetectedPayload {
                url,
                kind,
                source: "clipboard".to_string(),
                detected_at: chrono::Utc::now().to_rfc3339(),
            };

            if let Err(e) = app.emit(CLIPBOARD_URL_DETECTED_EVENT, payload) {
                log::debug!("Failed to emit clipboard event: {}", e);
            }
        }
    }
}

static URL_REGEX: Lazy<regex::Regex> = Lazy::new(|| {
    // フロントエンド（image-search-dialog）は HTTPS のみ受け付けるため、ここでも HTTPS の URL のみに限定する
    regex::Regex::new(r"https://\S+").expect(
        "Failed to compile URL regex pattern - this is a static pattern and should never fail",
    )
});

fn extract_first_url(text: &str) -> Option<String> {
    // URLは最小限で: 空白/改行で区切られている想定
    // （コピー元によっては末尾に ')' ',' '!' などが付くことがあるので軽く剥がす）
    let m = URL_REGEX.find(text)?;
    let mut s = m.as_str().to_string();
    while s.ends_with([')', ']', '}', '>', ',', '.', ';', '"', '\'', '!', '?']) {
        s.pop();
    }
    Some(s)
}

fn is_image_url(url: &str) -> bool {
    // まずはパースできるURLのみ対象
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    let path = parsed.path().to_ascii_lowercase();
    path.ends_with(".jpg")
        || path.ends_with(".jpeg")
        || path.ends_with(".png")
        || path.ends_with(".webp")
        || path.ends_with(".gif")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_first_url_basic() {
        assert_eq!(
            extract_first_url("https://example.com/a.jpg").as_deref(),
            Some("https://example.com/a.jpg")
        );
        assert_eq!(
            extract_first_url("foo https://example.com/a.png bar").as_deref(),
            Some("https://example.com/a.png")
        );
        assert_eq!(extract_first_url("no url here"), None);
    }

    #[test]
    fn test_extract_first_url_strips_trailing_punct() {
        assert_eq!(
            extract_first_url("(https://example.com/a.jpg)").as_deref(),
            Some("https://example.com/a.jpg")
        );
        assert_eq!(
            extract_first_url("https://example.com/a.jpg,").as_deref(),
            Some("https://example.com/a.jpg")
        );
        // 感嘆符などの記号も末尾から除去される
        assert_eq!(
            extract_first_url("https://example.com/a.jpg!").as_deref(),
            Some("https://example.com/a.jpg")
        );
        assert_eq!(
            extract_first_url("Check out https://example.com/image.png!").as_deref(),
            Some("https://example.com/image.png")
        );
    }

    #[test]
    fn test_is_image_url() {
        assert!(is_image_url("https://example.com/a.JPG"));
        assert!(is_image_url("https://example.com/a.jpeg"));
        assert!(is_image_url("https://example.com/a.png?x=1"));
        assert!(!is_image_url("https://example.com/a.svg"));
        assert!(!is_image_url("not a url"));
    }

    #[test]
    fn test_extract_first_url_rejects_http() {
        // HTTPのURLは検出しない（HTTPSのみ受け付ける）
        assert_eq!(extract_first_url("http://example.com/a.jpg"), None);
        assert_eq!(extract_first_url("foo http://example.com/a.png bar"), None);
        assert_eq!(
            extract_first_url("Check out http://example.com/image.jpg!"),
            None
        );
    }

    #[test]
    fn test_extract_first_url_accepts_https_only() {
        // HTTPSのURLは正しく検出される
        assert_eq!(
            extract_first_url("https://example.com/a.jpg").as_deref(),
            Some("https://example.com/a.jpg")
        );
        // HTTP と HTTPS が混在している場合は HTTPS のみ
        assert_eq!(
            extract_first_url("http://bad.com/x.jpg https://good.com/y.jpg").as_deref(),
            Some("https://good.com/y.jpg")
        );
    }
}
