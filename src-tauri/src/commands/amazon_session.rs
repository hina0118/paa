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

use crate::{plugins::amazon::html_parser, repository::SqliteOrderRepository};

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
/// 取得対象は `htmls` テーブルの Amazon 注文詳細 URL の全レコード。
#[tauri::command]
pub async fn start_amazon_order_fetch(
    app_handle: AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    session_state: tauri::State<'_, AmazonSessionState>,
) -> Result<(), String> {
    let win = app_handle
        .get_webview_window("amazon-session")
        .ok_or("Amazon ウィンドウが開いていません。先にログインしてください。")?;

    session_state.try_start()?;

    let pool_clone = pool.inner().clone();
    let app_clone = app_handle.clone();
    let state_clone = session_state.inner().clone();

    tokio::spawn(async move {
        let result = run_order_fetch_batch(&app_clone, &pool_clone, &win, &state_clone).await;
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
) -> Result<bool, String> {
    // html_content IS NULL = まだ WebView で取得していない URL のみ対象とする。
    // 取得済み（html_content が保存済み）の URL は再アクセス不要のためスキップ。
    let targets: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, url FROM htmls \
         WHERE html_content IS NULL \
           AND (url LIKE 'https://www.amazon.co.jp/your-orders/order-details%' \
             OR url LIKE 'https://www.amazon.co.jp/gp/your-account/order-details%') \
         ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch target htmls: {e}"))?;

    let total = targets.len();
    log::info!(
        "[amazon_session] {} order detail page(s) pending fetch",
        total
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

        // URL から orderID を抽出
        let order_number = match extract_order_id_from_url(&url) {
            Some(id) => id,
            None => {
                log::warn!("[amazon_session] Cannot extract orderID from URL: {}", url);
                continue;
            }
        };

        // WebView で HTML を取得
        let html = match fetch_one_html(app, win, &url).await {
            Ok(h) => h,
            Err(e) => {
                log::warn!("[amazon_session] Failed to fetch {}: {e}", url);
                continue;
            }
        };

        // HTML をパース
        let order_info = match html_parser::parse_order_detail_html(&html, &order_number) {
            Ok(info) => info,
            Err(e) => {
                log::warn!("[amazon_session] Failed to parse {}: {e}", url);
                continue;
            }
        };

        if order_info.items.is_empty() {
            log::warn!(
                "[amazon_session] No items parsed for order {} ({})",
                order_number,
                url
            );
        }

        // トランザクションで DB 更新
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin tx: {e}"))?;

        if let Err(e) = SqliteOrderRepository::save_order_in_tx(
            &mut tx,
            &order_info,
            None,
            Some("amazon.co.jp".to_string()),
            None,
        )
        .await
        {
            log::warn!(
                "[amazon_session] save_order failed for order {} ({}): {e}",
                order_number,
                url
            );
            if let Err(rollback_err) = tx.rollback().await {
                log::error!(
                    "[amazon_session] Failed to rollback tx for {}: {rollback_err}",
                    url
                );
            }
            continue;
        }

        // htmls テーブルを更新（html_content 保存 + analysis_status = completed）
        if let Err(e) = sqlx::query(
            "UPDATE htmls SET html_content = ?, analysis_status = 'completed' WHERE id = ?",
        )
        .bind(&html)
        .bind(html_id)
        .execute(&mut *tx)
        .await
        {
            log::warn!("[amazon_session] Failed to update htmls {}: {e}", url);
        }

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit: {e}"))?;

        log::info!(
            "[amazon_session] Processed order {} ({}/{})",
            order_number,
            i + 1,
            total
        );
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

/// URL の `orderID` クエリパラメータを取り出す
fn extract_order_id_from_url(url: &str) -> Option<String> {
    // 簡易クエリパース: "orderID=XXX-XXXXXXX-XXXXXXX"
    url.split('?').nth(1)?.split('&').find_map(|param| {
        let (key, value) = param.split_once('=')?;
        if key == "orderID" {
            Some(value.to_string())
        } else {
            None
        }
    })
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
    fn test_extract_order_id_from_url() {
        let url = "https://www.amazon.co.jp/your-orders/order-details?orderID=123-4567890-1234567";
        assert_eq!(
            extract_order_id_from_url(url),
            Some("123-4567890-1234567".to_string())
        );
    }

    #[test]
    fn test_extract_order_id_with_extra_params() {
        let url = "https://www.amazon.co.jp/your-orders/order-details?ref=ppx_yo_dt_b_order_details_fullpage&orderID=234-5678901-2345678";
        assert_eq!(
            extract_order_id_from_url(url),
            Some("234-5678901-2345678".to_string())
        );
    }

    #[test]
    fn test_extract_order_id_missing() {
        assert_eq!(
            extract_order_id_from_url("https://www.amazon.co.jp/your-orders/orders"),
            None
        );
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
