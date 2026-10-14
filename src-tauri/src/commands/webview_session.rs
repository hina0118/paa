//! WebView ログインセッション経由の HTML 取得（Amazon・駿河屋共通）
//!
//! ログイン済みの WebView ウィンドウで対象ページへ順にナビゲートし、
//! ページ読み込み完了時に eval() した JS から `window.__TAURI__.event.emit()` で
//! HTML を受け取り、`htmls.html_content` に保存する。パースは `run_batch_parse_task` で行う。
//!
//! 店舗ごとの差分（URL・ウィンドウ設定・イベント名など）は [`WebviewSession`] の定数で与える。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};

/// HTML 受信の待機時間
const HTML_READY_TIMEOUT: Duration = Duration::from_secs(30);

/// 店舗ごとの WebView セッション設定
pub(crate) struct WebviewSession {
    /// ログ出力用タグ（例: `amazon_session`）
    pub log_tag: &'static str,
    /// WebView ウィンドウのラベル
    pub window_label: &'static str,
    pub window_title: &'static str,
    pub window_size: (f64, f64),
    /// ウィンドウを開いたときに表示する URL
    pub start_url: &'static str,
    /// HTML を送信する対象ページかどうか
    pub is_target_page: fn(&str) -> bool,
    /// `on_page_load` で eval() する JS（`<prefix>:html_ready` イベントで HTML を emit する）
    pub emit_html_js: &'static str,
    /// HTML 受信イベント名（`emit_html_js` 内のイベント名と一致させる）
    pub html_ready_event: &'static str,
    pub progress_event: &'static str,
    pub complete_event: &'static str,
    /// 取得対象の `htmls` 行を絞り込む SQL 条件（固定文字列のみ）
    pub target_url_condition: &'static str,
    pub window_not_open_message: &'static str,
    pub timeout_message: &'static str,
}

impl WebviewSession {
    /// ログインウィンドウを開く（既存ウィンドウがあればフォーカスする）
    pub(crate) fn open_window(&'static self, app_handle: &AppHandle) -> Result<(), String> {
        if let Some(win) = app_handle.get_webview_window(self.window_label) {
            win.show().ok();
            win.set_focus().ok();
            return Ok(());
        }

        let url = WebviewUrl::External(
            self.start_url
                .parse()
                .map_err(|e: url::ParseError| e.to_string())?,
        );

        WebviewWindowBuilder::new(app_handle, self.window_label, url)
            .title(self.window_title)
            .inner_size(self.window_size.0, self.window_size.1)
            .on_page_load(move |window, payload| {
                use tauri::webview::PageLoadEvent;
                if !matches!(payload.event(), PageLoadEvent::Finished) {
                    return;
                }
                if !(self.is_target_page)(payload.url().as_str()) {
                    return;
                }
                // capabilities/<shop>-session.json の core:event:allow-emit により許可済み
                let _ = window.eval(self.emit_html_js);
            })
            .build()
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// 取得バッチをバックグラウンドで開始する
    ///
    /// ウィンドウが開いていない（未ログイン）場合はエラーを返す。
    /// 多重起動チェック（`try_start`）は呼び出し元で行い、完了時に `finish` が呼ばれる。
    pub(crate) fn spawn_fetch<C, F>(
        &'static self,
        app_handle: &AppHandle,
        pool: SqlitePool,
        should_cancel: C,
        finish: F,
        force_refetch: bool,
    ) where
        C: Fn() -> bool + Send + 'static,
        F: FnOnce() + Send + 'static,
    {
        let app = app_handle.clone();
        tokio::spawn(async move {
            let result = match app.get_webview_window(self.window_label) {
                Some(win) => {
                    self.run_batch(&app, &pool, &win, should_cancel, force_refetch)
                        .await
                }
                None => Err(self.window_not_open_message.to_string()),
            };
            finish();

            #[derive(serde::Serialize, Clone)]
            struct FetchCompletePayload {
                cancelled: bool,
                error: Option<String>,
            }

            let payload = match result {
                Ok(cancelled) => {
                    if cancelled {
                        log::info!("[{}] Batch cancelled by user", self.log_tag);
                    }
                    FetchCompletePayload {
                        cancelled,
                        error: None,
                    }
                }
                Err(e) => {
                    log::error!("[{}] Batch failed: {e}", self.log_tag);
                    FetchCompletePayload {
                        cancelled: false,
                        error: Some(e),
                    }
                }
            };
            let _ = app.emit(self.complete_event, payload);
        });
    }

    /// ウィンドウが開いているか（未ログインなら Err）
    pub(crate) fn ensure_window_open(&self, app_handle: &AppHandle) -> Result<(), String> {
        app_handle
            .get_webview_window(self.window_label)
            .map(|_| ())
            .ok_or_else(|| self.window_not_open_message.to_string())
    }

    /// 取得対象の `htmls` 行を選ぶ SQL
    ///
    /// - `force_refetch = true`: 取得済みを含む全件（HTML 更新時に使用）
    /// - `force_refetch = false`: `html_content IS NULL` のみ（差分取得・デフォルト）
    fn target_query(&self, force_refetch: bool) -> String {
        let only_missing = if force_refetch {
            ""
        } else {
            "html_content IS NULL AND "
        };
        format!(
            "SELECT id, url FROM htmls WHERE {only_missing}{} ORDER BY id",
            self.target_url_condition
        )
    }

    /// バッチ実行結果。`Ok(true)` = ユーザーによるキャンセル、`Ok(false)` = 正常完了。
    async fn run_batch(
        &self,
        app: &AppHandle,
        pool: &SqlitePool,
        win: &tauri::WebviewWindow,
        should_cancel: impl Fn() -> bool,
        force_refetch: bool,
    ) -> Result<bool, String> {
        let targets: Vec<(i64, String)> = sqlx::query_as(&self.target_query(force_refetch))
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Failed to fetch target htmls: {e}"))?;

        let total = targets.len();
        log::info!(
            "[{}] {} page(s) to fetch (force_refetch={})",
            self.log_tag,
            total,
            force_refetch
        );

        for (i, (html_id, url)) in targets.into_iter().enumerate() {
            if should_cancel() {
                log::info!("[{}] Cancelled at {}/{}", self.log_tag, i, total);
                return Ok(true);
            }

            let _ = app.emit(
                self.progress_event,
                serde_json::json!({ "current": i + 1, "total": total, "url": &url }),
            );

            let html = match self.fetch_one_html(app, win, &url).await {
                Ok(h) => h,
                Err(e) => {
                    log::warn!("[{}] Failed to fetch {}: {e}", self.log_tag, url);
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
                    "[{}] Failed to save html_content for {}: {e}",
                    self.log_tag,
                    url
                );
                continue;
            }

            log::info!("[{}] Fetched HTML ({}/{})", self.log_tag, i + 1, total);
        }

        Ok(false)
    }

    /// WebView を指定 URL にナビゲートし、`html_ready_event` で送られる HTML を受け取る
    async fn fetch_one_html(
        &self,
        app: &AppHandle,
        win: &tauri::WebviewWindow,
        url: &str,
    ) -> Result<String, String> {
        let (tx, rx) = tokio::sync::oneshot::channel::<String>();
        let tx = Arc::new(Mutex::new(Some(tx)));

        let event_id = app.once(self.html_ready_event, move |event| {
            // JS から送られる payload は JSON エンコードされた文字列
            let html = serde_json::from_str::<String>(event.payload())
                .unwrap_or_else(|_| event.payload().to_string());
            if let Some(sender) = tx.lock().ok().and_then(|mut guard| guard.take()) {
                let _ = sender.send(html);
            }
        });

        let parsed_url: tauri::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
        win.navigate(parsed_url).map_err(|e| e.to_string())?;

        match tokio::time::timeout(HTML_READY_TIMEOUT, rx).await {
            Ok(Ok(html)) => Ok(html),
            Ok(Err(_)) => {
                app.unlisten(event_id);
                Err("HTML 取得チャネルが閉じました".to_string())
            }
            Err(_) => {
                app.unlisten(event_id);
                Err(self.timeout_message.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_session() -> WebviewSession {
        WebviewSession {
            log_tag: "test",
            window_label: "test",
            window_title: "test",
            window_size: (800.0, 600.0),
            start_url: "https://example.com/",
            is_target_page: |_| true,
            emit_html_js: "",
            html_ready_event: "test:html_ready",
            progress_event: "test:fetch_progress",
            complete_event: "test:fetch_complete",
            target_url_condition: "url LIKE 'https://example.com/%'",
            window_not_open_message: "not open",
            timeout_message: "timeout",
        }
    }

    #[tokio::test]
    async fn target_query_selects_missing_or_all_rows() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE htmls (id INTEGER PRIMARY KEY, url TEXT NOT NULL, html_content TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO htmls (url, html_content) VALUES \
             ('https://example.com/a', NULL), \
             ('https://example.com/b', '<html></html>'), \
             ('https://other.com/c', NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let session = dummy_session();
        let missing: Vec<(i64, String)> = sqlx::query_as(&session.target_query(false))
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].1, "https://example.com/a");

        let all: Vec<(i64, String)> = sqlx::query_as(&session.target_query(true))
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(all.len(), 2);
    }
}
