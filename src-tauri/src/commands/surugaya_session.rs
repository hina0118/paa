//! 駿河屋マーケットプレイス マイページ HTML 取得コマンド
//!
//! WebView ウィンドウを通じてログイン済みセッションを利用し、
//! マイページ HTML を取得・パースして注文データを補完する。
//! 取得処理の本体は [`super::webview_session`] で Amazon と共通化している。
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

use sqlx::SqlitePool;
use tauri::AppHandle;

use super::webview_session::WebviewSession;

// ─────────────────────────────────────────────────────────────────────────────
// 状態管理・セッション設定
// ─────────────────────────────────────────────────────────────────────────────

crate::batch_run_state::define_batch_state!(
    /// マイページ取得バッチの実行状態（`BatchRunState` の薄いラッパー）
    SurugayaSessionState,
    "マイページ取得は既に実行中です。"
);

static SURUGAYA_SESSION: WebviewSession = WebviewSession {
    log_tag: "surugaya_session",
    window_label: "surugaya-session",
    window_title: "駿河屋 マイページ",
    window_size: (1024.0, 768.0),
    start_url: "https://www.suruga-ya.jp/mypage/login",
    // マイページ詳細ページのロード完了時のみ HTML を送信
    is_target_page: |url| url.contains("pcmypage/action_sell_search/detail"),
    emit_html_js: concat!(
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
    ),
    html_ready_event: "surugaya:html_ready",
    progress_event: "surugaya:fetch_progress",
    complete_event: "surugaya:fetch_complete",
    target_url_condition: "url LIKE 'https://www.suruga-ya.jp/pcmypage/%'",
    window_not_open_message: "駿河屋ウィンドウが開いていません。先にログインしてください。",
    timeout_message: "マイページ取得タイムアウト（30秒）",
};

// ─────────────────────────────────────────────────────────────────────────────
// Tauri コマンド
// ─────────────────────────────────────────────────────────────────────────────

/// 駿河屋ログインウィンドウを開く
///
/// 既存ウィンドウがある場合はフォーカスする。
/// ウィンドウ内で suruga-ya.jp にログイン後、`start_surugaya_mypage_fetch` を呼ぶ。
#[tauri::command]
pub async fn open_surugaya_login_window(app_handle: AppHandle) -> Result<(), String> {
    SURUGAYA_SESSION.open_window(&app_handle)
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
    SURUGAYA_SESSION.ensure_window_open(&app_handle)?;
    session_state.try_start()?;

    let state_for_cancel = session_state.inner().clone();
    let state_for_finish = session_state.inner().clone();
    SURUGAYA_SESSION.spawn_fetch(
        &app_handle,
        pool.inner().clone(),
        move || state_for_cancel.should_cancel(),
        move || state_for_finish.finish(),
        force_refetch.unwrap_or(false),
    );
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

    #[test]
    fn test_is_target_page() {
        let is_target = SURUGAYA_SESSION.is_target_page;
        assert!(is_target(
            "https://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=1"
        ));
        assert!(!is_target("https://www.suruga-ya.jp/mypage/login"));
    }
}
