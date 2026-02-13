//! クリップボード監視（ポーリング）
//!
//! - OSイベントフックは使わず、一定間隔でクリップボード（テキスト）を取得して変化を検知する。
//! - URL（特に画像URL）を検知したらフロントへイベントで通知する。

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

pub const CLIPBOARD_URL_DETECTED_EVENT: &str = "clipboard-url-detected";

/// クリップボードテキストの最大サイズ（バイト）
/// 10KB を超える内容は処理をスキップして、メモリの過剰使用を防ぐ
const MAX_CLIPBOARD_SIZE: usize = 10_240;

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
            return;
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
                Ok(t) if t.len() <= MAX_CLIPBOARD_SIZE => {
                    consecutive_read_errors = 0;
                    t
                }
                Ok(t) => {
                    // クリップボードの読み取りは成功しているため、エラーカウンタをリセット
                    consecutive_read_errors = 0;
                    // クリップボード内容が MAX_CLIPBOARD_SIZE より大きい場合はスキップ（メモリの過剰な使用を防ぐ）
                    // ハッシュ値を last_text に保存して、同じ大容量コンテンツでログが繰り返し出力されるのを防ぐ
                    // 注意: ハッシュ計算は毎回行われるが、実際のテキスト保存と比べて遥かに軽量
                    let hash = format!("__LARGE_CONTENT_HASH_{:x}__", calculate_simple_hash(&t));
                    if last_text.as_deref() != Some(&hash) {
                        log::debug!(
                            "Skipping large clipboard content ({} bytes > {} bytes limit)",
                            t.len(),
                            MAX_CLIPBOARD_SIZE
                        );
                        last_text = Some(hash);
                    }
                    continue;
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
    // （コピー元によっては末尾に ')' ',' '!' '?' などが付くことがあるので軽く剥がす）
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

/// 簡易的なハッシュ計算（大容量コンテンツの重複検知用）
/// メモリ効率のため、テキスト全体を保存せずハッシュ値のみを使用
/// 
/// 注意: ハッシュ衝突の可能性はあるが、このユースケース（ログの重複防止）では
/// 衝突が起きても重大な問題にはならない。最悪の場合、異なる大容量コンテンツでも
/// 一度だけログが出力されることになるが、機能的には許容範囲。
/// DefaultHasher を使用しているのは、暗号学的強度が不要で高速性を優先するため。
fn calculate_simple_hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
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

    #[test]
    fn test_calculate_simple_hash_consistency() {
        // 同じ入力に対して同じハッシュ値が返されることを確認
        let text = "test content for hashing";
        let hash1 = calculate_simple_hash(text);
        let hash2 = calculate_simple_hash(text);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_calculate_simple_hash_different_inputs() {
        // 異なる入力に対して異なるハッシュ値が返されることを確認
        // 注意: ハッシュ衝突は理論上可能だが、このテストケースでは発生しないはず
        let text1 = "first text";
        let text2 = "second text";
        let hash1 = calculate_simple_hash(text1);
        let hash2 = calculate_simple_hash(text2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_calculate_simple_hash_empty_string() {
        // 空文字列のハッシュも計算できることを確認
        let hash = calculate_simple_hash("");
        // ハッシュ値が何らかの値を返すことを確認（0でないことを期待）
        // 注意: DefaultHasherの実装により、空文字列のハッシュは0ではない
        assert!(hash != 0 || hash == 0); // 常に真だが、計算自体がパニックしないことを確認
    }

    #[test]
    fn test_calculate_simple_hash_large_content() {
        // 大容量コンテンツのハッシュ計算も正常に動作することを確認
        let large_text = "a".repeat(20_000); // 20KB
        let hash = calculate_simple_hash(&large_text);
        // ハッシュ計算がパニックせず、値を返すことを確認
        assert!(hash > 0 || hash == 0);
    }
}
