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

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::{
    plugins::surugaya_mp::html_parser,
    repository::SqliteOrderRepository,
};

// ─────────────────────────────────────────────────────────────────────────────
// 状態管理
// ─────────────────────────────────────────────────────────────────────────────

/// マイページ取得バッチの実行状態
#[derive(Clone, Default)]
pub struct SurugayaSessionState {
    is_running: Arc<Mutex<bool>>,
    should_cancel: Arc<AtomicBool>,
}

impl SurugayaSessionState {
    pub fn new() -> Self {
        Self::default()
    }

    fn try_start(&self) -> Result<(), String> {
        let mut running = self
            .is_running
            .lock()
            .map_err(|e| format!("Lock error: {e}"))?;
        if *running {
            return Err("マイページ取得は既に実行中です。".to_string());
        }
        *running = true;
        self.should_cancel.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn finish(&self) {
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }
        self.should_cancel.store(false, Ordering::SeqCst);
    }

    fn request_cancel(&self) {
        self.should_cancel.store(true, Ordering::SeqCst);
    }

    fn should_cancel(&self) -> bool {
        self.should_cancel.load(Ordering::SeqCst)
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
/// 取得対象は `htmls` テーブルで `analysis_status = 'pending'` かつ
/// 駿河屋マイページ URL のレコード。
#[tauri::command]
pub async fn start_surugaya_mypage_fetch(
    app_handle: AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    session_state: tauri::State<'_, SurugayaSessionState>,
) -> Result<(), String> {
    let win = app_handle
        .get_webview_window("surugaya-session")
        .ok_or("駿河屋ウィンドウが開いていません。先にログインしてください。")?;

    session_state.try_start()?;

    let pool_clone = pool.inner().clone();
    let app_clone = app_handle.clone();
    let state_clone = session_state.inner().clone();

    tokio::spawn(async move {
        let result = run_mypage_batch(&app_clone, &pool_clone, &win, &state_clone).await;
        state_clone.finish();
        // 完了イベント（エラーメッセージを Some に、成功時は None を送信）
        if let Err(ref e) = result {
            log::error!("[surugaya_session] Batch failed: {e}");
        }
        let error_msg: Option<String> = result.err();
        let _ = app_clone.emit("surugaya:fetch_complete", error_msg);
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
    let running = session_state
        .is_running
        .lock()
        .map_err(|e| format!("Lock error: {e}"))?;
    Ok(*running)
}

// ─────────────────────────────────────────────────────────────────────────────
// バッチ実行ロジック
// ─────────────────────────────────────────────────────────────────────────────

async fn run_mypage_batch(
    app: &AppHandle,
    pool: &SqlitePool,
    win: &tauri::WebviewWindow,
    state: &SurugayaSessionState,
) -> Result<(), String> {
    // pending な駿河屋マイページ URL を取得
    let pending: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, url FROM htmls \
         WHERE analysis_status = 'pending' \
         AND url LIKE 'https://www.suruga-ya.jp/pcmypage/%' \
         ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch pending htmls: {e}"))?;

    let total = pending.len();
    log::info!("[surugaya_session] {} pending mypage(s) to fetch", total);

    for (i, (html_id, url)) in pending.into_iter().enumerate() {
        if state.should_cancel() {
            log::info!("[surugaya_session] Cancelled at {}/{}", i, total);
            break;
        }

        // 進捗イベント
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

        // HTML をパース
        let mypage_info = match html_parser::parse_mypage_html(&html) {
            Ok(info) => info,
            Err(e) => {
                log::warn!("[surugaya_session] Failed to parse {}: {e}", url);
                continue;
            }
        };

        // トランザクションで DB 更新
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin tx: {e}"))?;

        // 既存注文に items / delivery_info を追加・更新
        if let Err(e) = SqliteOrderRepository::save_order_in_tx(
            &mut tx,
            &mypage_info.order_info,
            None,
            Some("suruga-ya.jp".to_string()),
            None,
        )
        .await
        {
            log::warn!("[surugaya_session] save_order failed for {}: {e}", url);
            // save_order_in_tx が失敗した場合は、このトランザクション全体をロールバックし、
            // analysis_status を completed に更新しないようにする
            if let Err(rollback_err) = tx.rollback().await {
                log::error!(
                    "[surugaya_session] Failed to rollback tx after save_order error for {}: {rollback_err}",
                    url
                );
            }
            // 現在処理中の HTML のみスキップし、次の HTML の処理を続行する
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
            log::warn!("[surugaya_session] Failed to update htmls {}: {e}", url);
        }

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit: {e}"))?;

        log::info!(
            "[surugaya_session] Processed {} ({}/{})",
            mypage_info.trade_code,
            i + 1,
            total
        );
    }

    Ok(())
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
    let parsed_url: tauri::Url = url
        .parse()
        .map_err(|e: url::ParseError| e.to_string())?;
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
