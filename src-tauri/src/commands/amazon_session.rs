//! Amazon.co.jp 注文詳細ページ HTML 取得コマンド
//!
//! WebView ウィンドウを通じてログイン済みセッションを利用し、
//! 注文詳細ページ HTML を取得・パースして注文データを補完する。
//!
//! # 使用フロー
//! 1. `open_amazon_login_window` でログインウィンドウを開く
//! 2. ユーザーが Amazon.co.jp にログイン
//! 3. `start_amazon_order_fetch` でバッチ取得を開始
//!
//! # 権限設定
//! `capabilities/amazon-session.json` で amazon.co.jp ドメインからの
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

/// 注文詳細取得バッチの実行状態（`BatchRunState` の薄いラッパー）
#[derive(Clone, Default)]
pub struct AmazonSessionState(crate::BatchRunState);

impl AmazonSessionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn try_start(&self) -> Result<(), String> {
        self.0
            .try_start()
            .map_err(|_| "Amazon 注文取得は既に実行中です。".to_string())
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

    fn is_running(&self) -> bool {
        self.0.is_running()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri コマンド
// ─────────────────────────────────────────────────────────────────────────────

/// Amazon ログインウィンドウを開く
///
/// 既存ウィンドウがある場合はフォーカスする。
/// ウィンドウ内で Amazon.co.jp にログイン後、`start_amazon_order_fetch` を呼ぶ。
#[tauri::command]
pub async fn open_amazon_login_window(app_handle: AppHandle) -> Result<(), String> {
    const WINDOW_LABEL: &str = "amazon-session";

    if let Some(win) = app_handle.get_webview_window(WINDOW_LABEL) {
        win.show().ok();
        win.set_focus().ok();
        return Ok(());
    }

    // 注文一覧ページを開く（未ログインの場合はサインインページにリダイレクトされる）
    let url = WebviewUrl::External(
        "https://www.amazon.co.jp/your-orders/orders"
            .parse()
            .map_err(|e: url::ParseError| e.to_string())?,
    );

    WebviewWindowBuilder::new(&app_handle, WINDOW_LABEL, url)
        .title("Amazon.co.jp 注文詳細")
        .inner_size(1280.0, 900.0)
        .on_page_load(|window, payload| {
            use tauri::webview::PageLoadEvent;
            if !matches!(payload.event(), PageLoadEvent::Finished) {
                return;
            }
            let url_str = payload.url().as_str();
            // 注文詳細ページ（orderID パラメータを含む URL）のみ HTML を送信
            if !is_order_detail_url(url_str) {
                return;
            }
            let _ = window.eval(concat!(
                "(function(){",
                "try{",
                "window.__TAURI__.event.emit(",
                "'amazon:html_ready',",
                "document.documentElement.outerHTML",
                ");",
                "}catch(e){",
                "console.error('[PAA] amazon html emit error:',e);",
                "}",
                "})()"
            ));
        })
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Amazon 注文詳細取得バッチを開始
///
/// `amazon-session` ウィンドウが開いていない（未ログイン）場合はエラーを返す。
/// `force_refetch = true` の場合は取得済み HTML も含めて全件再取得する。
#[tauri::command]
pub async fn start_amazon_order_fetch(
    app_handle: AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    session_state: tauri::State<'_, AmazonSessionState>,
    force_refetch: Option<bool>,
) -> Result<(), String> {
    let win = app_handle
        .get_webview_window("amazon-session")
        .ok_or("Amazon ウィンドウが開いていません。先にログインしてください。")?;

    session_state.try_start()?;

    let pool_clone = pool.inner().clone();
    let app_clone = app_handle.clone();
    let state_clone = session_state.inner().clone();
    let force_refetch = force_refetch.unwrap_or(false);

    tokio::spawn(async move {
        let result =
            run_order_fetch_batch(&app_clone, &pool_clone, &win, &state_clone, force_refetch)
                .await;
        state_clone.finish();

        #[derive(serde::Serialize, Clone)]
        struct FetchCompletePayload {
            cancelled: bool,
            error: Option<String>,
        }

        let payload = match result {
            Ok(cancelled) => {
                if cancelled {
                    log::info!("[amazon_session] Batch cancelled by user");
                }
                FetchCompletePayload {
                    cancelled,
                    error: None,
                }
            }
            Err(e) => {
                log::error!("[amazon_session] Batch failed: {e}");
                FetchCompletePayload {
                    cancelled: false,
                    error: Some(e),
                }
            }
        };
        let _ = app_clone.emit("amazon:fetch_complete", payload);
    });

    Ok(())
}

/// Amazon 注文詳細取得バッチをキャンセル
#[tauri::command]
pub async fn cancel_amazon_order_fetch(
    session_state: tauri::State<'_, AmazonSessionState>,
) -> Result<(), String> {
    session_state.request_cancel();
    Ok(())
}

/// Amazon 注文詳細取得バッチの実行状態を返す
#[tauri::command]
pub async fn get_amazon_order_fetch_status(
    session_state: tauri::State<'_, AmazonSessionState>,
) -> Result<bool, String> {
    Ok(session_state.is_running())
}

// ─────────────────────────────────────────────────────────────────────────────
// バッチ実行ロジック
// ─────────────────────────────────────────────────────────────────────────────

/// バッチ実行結果。`Ok(true)` = ユーザーによるキャンセル、`Ok(false)` = 正常完了。
pub(crate) async fn run_order_fetch_batch(
    app: &AppHandle,
    pool: &SqlitePool,
    win: &tauri::WebviewWindow,
    state: &AmazonSessionState,
    force_refetch: bool,
) -> Result<bool, String> {
    // force_refetch = true: 取得済みを含む全件を対象とする（HTML 更新時に使用）
    // force_refetch = false: html_content IS NULL のみ（差分取得・デフォルト）
    let sql = if force_refetch {
        "SELECT id, url FROM htmls \
         WHERE (url LIKE 'https://www.amazon.co.jp/your-orders/order-details%' \
             OR url LIKE 'https://www.amazon.co.jp/gp/your-account/order-details%') \
         ORDER BY id"
    } else {
        "SELECT id, url FROM htmls \
         WHERE html_content IS NULL \
           AND (url LIKE 'https://www.amazon.co.jp/your-orders/order-details%' \
             OR url LIKE 'https://www.amazon.co.jp/gp/your-account/order-details%') \
         ORDER BY id"
    };

    let targets: Vec<(i64, String)> = sqlx::query_as(sql)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to fetch target htmls: {e}"))?;

    let total = targets.len();
    log::info!(
        "[amazon_session] {} order detail page(s) to fetch (force_refetch={})",
        total,
        force_refetch
    );

    for (i, (html_id, url)) in targets.into_iter().enumerate() {
        if state.should_cancel() {
            log::info!("[amazon_session] Cancelled at {}/{}", i, total);
            return Ok(true);
        }

        let _ = app.emit(
            "amazon:fetch_progress",
            serde_json::json!({ "current": i + 1, "total": total, "url": &url }),
        );

        // WebView で HTML を取得
        let html = match fetch_one_html(app, win, &url).await {
            Ok(h) => h,
            Err(e) => {
                log::warn!("[amazon_session] Failed to fetch {}: {e}", url);
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
                "[amazon_session] Failed to save html_content for {}: {e}",
                url
            );
            continue;
        }

        log::info!("[amazon_session] Fetched HTML ({}/{})", i + 1, total);
    }

    Ok(false)
}

/// WebView を指定 URL にナビゲートし、注文詳細 HTML を受け取る
async fn fetch_one_html(
    app: &AppHandle,
    win: &tauri::WebviewWindow,
    url: &str,
) -> Result<String, String> {
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    let tx_arc = Arc::new(Mutex::new(Some(tx)));
    let tx_clone = tx_arc.clone();

    let event_id = app.once("amazon:html_ready", move |event| {
        let html = serde_json::from_str::<String>(event.payload())
            .unwrap_or_else(|_| event.payload().to_string());
        if let Ok(mut guard) = tx_clone.lock() {
            if let Some(sender) = guard.take() {
                let _ = sender.send(html);
            }
        }
    });

    let parsed_url: tauri::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
    win.navigate(parsed_url).map_err(|e| e.to_string())?;

    match tokio::time::timeout(Duration::from_secs(30), rx).await {
        Ok(Ok(html)) => Ok(html),
        Ok(Err(_)) => {
            app.unlisten(event_id);
            Err("HTML 取得チャネルが閉じました".to_string())
        }
        Err(_) => {
            app.unlisten(event_id);
            Err("注文詳細取得タイムアウト（30秒）".to_string())
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// URL ヘルパー
// ─────────────────────────────────────────────────────────────────────────────

/// Amazon 注文詳細ページの URL かどうかを判定する
fn is_order_detail_url(url: &str) -> bool {
    (url.contains("your-orders/order-details") || url.contains("gp/your-account/order-details"))
        && url.contains("orderID")
}


// ─────────────────────────────────────────────────────────────────────────────
// テスト
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_order_detail_url_your_orders() {
        assert!(is_order_detail_url(
            "https://www.amazon.co.jp/your-orders/order-details?orderID=123-4567890-1234567"
        ));
    }

    #[test]
    fn test_is_order_detail_url_gp() {
        assert!(is_order_detail_url(
            "https://www.amazon.co.jp/gp/your-account/order-details?orderID=111-2222222-3333333"
        ));
    }

    #[test]
    fn test_is_order_detail_url_false_orders_list() {
        assert!(!is_order_detail_url(
            "https://www.amazon.co.jp/your-orders/orders"
        ));
    }

    #[test]
    fn test_is_order_detail_url_false_product() {
        assert!(!is_order_detail_url(
            "https://www.amazon.co.jp/dp/B07XYZ1234"
        ));
    }

    #[test]
    fn test_amazon_session_state_default() {
        let state = AmazonSessionState::default();
        assert!(!state.should_cancel());
        assert!(!state.is_running());
    }

    #[test]
    fn test_try_start_and_finish() {
        let state = AmazonSessionState::new();
        assert!(state.try_start().is_ok());
        assert!(state.try_start().is_err()); // 二重起動はエラー
        state.finish();
        assert!(state.try_start().is_ok()); // finish 後は再起動可能
    }

    #[test]
    fn test_cancel_flag() {
        let state = AmazonSessionState::new();
        assert!(!state.should_cancel());
        state.request_cancel();
        assert!(state.should_cancel());
        state.finish();
        assert!(!state.should_cancel());
    }
}
