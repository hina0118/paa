//! 駿河屋マーケットプレイス マイページ HTML 取得コマンド
//!
//! WebView ウィンドウを通じてログイン済みセッションを利用し、
//! マイページ HTML を取得・パースして注文データを補完する。
//!
//! # 使用フロー
//! 1. `open_surugaya_login_window` でログインウィンドウを開く
//! 2. ユーザーが suruga-ya.jp でログイン
//! 3. `start_surugaya_mypage_fetch` でバッチ取得を開始
//!
//! # 権限設定
//! `capabilities/surugaya-session.json` で suruga-ya.jp ドメインからの
//! `core:event:allow-emit` を許可している。
//! `on_page_load` で eval() した JS から `window.__TAURI__.event.emit()` を
//! 呼び出し、HTML を Rust 側に渡す。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};


// ─────────────────────────────────────────────────────────────────────────────
// 状態管理
// ─────────────────────────────────────────────────────────────────────────────

/// マイページ取得バッチの実行状態（`BatchRunState` の薄いラッパー）
#[derive(Clone, Default)]
pub struct SurugayaSessionState(crate::BatchRunState);

impl SurugayaSessionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn try_start(&self) -> Result<(), String> {
        self.0
            .try_start()
            .map_err(|_| "マイページ取得は既に実行中です。".to_string())
    }

    pub(crate) fn finish(&self) {
        self.0.finish();
    }

    fn request_cancel(&self) {
        self.0.request_cancel();
    }

    fn should_cancel(&self) -> bool {
        self.0.should_cancel()
    }

    /// 実行中かどうかを返す（`get_surugaya_mypage_fetch_status` コマンド用）
    fn is_running(&self) -> bool {
        self.0.is_running()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri コマンド
// ─────────────────────────────────────────────────────────────────────────────

/// 駿河屋ログインウィンドウを開く
///
/// 既存ウィンドウがある場合はフォーカスする。
/// ウィンドウ内で suruga-ya.jp にログイン後、`start_surugaya_mypage_fetch` を呼ぶ。
#[tauri::command]
pub async fn open_surugaya_login_window(app_handle: AppHandle) -> Result<(), String> {
    const WINDOW_LABEL: &str = "surugaya-session";

    // 既存ウィンドウがあればフォーカスして終了
    if let Some(win) = app_handle.get_webview_window(WINDOW_LABEL) {
        win.show().ok();
        win.set_focus().ok();
        return Ok(());
    }

    let url = WebviewUrl::External(
        "https://www.suruga-ya.jp/mypage/login"
            .parse()
            .map_err(|e: url::ParseError| e.to_string())?,
    );

    // on_page_load: マイページ詳細ページのロード完了時に HTML を Tauri イベントで送信
    WebviewWindowBuilder::new(&app_handle, WINDOW_LABEL, url)
        .title("駿河屋 マイページ")
        .inner_size(1024.0, 768.0)
        .on_page_load(|window, payload| {
            use tauri::webview::PageLoadEvent;
            if !matches!(payload.event(), PageLoadEvent::Finished) {
                return;
            }
            let url_str = payload.url().as_str();
            if !url_str.contains("pcmypage/action_sell_search/detail") {
                return;
            }
            // マイページ詳細ページ: HTML を取得して Tauri イベントで Rust 側に送信
            // capabilities/surugaya-session.json の core:event:allow-emit により許可済み
            let _ = window.eval(concat!(
                "(function(){",
                "try{",
                "window.__TAURI__.event.emit(",
                "'surugaya:html_ready',",
                "document.documentElement.outerHTML",
                ");",
                "}catch(e){",
                "console.error('[PAA] surugaya html emit error:',e);",
                "}",
                "})()"
            ));
        })
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 駿河屋マイページ取得バッチを開始
///
/// `surugaya-session` ウィンドウが開いていない（未ログイン）場合はエラーを返す。
/// `force_refetch = true` の場合は取得済み HTML も含めて全件再取得する。
#[tauri::command]
pub async fn start_surugaya_mypage_fetch(
    app_handle: AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    session_state: tauri::State<'_, SurugayaSessionState>,
    force_refetch: Option<bool>,
) -> Result<(), String> {
    let win = app_handle
        .get_webview_window("surugaya-session")
        .ok_or("駿河屋ウィンドウが開いていません。先にログインしてください。")?;

    session_state.try_start()?;

    let pool_clone = pool.inner().clone();
    let app_clone = app_handle.clone();
    let state_clone = session_state.inner().clone();
    let force_refetch = force_refetch.unwrap_or(false);

    tokio::spawn(async move {
        let result =
            run_mypage_batch(&app_clone, &pool_clone, &win, &state_clone, force_refetch).await;
        state_clone.finish();

        #[derive(serde::Serialize, Clone)]
        struct FetchCompletePayload {
            cancelled: bool,
            error: Option<String>,
        }

        let payload = match result {
            Ok(cancelled) => {
                if cancelled {
                    log::info!("[surugaya_session] Batch cancelled by user");
                }
                FetchCompletePayload {
                    cancelled,
                    error: None,
                }
            }
            Err(e) => {
                log::error!("[surugaya_session] Batch failed: {e}");
                FetchCompletePayload {
                    cancelled: false,
                    error: Some(e),
                }
            }
        };
        let _ = app_clone.emit("surugaya:fetch_complete", payload);
    });

    Ok(())
}

/// 駿河屋マイページ取得バッチをキャンセル
#[tauri::command]
pub async fn cancel_surugaya_mypage_fetch(
    session_state: tauri::State<'_, SurugayaSessionState>,
) -> Result<(), String> {
    session_state.request_cancel();
    Ok(())
}

/// 駿河屋マイページ取得バッチの実行状態を返す
#[tauri::command]
pub async fn get_surugaya_mypage_fetch_status(
    session_state: tauri::State<'_, SurugayaSessionState>,
) -> Result<bool, String> {
    Ok(session_state.is_running())
}

// ─────────────────────────────────────────────────────────────────────────────
// バッチ実行ロジック
// ─────────────────────────────────────────────────────────────────────────────

/// バッチ実行結果。`Ok(true)` = ユーザーによるキャンセル、`Ok(false)` = 正常完了。
///
/// パイプラインステップ（`orchestration::pipeline_steps::run_surugaya_step`）から
/// も呼び出されるため `pub(crate)` としている。
pub(crate) async fn run_mypage_batch(
    app: &AppHandle,
    pool: &SqlitePool,
    win: &tauri::WebviewWindow,
    state: &SurugayaSessionState,
    force_refetch: bool,
) -> Result<bool, String> {
    // force_refetch = true: 取得済みを含む全件を対象とする（HTML 更新時に使用）
    // force_refetch = false: html_content IS NULL のみ（差分取得・デフォルト）
    let sql = if force_refetch {
        "SELECT id, url FROM htmls \
         WHERE url LIKE 'https://www.suruga-ya.jp/pcmypage/%' \
         ORDER BY id"
    } else {
        "SELECT id, url FROM htmls \
         WHERE html_content IS NULL \
           AND url LIKE 'https://www.suruga-ya.jp/pcmypage/%' \
         ORDER BY id"
    };

    let targets: Vec<(i64, String)> = sqlx::query_as(sql)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to fetch target htmls: {e}"))?;

    let total = targets.len();
    log::info!(
        "[surugaya_session] {} mypage(s) to fetch (force_refetch={})",
        total,
        force_refetch
    );

    for (i, (html_id, url)) in targets.into_iter().enumerate() {
        if state.should_cancel() {
            log::info!("[surugaya_session] Cancelled at {}/{}", i, total);
            return Ok(true);
        }

        let _ = app.emit(
            "surugaya:fetch_progress",
            serde_json::json!({ "current": i + 1, "total": total, "url": &url }),
        );

        let html = match fetch_one_html(app, win, &url).await {
            Ok(h) => h,
            Err(e) => {
                log::warn!("[surugaya_session] Failed to fetch {}: {e}", url);
                continue;
            }
        };

        // htmls テーブルに html_content を保存（パースは run_batch_parse_task で行う）
        if let Err(e) = sqlx::query("UPDATE htmls SET html_content = ? WHERE id = ?")
            .bind(&html)
            .bind(html_id)
            .execute(pool)
            .await
        {
            log::warn!(
                "[surugaya_session] Failed to save html_content for {}: {e}",
                url
            );
            continue;
        }

        log::info!("[surugaya_session] Fetched HTML ({}/{})", i + 1, total);
    }

    Ok(false)
}

/// WebView を指定 URL にナビゲートし、マイページ HTML を受け取る
///
/// `on_page_load` の eval() → `window.__TAURI__.event.emit()` → この関数の
/// `app.once("surugaya:html_ready", ...)` というパスで HTML を受信する。
async fn fetch_one_html(
    app: &AppHandle,
    win: &tauri::WebviewWindow,
    url: &str,
) -> Result<String, String> {
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let tx_arc = Arc::new(Mutex::new(Some(tx)));
    let tx_clone = tx_arc.clone();

    // surugaya:html_ready イベントを一回だけ受け取る
    let event_id = app.once("surugaya:html_ready", move |event| {
        // JS から送られる payload は JSON エンコードされた文字列
        let html = serde_json::from_str::<String>(event.payload())
            .unwrap_or_else(|_| event.payload().to_string());
        if let Ok(mut guard) = tx_clone.lock() {
            if let Some(sender) = guard.take() {
                let _ = sender.send(html);
            }
        }
    });

    // WebView をナビゲート
    let parsed_url: tauri::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
    win.navigate(parsed_url).map_err(|e| e.to_string())?;

    // HTML の受信を最大 30 秒待機
    match tokio::time::timeout(Duration::from_secs(30), rx).await {
        Ok(Ok(html)) => Ok(html),
        Ok(Err(_)) => {
            app.unlisten(event_id);
            Err("HTML 取得チャネルが閉じました".to_string())
        }
        Err(_) => {
            app.unlisten(event_id);
            Err("マイページ取得タイムアウト（30秒）".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surugaya_session_state_default() {
        let state = SurugayaSessionState::default();
        assert!(!state.should_cancel());
    }

    #[test]
    fn test_try_start_and_finish() {
        let state = SurugayaSessionState::new();
        assert!(state.try_start().is_ok());
        assert!(state.try_start().is_err()); // 二重起動はエラー
        state.finish();
        assert!(state.try_start().is_ok()); // finish 後は再起動可能
    }

    #[test]
    fn test_cancel_flag() {
        let state = SurugayaSessionState::new();
        assert!(!state.should_cancel());
        state.request_cancel();
        assert!(state.should_cancel());
        state.finish(); // finish でキャンセルフラグもリセット
        assert!(!state.should_cancel());
    }
}
