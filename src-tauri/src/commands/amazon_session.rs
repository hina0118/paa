//! Amazon.co.jp 注文詳細ページ HTML 取得コマンド
//!
//! WebView ウィンドウを通じてログイン済みセッションを利用し、
//! 注文詳細ページ HTML を取得・パースして注文データを補完する。
//! 取得処理の本体は [`super::webview_session`] で駿河屋と共通化している。
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

use sqlx::SqlitePool;
use tauri::AppHandle;

use super::webview_session::WebviewSession;

// ─────────────────────────────────────────────────────────────────────────────
// 状態管理・セッション設定
// ─────────────────────────────────────────────────────────────────────────────

crate::batch_run_state::define_batch_state!(
    /// 注文詳細取得バッチの実行状態（`BatchRunState` の薄いラッパー）
    AmazonSessionState,
    "Amazon 注文取得は既に実行中です。"
);

static AMAZON_SESSION: WebviewSession = WebviewSession {
    log_tag: "amazon_session",
    window_label: "amazon-session",
    window_title: "Amazon.co.jp 注文詳細",
    window_size: (1280.0, 900.0),
    // 注文一覧ページを開く（未ログインの場合はサインインページにリダイレクトされる）
    start_url: "https://www.amazon.co.jp/your-orders/orders",
    // 注文詳細ページ（orderID パラメータを含む URL）のみ HTML を送信
    is_target_page: is_order_detail_url,
    emit_html_js: concat!(
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
    ),
    html_ready_event: "amazon:html_ready",
    progress_event: "amazon:fetch_progress",
    complete_event: "amazon:fetch_complete",
    target_url_condition: "(url LIKE 'https://www.amazon.co.jp/your-orders/order-details%' \
         OR url LIKE 'https://www.amazon.co.jp/gp/your-account/order-details%')",
    window_not_open_message: "Amazon ウィンドウが開いていません。先にログインしてください。",
    timeout_message: "注文詳細取得タイムアウト（30秒）",
};

// ─────────────────────────────────────────────────────────────────────────────
// Tauri コマンド
// ─────────────────────────────────────────────────────────────────────────────

/// Amazon ログインウィンドウを開く
///
/// 既存ウィンドウがある場合はフォーカスする。
/// ウィンドウ内で Amazon.co.jp にログイン後、`start_amazon_order_fetch` を呼ぶ。
#[tauri::command]
pub async fn open_amazon_login_window(app_handle: AppHandle) -> Result<(), String> {
    AMAZON_SESSION.open_window(&app_handle)
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
    AMAZON_SESSION.ensure_window_open(&app_handle)?;
    session_state.try_start()?;

    let state_for_cancel = session_state.inner().clone();
    let state_for_finish = session_state.inner().clone();
    AMAZON_SESSION.spawn_fetch(
        &app_handle,
        pool.inner().clone(),
        move || state_for_cancel.should_cancel(),
        move || state_for_finish.finish(),
        force_refetch.unwrap_or(false),
    );
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
