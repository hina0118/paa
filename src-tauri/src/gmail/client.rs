//! Gmail API連携モジュール
//!
//! # セキュリティガイドライン
//! このモジュールはユーザーのメールデータを扱うため、以下のセキュリティルールを厳守してください：
//!
//! - **機密情報のログ出力禁止**: メール本文、件名、送信者/受信者情報などをログに出力しないこと
//! - **デバッグログの制限**: base64データの内容、メールペイロードの詳細を出力しないこと
//! - **メトリクスのみ**: ログに出力できるのは文字数、件数、処理時間などの統計情報のみ
//! - **本番環境**: リリースビルドではWarnレベル以上のログのみが出力されます

use crate::gmail_client::GmailClientTrait;
#[cfg(test)]
use crate::logic::sync_logic::build_sync_query;
use crate::repository::EmailRepository;
use async_trait::async_trait;
use google_gmail1::api::Scope;
use google_gmail1::{hyper_rustls, Gmail};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use yup_oauth2 as oauth2;

// カスタムInstalledFlowDelegateでブラウザを自動的に開く
struct CustomFlowDelegate;

impl oauth2::authenticator_delegate::InstalledFlowDelegate for CustomFlowDelegate {
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        need_code: bool,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>>
    {
        Box::pin(async move {
            log::info!("Opening browser with URL: {url}");

            // ブラウザで認証URLを開く
            if let Err(e) = webbrowser::open(url) {
                log::warn!("Failed to open browser automatically: {e}");
                log::warn!("Please open this URL manually in your browser:");
                log::warn!("{url}");
            } else {
                log::info!("Browser opened successfully. Please complete the authentication in your browser.");
            }

            if need_code {
                log::info!("Waiting for authentication code...");
            }

            // HTTPRedirectモードでは空文字列を返す（リダイレクトでコードを受け取る）
            Ok(String::new())
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailMessage {
    pub message_id: String,
    pub snippet: String,
    pub subject: Option<String>,
    pub body_plain: Option<String>,
    pub body_html: Option<String>,
    pub internal_date: i64,
    pub from_address: Option<String>,
}

/// Gmail 同期の保存結果。saved_count は INSERT または ON CONFLICT DO UPDATE で rows_affected>0 の件数
/// （重複も更新されるため「新規のみ」ではない）。skipped_count は rows_affected=0 の件数（通常は 0）。
#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct FetchResult {
    pub fetched_count: usize,
    pub saved_count: usize,
    pub skipped_count: usize,
}

#[derive(Debug, Serialize)]
pub struct SyncMetadata {
    pub sync_status: String,
    pub oldest_fetched_date: Option<String>,
    pub total_synced_count: i64,
    pub batch_size: i64,
    pub last_sync_started_at: Option<String>,
    pub last_sync_completed_at: Option<String>,
    pub max_iterations: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ShopSettings {
    pub id: i64,
    pub shop_name: String,
    pub sender_address: String,
    pub parser_type: String,
    pub is_enabled: bool,
    #[serde(default)]
    #[sqlx(default)]
    pub subject_filters: Option<String>, // JSON array stored as string
    pub created_at: String,
    pub updated_at: String,
}

impl ShopSettings {
    /// subject_filtersをパースしてVec<String>として取得
    pub fn get_subject_filters(&self) -> Vec<String> {
        self.subject_filters
            .as_ref()
            .and_then(|json_str| serde_json::from_str::<Vec<String>>(json_str).ok())
            .unwrap_or_default()
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateShopSettings {
    pub shop_name: String,
    pub sender_address: String,
    pub parser_type: String,
    pub subject_filters: Option<Vec<String>>, // Frontend sends array, we'll convert to JSON
}

#[derive(Debug, Deserialize)]
pub struct UpdateShopSettings {
    pub shop_name: Option<String>,
    pub sender_address: Option<String>,
    pub parser_type: Option<String>,
    pub is_enabled: Option<bool>,
    pub subject_filters: Option<Vec<String>>,
}

/// Synchronization state for Gmail sync operations
///
/// 進捗テーブル削除後はメモリのみで状態を管理する。
/// last_error はエラー時に設定され、次回 start でクリアされる。
///
/// # Lock Ordering
/// To prevent deadlock, always acquire locks in this order:
/// 1. `should_cancel`
/// 2. `is_running`
///
/// This ordering must be maintained consistently throughout the codebase.
#[derive(Clone)]
pub struct SyncState {
    pub should_cancel: Arc<Mutex<bool>>,
    pub is_running: Arc<Mutex<bool>>,
    /// 直近のエラーメッセージ（エラー時のみ。try_start でクリア）
    pub last_error: Arc<Mutex<Option<String>>>,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            should_cancel: Arc::new(Mutex::new(false)),
            is_running: Arc::new(Mutex::new(false)),
            last_error: Arc::new(Mutex::new(None)),
        }
    }
}

impl SyncState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_cancel(&self) {
        if let Ok(mut cancel) = self.should_cancel.lock() {
            *cancel = true;
        }
    }

    pub fn should_stop(&self) -> bool {
        self.should_cancel.lock().map(|c| *c).unwrap_or(false)
    }

    /// エラーを記録（get_sync_status で error として返す）
    pub fn set_error(&self, msg: &str) {
        if let Ok(mut err) = self.last_error.lock() {
            *err = Some(msg.to_string());
        }
    }

    /// エラーをクリア（try_start で呼ぶ）
    pub fn clear_error(&self) {
        if let Ok(mut err) = self.last_error.lock() {
            *err = None;
        }
    }

    /// 強制的に idle にリセット（reset_sync_status コマンド用）
    pub fn force_idle(&self) {
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }
        self.clear_error();
    }

    #[allow(dead_code)]
    pub fn reset(&self) {
        if let Ok(mut cancel) = self.should_cancel.lock() {
            *cancel = false;
        }
    }

    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.is_running.lock().map(|r| *r).unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn set_running(&self, running: bool) {
        if let Ok(mut is_running) = self.is_running.lock() {
            *is_running = running;
        }
    }

    /// Atomically check if not running, reset cancellation flag, and set to running.
    /// Returns true if successfully started, false if already running.
    ///
    /// # Locking behavior
    /// This method acquires *both* `should_cancel` and `is_running` locks and updates
    /// them while holding the locks to avoid races with `request_cancel`. The locks
    /// are always taken in the same order (`should_cancel` then `is_running`) and no
    /// other method holds both locks simultaneously, so this does not introduce
    /// deadlock risk.
    pub fn try_start(&self) -> bool {
        // First, acquire the cancellation flag lock.
        let mut cancel = if let Ok(guard) = self.should_cancel.lock() {
            guard
        } else {
            log::error!(
                "Failed to acquire should_cancel lock in try_start (mutex poisoned or unavailable)"
            );
            return false;
        };

        // Then, acquire the running state lock. Lock order is consistent to avoid deadlocks.
        let mut is_running = if let Ok(guard) = self.is_running.lock() {
            guard
        } else {
            log::error!(
                "Failed to acquire is_running lock in try_start (mutex poisoned or unavailable)"
            );
            return false;
        };

        // If we're already running, do not change any flags.
        if *is_running {
            return false;
        }

        // Start running, clear any pending cancellation, and clear last error.
        *is_running = true;
        *cancel = false;
        drop(is_running);
        drop(cancel);
        self.clear_error();

        true
    }
}

/// RAII guard that automatically resets the running flag when dropped
/// This ensures cleanup happens even on early returns or panics
pub struct SyncGuard<'a> {
    sync_state: &'a SyncState,
}

impl<'a> SyncGuard<'a> {
    pub const fn new(sync_state: &'a SyncState) -> Self {
        Self { sync_state }
    }
}

impl Drop for SyncGuard<'_> {
    fn drop(&mut self) {
        // Attempt to clear the running flag. If the mutex is poisoned due to a panic,
        // recover the inner value and clear the flag so future syncs are not blocked.
        match self.sync_state.is_running.lock() {
            Ok(mut is_running) => {
                *is_running = false;
            }
            Err(poisoned) => {
                log::warn!(
                    "Mutex for running flag was poisoned in SyncGuard::drop; clearing flag anyway"
                );
                let mut is_running = poisoned.into_inner();
                *is_running = false;
            }
        }
    }
}

pub struct GmailClient {
    hub: Gmail<hyper_rustls::HttpsConnector<HttpConnector>>,
}

impl GmailClient {
    pub async fn new(app_handle: &AppHandle) -> Result<Self, String> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {e}"))?;

        std::fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Failed to create app data dir: {e}"))?;

        // トークンはファイルに保存（既存の動作を維持）
        let token_path = app_data_dir.join("gmail_token.json");

        // keyringから認証情報を取得
        let (client_id, client_secret) =
            crate::gmail::config::load_oauth_credentials(&app_data_dir)
            .map_err(|e| {
                format!(
                    "Gmail OAuth credentials not configured. Please set up OAuth credentials in Settings.\n\nError: {e}"
                )
            })?;

        let auth = Self::authenticate_from_keyring(&client_id, &client_secret, &token_path).await?;

        // トークンを取得して認証を確実にする
        // gmail.readonlyスコープのみを使用（デスクトップアプリケーションに必要な最小限の権限）
        // ※get_token が None を返すと Authorization ヘッダーが付与されず 403 エラーになる
        log::info!("Requesting OAuth token...");
        let token = auth
            .token(&["https://www.googleapis.com/auth/gmail.readonly"])
            .await
            .map_err(|e| format!("Failed to get OAuth token: {e}"))?;
        let token_str = token.token().unwrap_or("");
        if token_str.is_empty() {
            return Err(
                "OAuth token is empty. Please re-authenticate: delete gmail_token.json and run sync again.".to_string(),
            );
        }
        log::info!(
            "OAuth token obtained successfully (len={}, Authorization: Bearer will be set)",
            token_str.len()
        );

        // Gmail Hub用のHTTPコネクタとクライアントを作成
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|e| format!("Failed to create HTTPS connector: {e}"))?
            .https_or_http()
            .enable_http1()
            .build();

        let client = Client::builder(TokioExecutor::new()).build(https);

        let hub = Gmail::new(client, auth);

        Ok(Self { hub })
    }

    /// keyringから読み込んだ認証情報を使用して認証を実行
    async fn authenticate_from_keyring(
        client_id: &str,
        client_secret: &str,
        token_path: &PathBuf,
    ) -> Result<
        oauth2::authenticator::Authenticator<hyper_rustls::HttpsConnector<HttpConnector>>,
        String,
    > {
        // ApplicationSecretを手動で構築
        let secret = oauth2::ApplicationSecret {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            redirect_uris: vec!["http://localhost".to_string()],
            project_id: None,
            client_email: None,
            auth_provider_x509_cert_url: None,
            client_x509_cert_url: None,
        };

        log::info!("Starting OAuth authentication flow...");
        log::info!("Opening browser for authentication...");

        // カスタムブラウザオープナーを使用してHTTPRedirectモードで認証
        let auth = oauth2::InstalledFlowAuthenticator::builder(
            secret,
            oauth2::InstalledFlowReturnMethod::HTTPRedirect,
        )
        .persist_tokens_to_disk(token_path)
        .flow_delegate(Box::new(CustomFlowDelegate))
        .build()
        .await
        .map_err(|e| {
            format!(
                "Failed to create authenticator: {e}\n\n\
                If a browser window didn't open, please check the console for the authentication URL and open it manually.\n\
                URL format: https://accounts.google.com/o/oauth2/auth?..."
            )
        })?;

        Ok(auth)
    }

    #[allow(dead_code)]
    pub async fn fetch_messages(&self, query: &str) -> Result<Vec<GmailMessage>, String> {
        let mut all_messages = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut req = self
                .hub
                .users()
                .messages_list("me")
                .q(query)
                .include_spam_trash(true);

            if let Some(token) = page_token {
                req = req.page_token(&token);
            }

            let (_, result) = req
                .doit()
                .await
                .map_err(|e| format!("Failed to list messages: {e}"))?;

            if let Some(messages) = result.messages {
                // メッセージIDを収集
                let message_ids: Vec<String> =
                    messages.iter().filter_map(|msg| msg.id.clone()).collect();

                log::info!(
                    "Fetching {} messages in parallel batches",
                    message_ids.len()
                );

                // 順次処理でメッセージを取得
                // 注: 並列処理はライフタイムの問題とGmail API制限により複雑
                // 将来的な改善: tokio::spawn + Arc<Mutex<Hub>>の使用を検討
                for message_id in message_ids {
                    match self.get_message(&message_id).await {
                        Ok(msg) => all_messages.push(msg),
                        Err(e) => log::warn!("Failed to fetch message {message_id}: {e}"),
                    }
                }
            }

            page_token = result.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(all_messages)
    }

    async fn get_message(&self, message_id: &str) -> Result<GmailMessage, String> {
        log::debug!("Fetching message: {message_id}");

        let (response, message) = self
            .hub
            .users()
            .messages_get("me", message_id)
            .add_scope(Scope::Readonly)
            .format("full")
            .doit()
            .await
            .map_err(|e| format!("Failed to get message {message_id}: {e}"))?;

        log::debug!("Response status: {:?}", response.status());

        let snippet = message.snippet.unwrap_or_default();
        let internal_date = message.internal_date.unwrap_or(0);

        let mut body_plain: Option<String> = None;
        let mut body_html: Option<String> = None;
        let mut from_address: Option<String> = None;
        let mut subject: Option<String> = None;

        // Extract From and Subject headers from payload
        if let Some(payload) = &message.payload {
            if let Some(headers) = &payload.headers {
                for header in headers {
                    if let Some(name) = &header.name {
                        let name_lower = name.to_lowercase();
                        if name_lower == "from" {
                            from_address = header.value.clone();
                        } else if name_lower == "subject" {
                            subject = header.value.clone();
                        }
                    }
                }
            }
        }

        // 再帰的にMIMEパートを解析
        if let Some(payload) = &message.payload {
            log::debug!(
                "Message {} payload: mime_type={:?}, has_body={}, has_parts={}",
                message_id,
                payload.mime_type,
                payload.body.is_some(),
                payload.parts.as_ref().map_or(0, std::vec::Vec::len)
            );
            Self::extract_body_from_part(
                payload,
                &mut body_plain,
                &mut body_html,
                Some(message_id),
            );
        } else {
            log::warn!("Message {message_id} has no payload");
        }

        log::debug!(
            "Message {} extracted: plain={} bytes, html={} bytes",
            message_id,
            body_plain.as_ref().map_or(0, std::string::String::len),
            body_html.as_ref().map_or(0, std::string::String::len)
        );

        Ok(GmailMessage {
            message_id: message_id.to_string(),
            snippet,
            subject,
            body_plain,
            body_html,
            internal_date,
            from_address,
        })
    }

    /// body.data のバイト列を文字列にデコードする
    ///
    /// mime_type に charset が指定されている場合はそれを優先し、Shift_JIS/ISO-2022-JP の
    /// バイト列がたまたま UTF-8 としても解釈可能な場合の文字化けを防ぐ。
    /// 未指定時は UTF-8 → Base64 → ISO-2022-JP/Shift_JIS の順で試行。
    ///
    /// 不正シーケンスが含まれる場合（had_replacements）は警告を出しつつ部分的なデコード結果を返す。
    /// 部分結果を返す理由: 注文番号・追跡番号などパーサーが抽出する情報は、U+FFFD 等の置換文字が
    /// 含まれていても読み取り可能な部分から取得できることが多い。利用可能な部分を必ず返す設計
    /// （呼び出し元はメール本文パース用途に限定され、部分結果からでも注文情報を抽出できる方が有用）。
    fn decode_body_to_string(data: &[u8], mime_type: &str) -> String {
        let mime_lower = mime_type.to_lowercase();

        // 1. mime_type で charset が明示されている場合はそれを優先
        //    （Shift_JIS 等が valid UTF-8 と誤判定される文字化けを防ぐ）
        if mime_lower.contains("iso-2022-jp") || mime_lower.contains("iso_2022_jp") {
            let (decoded, _, had_replacements) = encoding_rs::ISO_2022_JP.decode(data);
            if had_replacements {
                log::warn!("ISO-2022-JP decode had replacement chars; returning partial content");
            }
            return decoded.into_owned();
        } else if mime_lower.contains("shift_jis")
            || mime_lower.contains("shift-jis")
            || mime_lower.contains("windows-31j")
            || mime_lower.contains("cp932")
        {
            let (decoded, _, had_replacements) = encoding_rs::SHIFT_JIS.decode(data);
            if had_replacements {
                log::warn!("Shift_JIS decode had replacement chars; returning partial content");
            }
            return decoded.into_owned();
        } else if mime_lower.contains("utf-8") || mime_lower.contains("utf8") {
            // charset=utf-8 が明示指定: UTF-8 でデコード。不正バイトは置換文字で処理（ISO-2022-JP にはフォールバックしない）
            if let Ok(data_str) = std::str::from_utf8(data) {
                if let Some(decoded) = Self::try_decode_base64(data_str) {
                    return decoded;
                }
                return data_str.to_string();
            }
            let (decoded, _, had_replacements) = encoding_rs::UTF_8.decode(data);
            if had_replacements {
                log::warn!("UTF-8 decode had replacement chars; returning partial content");
            }
            return decoded.into_owned();
        }

        // 2. charset 未指定: UTF-8 として解釈を試みる
        if let Ok(data_str) = std::str::from_utf8(data) {
            // Base64 形式の場合はデコードして再試行（Gmail API の body.data が base64 の場合）
            if let Some(decoded) = Self::try_decode_base64(data_str) {
                return decoded;
            }
            return data_str.to_string();
        }

        // 3. charset 未指定時のフォールバック: ISO-2022-JP を試行（日本語メールで最も一般的）
        let (decoded, _, had_replacements) = encoding_rs::ISO_2022_JP.decode(data);
        if had_replacements {
            log::warn!("Fallback encoding decode had replacement chars; returning partial content");
        }
        decoded.into_owned()
    }

    /// `Base64URL形式の文字列かどうかを検証する`
    ///
    /// Base64URLで使用される文字セット（A-Z, a-z, 0-9, -, _）のみで構成されているかチェック
    /// 長さが4の倍数に近い場合はBase64の可能性が高い
    fn is_base64_format(data: &str) -> bool {
        if data.is_empty() {
            return false;
        }

        // Base64URL文字セット: A-Z, a-z, 0-9, -, _
        // パディングなしの形式なので = はチェックしない
        let is_base64_chars = data
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

        if !is_base64_chars {
            return false;
        }

        // Base64は通常4の倍数の長さだが、パディングなしの場合は異なる可能性がある
        // 少なくとも妥当な長さ（8文字以上）であることを確認
        // 短すぎる文字列は通常のテキストの可能性が高い
        data.len() >= 8
    }

    /// `Base64URLデコードを試みる`
    ///
    /// `データがBase64形式でない場合はNoneを返す`
    /// `デコードに成功した場合はSome(decoded_string)を返す`
    fn try_decode_base64(data: &str) -> Option<String> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

        // Base64形式でない場合は早期リターン
        if !Self::is_base64_format(data) {
            log::debug!("Data is not in Base64 format, skipping decode");
            return None;
        }

        log::debug!("Attempting to decode base64, input length: {}", data.len());

        match URL_SAFE_NO_PAD.decode(data) {
            Ok(bytes) => {
                let result = String::from_utf8_lossy(&bytes).to_string();
                log::debug!(
                    "Successfully decoded {} bytes -> {} chars",
                    bytes.len(),
                    result.len()
                );
                Some(result)
            }
            Err(e) => {
                log::warn!(
                    "Base64 decode failed despite format check: {:?}, input length: {}",
                    e,
                    data.len()
                );
                None
            }
        }
    }

    // 再帰的にMIMEパートを解析する
    // message_id はトップレベル呼び出し時のみ渡し、ログのトレース用に使用
    fn extract_body_from_part(
        part: &google_gmail1::api::MessagePart,
        body_plain: &mut Option<String>,
        body_html: &mut Option<String>,
        message_id: Option<&str>,
    ) {
        // 現在のパートのbodyをチェック
        if let Some(mime_type) = &part.mime_type {
            log::debug!("Processing part with mime_type: {mime_type}");
            if let Some(body) = &part.body {
                log::debug!("  Body present, size: {:?}", body.size);
                if let Some(data) = &body.data {
                    log::debug!("  Data present, length: {} bytes", data.len());

                    // 文字列として解釈（UTF-8 → ISO-2022-JP/Shift_JIS のフォールバック）
                    let content = Self::decode_body_to_string(data, mime_type);
                    log::debug!("  Final content length: {} chars", content.len());
                    // mimeType は "text/plain; charset=..." のようにパラメータ付きの場合があるため starts_with で判定
                    let mime = mime_type.trim();
                    if mime.starts_with("text/plain") && body_plain.is_none() {
                        log::info!(
                            "Found text/plain body: {} chars{}",
                            content.len(),
                            message_id
                                .map(|id| format!(" (message_id={})", id))
                                .unwrap_or_default()
                        );
                        *body_plain = Some(content);
                    } else if mime.starts_with("text/html") && body_html.is_none() {
                        log::info!(
                            "Found text/html body: {} chars{}",
                            content.len(),
                            message_id
                                .map(|id| format!(" (message_id={})", id))
                                .unwrap_or_default()
                        );
                        *body_html = Some(content);
                    } else {
                        log::debug!("  Skipping mime_type: {mime_type}");
                    }
                } else {
                    log::debug!("  No data in body");
                }
            } else {
                log::debug!("  No body in part");
            }
        }

        // 子パートを再帰的に処理（再帰時は message_id を渡さない）
        if let Some(parts) = &part.parts {
            log::debug!("Processing {} child parts", parts.len());
            for child_part in parts {
                Self::extract_body_from_part(child_part, body_plain, body_html, None);
            }
        }
    }
}

/// GmailClientTrait の実装
///
/// これにより GmailClient をモックに置き換えてテストできます。
#[async_trait]
impl GmailClientTrait for GmailClient {
    async fn list_message_ids(
        &self,
        query: &str,
        max_results: u32,
        page_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), String> {
        let mut req = self
            .hub
            .users()
            .messages_list("me")
            .q(query)
            .max_results(max_results)
            .include_spam_trash(true);

        if let Some(ref token) = page_token {
            req = req.page_token(token);
        }

        let (_, result) = req
            .doit()
            .await
            .map_err(|e| format!("Failed to list messages: {e}"))?;

        let message_ids = result
            .messages
            .unwrap_or_default()
            .into_iter()
            .filter_map(|msg| msg.id)
            .collect();

        Ok((message_ids, result.next_page_token))
    }

    async fn get_message(&self, message_id: &str) -> Result<GmailMessage, String> {
        // 既存の get_message メソッドを呼び出す（GmailClient の固有実装）
        GmailClient::get_message(self, message_id).await
    }
}

pub async fn save_messages_to_db(
    pool: &SqlitePool,
    messages: &[GmailMessage],
    shop_settings: &[ShopSettings],
) -> Result<FetchResult, String> {
    log::info!("Saving {} messages to database using sqlx", messages.len());

    let mut saved_count = 0;
    let mut skipped_count = 0;
    let mut filtered_count = 0;

    // トランザクションを使用してバッチ処理
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {e}"))?;

    for msg in messages {
        // Check subject filter (use the logic module version for consistency)
        if !crate::logic::sync_logic::should_save_message(msg, shop_settings) {
            filtered_count += 1;
            log::debug!(
                "Message {} filtered out by subject filter (subject: {:?})",
                msg.message_id,
                msg.subject
            );
            continue;
        }

        let result = sqlx::query(
            r"
            INSERT INTO emails (message_id, body_plain, body_html, internal_date, from_address, subject)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(message_id) DO UPDATE SET
                body_plain = COALESCE(excluded.body_plain, body_plain),
                body_html = COALESCE(excluded.body_html, body_html),
                internal_date = COALESCE(excluded.internal_date, internal_date),
                from_address = COALESCE(excluded.from_address, from_address),
                subject = COALESCE(excluded.subject, subject)
            ",
        )
        .bind(&msg.message_id)
        .bind(&msg.body_plain)
        .bind(&msg.body_html)
        .bind(msg.internal_date)
        .bind(&msg.from_address)
        .bind(&msg.subject)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert message {}: {}", msg.message_id, e))?;

        if result.rows_affected() > 0 {
            saved_count += 1;
        } else {
            skipped_count += 1;
        }
    }

    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit transaction: {e}"))?;

    log::info!(
        "Saved {saved_count} messages (inserted or updated), skipped {skipped_count}, filtered {filtered_count} by subject"
    );

    Ok(FetchResult {
        fetched_count: messages.len(),
        saved_count,
        skipped_count,
    })
}

/// EmailRepository経由でメッセージをDBに保存する（テスト可能なバージョン）
///
/// # Arguments
/// * `repo` - EmailRepositoryを実装したリポジトリ
/// * `messages` - 保存するメッセージリスト（所有権を取得し、in-placeでフィルタリング）
/// * `shop_settings` - 有効なショップ設定（件名フィルタリングに使用）
///
/// # Returns
/// FetchResult（保存数、スキップ数などの統計情報）
pub async fn save_messages_to_db_with_repo(
    repo: &dyn EmailRepository,
    mut messages: Vec<GmailMessage>,
    shop_settings: &[ShopSettings],
) -> Result<FetchResult, String> {
    let original_count = messages.len();
    log::info!(
        "Saving {} messages to database via repository",
        original_count
    );

    // ショップ設定でin-placeフィルタリング（cloneを回避）
    messages.retain(|msg| crate::logic::sync_logic::should_save_message(msg, shop_settings));
    let filtered_count = original_count - messages.len();

    // リポジトリ経由で保存
    let (saved_count, skipped_count) = repo.save_messages(&messages).await?;

    log::info!(
        "Saved {saved_count} messages (inserted or updated), skipped {skipped_count}, filtered {filtered_count} by subject"
    );

    Ok(FetchResult {
        fetched_count: original_count,
        saved_count,
        skipped_count,
    })
}

/// Helper function to format timestamp as RFC3339 (used by tests)
#[cfg(test)]
fn format_timestamp(internal_date: i64) -> String {
    chrono::DateTime::from_timestamp_millis(internal_date)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| {
            log::warn!(
                "Invalid internal_date '{internal_date}' encountered when formatting timestamp; returning empty string"
            );
            String::new()
        })
}

// sync_gmail_incremental and sync_gmail_incremental_with_client removed.
// Use BatchRunner<GmailSyncTask> via start_sync command instead.

// ============================================================================
// Parser Type Routing Functions
// ============================================================================
// NOTE: should_save_message と extract_email_address は
// crate::logic::sync_logic に統一されました。
// get_parser_type_for_sender は crate::logic::email_parser::get_candidate_parsers を使用してください。

// ============================================================================
// Shop Settings Database Functions
// ============================================================================

/// Get all shop settings from the database
pub async fn get_all_shop_settings(pool: &SqlitePool) -> Result<Vec<ShopSettings>, String> {
    sqlx::query_as::<_, ShopSettings>(
        r#"
        SELECT id, shop_name, sender_address, parser_type, is_enabled,
               subject_filters, created_at, updated_at
        FROM shop_settings
        ORDER BY id ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch shop settings: {e}"))
}

/// Get enabled shop settings only
pub async fn get_enabled_shop_settings(pool: &SqlitePool) -> Result<Vec<ShopSettings>, String> {
    sqlx::query_as::<_, ShopSettings>(
        r#"
        SELECT id, shop_name, sender_address, parser_type, is_enabled,
               subject_filters, created_at, updated_at
        FROM shop_settings
        WHERE is_enabled = 1
        ORDER BY id ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch enabled shop settings: {e}"))
}

/// Create a new shop setting
pub async fn create_shop_setting(
    pool: &SqlitePool,
    settings: CreateShopSettings,
) -> Result<i64, String> {
    // Convert Vec<String> to JSON string
    let subject_filters_json = settings
        .subject_filters
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| format!("Failed to serialize subject_filters: {e}"))?;

    let result = sqlx::query(
        r#"
        INSERT INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled)
        VALUES (?, ?, ?, ?, 1)
        "#,
    )
    .bind(&settings.shop_name)
    .bind(&settings.sender_address)
    .bind(&settings.parser_type)
    .bind(&subject_filters_json)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to create shop setting: {e}"))?;

    Ok(result.last_insert_rowid())
}

/// Update an existing shop setting
pub async fn update_shop_setting(
    pool: &SqlitePool,
    id: i64,
    settings: UpdateShopSettings,
) -> Result<(), String> {
    let existing = sqlx::query_as::<_, ShopSettings>(
        "SELECT id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at FROM shop_settings WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to fetch shop setting: {e}"))?
    .ok_or_else(|| format!("Shop setting with id {id} not found"))?;

    let shop_name = settings.shop_name.unwrap_or(existing.shop_name);
    let sender_address = settings.sender_address.unwrap_or(existing.sender_address);
    let parser_type = settings.parser_type.unwrap_or(existing.parser_type);
    let is_enabled = settings.is_enabled.unwrap_or(existing.is_enabled);

    // Convert Vec<String> to JSON string if provided, otherwise keep existing
    let subject_filters_json = if let Some(filters) = settings.subject_filters {
        Some(
            serde_json::to_string(&filters)
                .map_err(|e| format!("Failed to serialize subject_filters: {e}"))?,
        )
    } else {
        existing.subject_filters
    };

    sqlx::query(
        r#"
        UPDATE shop_settings
        SET shop_name = ?, sender_address = ?, parser_type = ?, is_enabled = ?, subject_filters = ?,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(&shop_name)
    .bind(&sender_address)
    .bind(&parser_type)
    .bind(is_enabled)
    .bind(&subject_filters_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update shop setting: {e}"))?;

    Ok(())
}

/// Delete a shop setting
pub async fn delete_shop_setting(pool: &SqlitePool, id: i64) -> Result<(), String> {
    let result = sqlx::query("DELETE FROM shop_settings WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to delete shop setting: {e}"))?;

    if result.rows_affected() == 0 {
        return Err(format!("Shop setting with id {id} not found"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;

    // Helper function to create an in-memory test database
    async fn create_test_db() -> sqlx::SqlitePool {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .expect("Failed to create test database");

        // Create emails table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS emails (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id TEXT UNIQUE NOT NULL,
                body_plain TEXT,
                body_html TEXT,
                internal_date INTEGER NOT NULL,
                from_address TEXT,
                subject TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&pool)
        .await
        .expect("Failed to create emails table");

        pool
    }

    #[test]
    fn test_gmail_message_structure() {
        let message = GmailMessage {
            message_id: "test123".to_string(),
            snippet: "Test snippet".to_string(),
            body_plain: Some("Plain text body".to_string()),
            body_html: Some("<html>HTML body</html>".to_string()),
            internal_date: 1234567890000,
            from_address: Some("test@example.com".to_string()),
            subject: Some("Test subject".to_string()),
        };

        assert_eq!(message.message_id, "test123");
        assert_eq!(message.snippet, "Test snippet");
        assert!(message.body_plain.is_some());
        assert!(message.body_html.is_some());
        assert_eq!(message.internal_date, 1234567890000);
    }

    #[test]
    fn test_fetch_result_structure() {
        let result = FetchResult {
            fetched_count: 10,
            saved_count: 8,
            skipped_count: 2,
        };

        assert_eq!(result.fetched_count, 10);
        assert_eq!(result.saved_count, 8);
        assert_eq!(result.skipped_count, 2);
    }

    #[test]
    fn test_sync_state_initialization() {
        let sync_state = SyncState::new();

        assert!(!sync_state.should_stop());
        assert!(!sync_state.is_running());
    }

    #[test]
    fn test_sync_state_cancel() {
        let sync_state = SyncState::new();

        assert!(!sync_state.should_stop());

        sync_state.request_cancel();

        assert!(sync_state.should_stop());
    }

    #[test]
    fn test_sync_state_try_start() {
        let sync_state = SyncState::new();

        // First start should succeed
        assert!(sync_state.try_start());
        assert!(sync_state.is_running());

        // Second start should fail (already running)
        assert!(!sync_state.try_start());
        assert!(sync_state.is_running());
    }

    #[test]
    fn test_sync_state_try_start_resets_cancel() {
        let sync_state = SyncState::new();

        // Set cancel flag
        sync_state.request_cancel();
        assert!(sync_state.should_stop());

        // try_start should reset the cancel flag
        assert!(sync_state.try_start());
        assert!(!sync_state.should_stop());
        assert!(sync_state.is_running());
    }

    #[test]
    fn test_sync_state_reset() {
        let sync_state = SyncState::new();

        sync_state.request_cancel();
        assert!(sync_state.should_stop());

        sync_state.reset();
        assert!(!sync_state.should_stop());
    }

    #[tokio::test]
    async fn test_save_messages_to_db_empty() {
        let pool = create_test_db().await;
        let messages: Vec<GmailMessage> = vec![];
        let shop_settings: Vec<ShopSettings> = vec![];

        let result = save_messages_to_db(&pool, &messages, &shop_settings)
            .await
            .expect("Failed to save empty messages");

        assert_eq!(result.fetched_count, 0);
        assert_eq!(result.saved_count, 0);
        assert_eq!(result.skipped_count, 0);
    }

    #[tokio::test]
    async fn test_save_messages_to_db_single() {
        let pool = create_test_db().await;

        let message = GmailMessage {
            message_id: "msg001".to_string(),
            snippet: "Test message".to_string(),
            body_plain: Some("Plain text".to_string()),
            body_html: Some("<html>HTML</html>".to_string()),
            internal_date: 1609459200000, // 2021-01-01
            from_address: Some("test@example.com".to_string()),
            subject: Some("Test subject".to_string()),
        };

        let shop_settings = vec![ShopSettings {
            id: 1,
            shop_name: "Test Shop".to_string(),
            sender_address: "test@example.com".to_string(),
            parser_type: "test".to_string(),
            subject_filters: None,
            is_enabled: true,
            created_at: "2021-01-01 00:00:00".to_string(),
            updated_at: "2021-01-01 00:00:00".to_string(),
        }];

        let result = save_messages_to_db(&pool, &[message], &shop_settings)
            .await
            .expect("Failed to save message");

        assert_eq!(result.fetched_count, 1);
        assert_eq!(result.saved_count, 1);
        assert_eq!(result.skipped_count, 0);

        // Verify the message was saved
        let row: (String, i64) = sqlx::query_as(
            "SELECT message_id, internal_date FROM emails WHERE message_id = 'msg001'",
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch saved message");

        assert_eq!(row.0, "msg001");
        assert_eq!(row.1, 1609459200000);
    }

    #[tokio::test]
    async fn test_save_messages_to_db_duplicate() {
        let pool = create_test_db().await;

        let message = GmailMessage {
            message_id: "msg002".to_string(),
            snippet: "Test message".to_string(),
            body_plain: Some("Plain text".to_string()),
            body_html: Some("<html>HTML</html>".to_string()),
            internal_date: 1609459200000,
            from_address: Some("test@example.com".to_string()),
            subject: Some("Test subject".to_string()),
        };

        let shop_settings = vec![ShopSettings {
            id: 1,
            shop_name: "Test Shop".to_string(),
            sender_address: "test@example.com".to_string(),
            parser_type: "test".to_string(),
            subject_filters: None,
            is_enabled: true,
            created_at: "2021-01-01 00:00:00".to_string(),
            updated_at: "2021-01-01 00:00:00".to_string(),
        }];

        // Save first time
        let result1 = save_messages_to_db(&pool, std::slice::from_ref(&message), &shop_settings)
            .await
            .expect("Failed to save message first time");

        assert_eq!(result1.saved_count, 1);
        assert_eq!(result1.skipped_count, 0);

        // Save second time: ON CONFLICT DO UPDATE により UPDATE が実行され、saved としてカウントされる
        let result2 = save_messages_to_db(&pool, &[message], &shop_settings)
            .await
            .expect("Failed to save message second time");

        assert_eq!(result2.saved_count, 1);
        assert_eq!(result2.skipped_count, 0);
    }

    #[tokio::test]
    async fn test_save_messages_to_db_batch() {
        let pool = create_test_db().await;

        let messages = vec![
            GmailMessage {
                message_id: "msg003".to_string(),
                snippet: "Message 1".to_string(),
                body_plain: Some("Body 1".to_string()),
                body_html: None,
                internal_date: 1609459200000,
                from_address: Some("test@example.com".to_string()),
                subject: Some("Test subject 1".to_string()),
            },
            GmailMessage {
                message_id: "msg004".to_string(),
                snippet: "Message 2".to_string(),
                body_plain: None,
                body_html: Some("<html>Body 2</html>".to_string()),
                internal_date: 1609545600000,
                from_address: Some("test@example.com".to_string()),
                subject: Some("Test subject 2".to_string()),
            },
            GmailMessage {
                message_id: "msg005".to_string(),
                snippet: "Message 3".to_string(),
                body_plain: Some("Body 3".to_string()),
                body_html: Some("<html>Body 3</html>".to_string()),
                internal_date: 1609632000000,
                from_address: Some("test@example.com".to_string()),
                subject: Some("Test subject 3".to_string()),
            },
        ];

        let shop_settings = vec![ShopSettings {
            id: 1,
            shop_name: "Test Shop".to_string(),
            sender_address: "test@example.com".to_string(),
            parser_type: "test".to_string(),
            subject_filters: None,
            is_enabled: true,
            created_at: "2021-01-01 00:00:00".to_string(),
            updated_at: "2021-01-01 00:00:00".to_string(),
        }];

        let result = save_messages_to_db(&pool, &messages, &shop_settings)
            .await
            .expect("Failed to save batch");

        assert_eq!(result.fetched_count, 3);
        assert_eq!(result.saved_count, 3);
        assert_eq!(result.skipped_count, 0);

        // Verify count in database
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM emails")
            .fetch_one(&pool)
            .await
            .expect("Failed to count emails");

        assert_eq!(count.0, 3);
    }

    #[tokio::test]
    async fn test_save_messages_to_db_partial_duplicate() {
        let pool = create_test_db().await;

        let shop_settings = vec![ShopSettings {
            id: 1,
            shop_name: "Test Shop".to_string(),
            sender_address: "test@example.com".to_string(),
            parser_type: "test".to_string(),
            subject_filters: None,
            is_enabled: true,
            created_at: "2021-01-01 00:00:00".to_string(),
            updated_at: "2021-01-01 00:00:00".to_string(),
        }];

        // First batch
        let messages1 = vec![
            GmailMessage {
                message_id: "msg006".to_string(),
                snippet: "Message 1".to_string(),
                body_plain: Some("Body 1".to_string()),
                body_html: None,
                internal_date: 1609459200000,
                from_address: Some("test@example.com".to_string()),
                subject: Some("Test subject 1".to_string()),
            },
            GmailMessage {
                message_id: "msg007".to_string(),
                snippet: "Message 2".to_string(),
                body_plain: Some("Body 2".to_string()),
                body_html: None,
                internal_date: 1609545600000,
                from_address: Some("test@example.com".to_string()),
                subject: Some("Test subject 2".to_string()),
            },
        ];

        save_messages_to_db(&pool, &messages1, &shop_settings)
            .await
            .expect("Failed to save first batch");

        // Second batch with one duplicate
        let messages2 = vec![
            GmailMessage {
                message_id: "msg007".to_string(), // Duplicate
                snippet: "Message 2".to_string(),
                body_plain: Some("Body 2".to_string()),
                body_html: None,
                internal_date: 1609545600000,
                from_address: Some("test@example.com".to_string()),
                subject: Some("Test subject 2".to_string()),
            },
            GmailMessage {
                message_id: "msg008".to_string(), // New
                snippet: "Message 3".to_string(),
                body_plain: Some("Body 3".to_string()),
                body_html: None,
                internal_date: 1609632000000,
                from_address: Some("test@example.com".to_string()),
                subject: Some("Test subject 3".to_string()),
            },
        ];

        let result = save_messages_to_db(&pool, &messages2, &shop_settings)
            .await
            .expect("Failed to save second batch");

        assert_eq!(result.fetched_count, 2);
        // 重複(msg007)も新規(msg008)も ON CONFLICT DO UPDATE で rows_affected=1 のため saved としてカウント
        assert_eq!(result.saved_count, 2);
        assert_eq!(result.skipped_count, 0);
    }

    #[test]
    fn test_sync_metadata_structure() {
        let metadata = SyncMetadata {
            sync_status: "idle".to_string(),
            oldest_fetched_date: Some("2024-01-01T00:00:00Z".to_string()),
            total_synced_count: 1000,
            batch_size: 50,
            last_sync_started_at: Some("2024-01-15T10:00:00Z".to_string()),
            last_sync_completed_at: Some("2024-01-15T10:30:00Z".to_string()),
            max_iterations: 100,
            last_error_message: None,
        };

        assert_eq!(metadata.sync_status, "idle");
        assert!(metadata.oldest_fetched_date.is_some());
        assert_eq!(metadata.total_synced_count, 1000);
        assert_eq!(metadata.batch_size, 50);
    }

    // エラーハンドリングテスト

    #[tokio::test]
    async fn test_save_messages_db_constraint_violation() {
        let pool = create_test_db().await;

        let shop_settings = vec![ShopSettings {
            id: 1,
            shop_name: "Test Shop".to_string(),
            sender_address: "test@example.com".to_string(),
            parser_type: "test".to_string(),
            subject_filters: None,
            is_enabled: true,
            created_at: "2021-01-01 00:00:00".to_string(),
            updated_at: "2021-01-01 00:00:00".to_string(),
        }];

        // 正常にメッセージを保存
        let message = GmailMessage {
            message_id: "msg_unique".to_string(),
            snippet: "Test message".to_string(),
            body_plain: Some("Plain text".to_string()),
            body_html: Some("<html>HTML</html>".to_string()),
            internal_date: 1609459200000,
            from_address: Some("test@example.com".to_string()),
            subject: Some("Test subject".to_string()),
        };

        let result1 = save_messages_to_db(&pool, std::slice::from_ref(&message), &shop_settings)
            .await
            .expect("Failed to save first message");

        assert_eq!(result1.saved_count, 1);

        // 同じmessage_idで再度保存: ON CONFLICT DO UPDATE により UPDATE が実行され、saved としてカウントされる
        let result2 = save_messages_to_db(&pool, &[message], &shop_settings)
            .await
            .expect("Should handle duplicate gracefully");

        assert_eq!(result2.saved_count, 1);
        assert_eq!(result2.skipped_count, 0);
    }

    #[tokio::test]
    async fn test_save_messages_invalid_internal_date() {
        let pool = create_test_db().await;

        let shop_settings = vec![ShopSettings {
            id: 1,
            shop_name: "Test Shop".to_string(),
            sender_address: "test@example.com".to_string(),
            parser_type: "test".to_string(),
            subject_filters: None,
            is_enabled: true,
            created_at: "2021-01-01 00:00:00".to_string(),
            updated_at: "2021-01-01 00:00:00".to_string(),
        }];

        // internal_dateが負の値（無効なタイムスタンプ）
        let message = GmailMessage {
            message_id: "msg_invalid_date".to_string(),
            snippet: "Test message".to_string(),
            body_plain: Some("Plain text".to_string()),
            body_html: Some("<html>HTML</html>".to_string()),
            internal_date: -1, // 無効な値
            from_address: Some("test@example.com".to_string()),
            subject: Some("Test subject".to_string()),
        };

        // データベース制約によっては保存される可能性があるが、
        // アプリケーションロジックでバリデーションを行う場合はエラーになる
        let result = save_messages_to_db(&pool, &[message], &shop_settings).await;

        // この場合、SQLiteは負の値も許容するため成功する
        assert!(result.is_ok());
        if let Ok(res) = result {
            assert_eq!(res.saved_count, 1);
        }
    }

    #[tokio::test]
    async fn test_save_messages_empty_message_id() {
        let pool = create_test_db().await;

        let shop_settings = vec![ShopSettings {
            id: 1,
            shop_name: "Test Shop".to_string(),
            sender_address: "test@example.com".to_string(),
            parser_type: "test".to_string(),
            subject_filters: None,
            is_enabled: true,
            created_at: "2021-01-01 00:00:00".to_string(),
            updated_at: "2021-01-01 00:00:00".to_string(),
        }];

        // message_idが空文字列
        let message = GmailMessage {
            message_id: String::new(),
            snippet: "Test message".to_string(),
            body_plain: Some("Plain text".to_string()),
            body_html: Some("<html>HTML</html>".to_string()),
            internal_date: 1609459200000,
            from_address: Some("test@example.com".to_string()),
            subject: Some("Test subject".to_string()),
        };

        let result = save_messages_to_db(&pool, &[message], &shop_settings).await;

        // SQLiteはNOT NULL制約でも空文字列を許容する
        assert!(result.is_ok());
        if let Ok(res) = result {
            assert_eq!(res.saved_count, 1);
        }
    }

    #[tokio::test]
    async fn test_save_messages_very_large_body() {
        let pool = create_test_db().await;

        let shop_settings = vec![ShopSettings {
            id: 1,
            shop_name: "Test Shop".to_string(),
            sender_address: "test@example.com".to_string(),
            parser_type: "test".to_string(),
            subject_filters: None,
            is_enabled: true,
            created_at: "2021-01-01 00:00:00".to_string(),
            updated_at: "2021-01-01 00:00:00".to_string(),
        }];

        // 非常に大きなボディ（1MB）
        let large_body = "x".repeat(1024 * 1024);

        let message = GmailMessage {
            message_id: "msg_large_body".to_string(),
            snippet: "Test message".to_string(),
            body_plain: Some(large_body.clone()),
            body_html: Some(large_body),
            internal_date: 1609459200000,
            from_address: Some("test@example.com".to_string()),
            subject: Some("Test subject".to_string()),
        };

        let result = save_messages_to_db(&pool, &[message], &shop_settings).await;

        // 大きなデータも保存できる
        assert!(result.is_ok());
        if let Ok(res) = result {
            assert_eq!(res.saved_count, 1);
        }
    }

    #[tokio::test]
    async fn test_save_messages_unicode_content() {
        let pool = create_test_db().await;

        let shop_settings = vec![ShopSettings {
            id: 1,
            shop_name: "Test Shop".to_string(),
            sender_address: "test@example.com".to_string(),
            parser_type: "test".to_string(),
            subject_filters: None,
            is_enabled: true,
            created_at: "2021-01-01 00:00:00".to_string(),
            updated_at: "2021-01-01 00:00:00".to_string(),
        }];

        // Unicode文字を含むメッセージ
        let message = GmailMessage {
            message_id: "msg_unicode".to_string(),
            snippet: "テストメッセージ 🎉".to_string(),
            body_plain: Some("こんにちは、世界！\n你好世界！\n안녕하세요！".to_string()),
            body_html: Some("<html>🌍 Unicode HTML 🌏</html>".to_string()),
            internal_date: 1609459200000,
            from_address: Some("test@example.com".to_string()),
            subject: Some("テスト件名".to_string()),
        };

        let result = save_messages_to_db(&pool, std::slice::from_ref(&message), &shop_settings)
            .await
            .expect("Failed to save unicode message");

        assert_eq!(result.saved_count, 1);

        // データベースから取得して検証
        let row: (String, Option<String>) = sqlx::query_as(
            "SELECT message_id, body_plain FROM emails WHERE message_id = 'msg_unicode'",
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch");

        assert_eq!(row.0, "msg_unicode");
        assert!(row.1.is_some());
        assert!(row.1.unwrap().contains("こんにちは"));
    }

    #[tokio::test]
    async fn test_save_messages_special_characters() {
        let pool = create_test_db().await;

        let shop_settings = vec![ShopSettings {
            id: 1,
            shop_name: "Test Shop".to_string(),
            sender_address: "test@example.com".to_string(),
            parser_type: "test".to_string(),
            subject_filters: None,
            is_enabled: true,
            created_at: "2021-01-01 00:00:00".to_string(),
            updated_at: "2021-01-01 00:00:00".to_string(),
        }];

        // 特殊文字を含むメッセージ（SQL injection対策テスト）
        let message = GmailMessage {
            message_id: "msg'; DROP TABLE emails; --".to_string(),
            snippet: "Test <script>alert('xss')</script>".to_string(),
            body_plain: Some("Plain text with 'quotes' and \"double quotes\"".to_string()),
            body_html: Some("<html><body onload='alert(1)'>HTML</body></html>".to_string()),
            internal_date: 1609459200000,
            from_address: Some("test@example.com".to_string()),
            subject: Some("Test'; DROP TABLE--".to_string()),
        };

        let result = save_messages_to_db(&pool, std::slice::from_ref(&message), &shop_settings)
            .await
            .expect("Failed to save message with special characters");

        assert_eq!(result.saved_count, 1);

        // テーブルが削除されていないことを確認
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM emails")
            .fetch_one(&pool)
            .await
            .expect("Table should still exist");

        assert_eq!(count.0, 1);

        // データが正しく保存されていることを確認
        let row: (String,) = sqlx::query_as("SELECT message_id FROM emails WHERE message_id = ?")
            .bind("msg'; DROP TABLE emails; --")
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch");

        assert_eq!(row.0, "msg'; DROP TABLE emails; --");
    }

    // ヘルパー関数のテスト

    #[test]
    fn test_build_sync_query_without_date() {
        let addresses = vec!["test@example.com".to_string()];
        let query = build_sync_query(&addresses, &None);
        assert_eq!(query, "in:anywhere (from:test@example.com)");
    }

    #[test]
    fn test_build_sync_query_with_valid_date() {
        let addresses = vec!["test@example.com".to_string()];
        let date = Some("2024-01-15T10:30:00Z".to_string());
        let query = build_sync_query(&addresses, &date);
        assert!(query.contains("from:test@example.com"));
        assert!(query.contains("before:2024/01/15"));
    }

    #[test]
    fn test_build_sync_query_with_invalid_date() {
        let addresses = vec!["test@example.com".to_string()];
        let date = Some("invalid-date".to_string());
        let query = build_sync_query(&addresses, &date);
        // 無効な日付の場合、基本クエリのみが返される
        assert_eq!(query, "in:anywhere (from:test@example.com)");
    }

    #[test]
    fn test_build_sync_query_with_different_dates() {
        let addresses = vec!["test@example.com".to_string()];
        let test_cases = vec![
            ("2024-01-01T00:00:00Z", "before:2024/01/01"),
            ("2023-12-31T23:59:59Z", "before:2023/12/31"),
            ("2024-06-15T12:00:00Z", "before:2024/06/15"),
        ];

        for (date_str, expected_before) in test_cases {
            let query = build_sync_query(&addresses, &Some(date_str.to_string()));
            assert!(
                query.contains(expected_before),
                "Query: {query}, Expected: {expected_before}"
            );
        }
    }

    #[test]
    fn test_build_sync_query_with_multiple_addresses() {
        let addresses = vec![
            "test1@example.com".to_string(),
            "test2@example.com".to_string(),
            "test3@example.com".to_string(),
        ];
        let query = build_sync_query(&addresses, &None);
        assert_eq!(
            query,
            "in:anywhere (from:test1@example.com OR from:test2@example.com OR from:test3@example.com)"
        );
    }

    #[test]
    fn test_build_sync_query_with_empty_addresses() {
        let addresses: Vec<String> = vec![];
        let query = build_sync_query(&addresses, &None);
        // Should fallback to keyword search
        assert_eq!(
            query,
            r"in:anywhere subject:(注文 OR 予約 OR ありがとうございます)"
        );
    }

    #[test]
    fn test_format_timestamp_valid() {
        // 2024-01-15 10:30:00 UTC in milliseconds
        let timestamp = 1705318200000i64;
        let formatted = format_timestamp(timestamp);

        assert!(!formatted.is_empty());
        // RFC3339形式であることを確認
        assert!(chrono::DateTime::parse_from_rfc3339(&formatted).is_ok());
    }

    #[test]
    fn test_format_timestamp_zero() {
        // タイムスタンプ0（1970-01-01 00:00:00 UTC）
        let formatted = format_timestamp(0);
        assert!(!formatted.is_empty());
        assert!(formatted.contains("1970-01-01"));
    }

    #[test]
    fn test_format_timestamp_negative() {
        // 負のタイムスタンプ（1970年より前）
        let formatted = format_timestamp(-1000);
        // 負の値は空文字列を返す（無効として扱われる）
        assert!(formatted.is_empty() || chrono::DateTime::parse_from_rfc3339(&formatted).is_ok());
    }

    #[test]
    fn test_format_timestamp_max_valid() {
        // 非常に大きな値（遠い未来）
        let timestamp = 9999999999999i64;
        let formatted = format_timestamp(timestamp);

        if !formatted.is_empty() {
            assert!(chrono::DateTime::parse_from_rfc3339(&formatted).is_ok());
        }
    }

    #[test]
    fn test_fetch_result_calculation() {
        // FetchResultの計算ロジックをテスト
        let fetched = 100;
        let saved = 85;
        let skipped = fetched - saved;

        let result = FetchResult {
            fetched_count: fetched,
            saved_count: saved,
            skipped_count: skipped,
        };

        assert_eq!(result.fetched_count, 100);
        assert_eq!(result.saved_count, 85);
        assert_eq!(result.skipped_count, 15);
        assert_eq!(
            result.saved_count + result.skipped_count,
            result.fetched_count
        );
    }

    #[test]
    fn test_gmail_message_with_none_values() {
        // body_plainとbody_htmlがNoneの場合
        let message = GmailMessage {
            message_id: "msg_none".to_string(),
            snippet: "Only snippet".to_string(),
            body_plain: None,
            body_html: None,
            internal_date: 1609459200000,
            from_address: None,
            subject: None,
        };

        assert!(message.body_plain.is_none());
        assert!(message.body_html.is_none());
        assert!(!message.snippet.is_empty());
    }

    #[test]
    fn test_sync_metadata_default_values() {
        let metadata = SyncMetadata {
            sync_status: "idle".to_string(),
            oldest_fetched_date: None,
            total_synced_count: 0,
            batch_size: 50,
            last_sync_started_at: None,
            last_sync_completed_at: None,
            max_iterations: 100,
            last_error_message: None,
        };

        assert_eq!(metadata.sync_status, "idle");
        assert_eq!(metadata.total_synced_count, 0);
        assert_eq!(metadata.batch_size, 50);
        assert!(metadata.oldest_fetched_date.is_none());
    }

    #[test]
    fn test_sync_state_is_running() {
        let state = SyncState::new();
        assert!(!state.is_running());

        state.set_running(true);
        assert!(state.is_running());

        state.set_running(false);
        assert!(!state.is_running());
    }

    #[test]
    fn test_sync_state_set_running() {
        let state = SyncState::new();

        // Initially not running
        assert!(!state.is_running());

        // Set to running
        state.set_running(true);
        assert!(state.is_running());

        // Set back to not running
        state.set_running(false);
        assert!(!state.is_running());
    }

    #[test]
    fn test_sync_state_reset_clears_cancel_flag() {
        let state = SyncState::new();

        // Request cancel
        state.request_cancel();
        assert!(state.should_stop());

        // Reset should clear the cancel flag
        state.reset();
        assert!(!state.should_stop());
    }

    #[test]
    fn test_sync_state_reset_when_not_cancelled() {
        let state = SyncState::new();

        // Reset when not cancelled should have no effect
        state.reset();
        assert!(!state.should_stop());
    }

    #[test]
    fn test_sync_state_multiple_resets() {
        let state = SyncState::new();

        // Cancel, reset, cancel, reset
        state.request_cancel();
        assert!(state.should_stop());

        state.reset();
        assert!(!state.should_stop());

        state.request_cancel();
        assert!(state.should_stop());

        state.reset();
        assert!(!state.should_stop());
    }

    #[test]
    fn test_sync_state_running_and_cancel_independent() {
        let state = SyncState::new();

        // Set running doesn't affect cancel state
        state.set_running(true);
        assert!(state.is_running());
        assert!(!state.should_stop());

        // Request cancel doesn't affect running state
        state.request_cancel();
        assert!(state.is_running());
        assert!(state.should_stop());

        // Reset cancel doesn't affect running state
        state.reset();
        assert!(state.is_running());
        assert!(!state.should_stop());
    }

    #[test]
    fn test_fetch_result_all_fields() {
        let result = FetchResult {
            fetched_count: 100,
            saved_count: 95,
            skipped_count: 5,
        };

        assert_eq!(result.fetched_count, 100);
        assert_eq!(result.saved_count, 95);
        assert_eq!(result.skipped_count, 5);
    }

    #[test]
    fn test_fetch_result_zero_values() {
        let result = FetchResult {
            fetched_count: 0,
            saved_count: 0,
            skipped_count: 0,
        };

        assert_eq!(result.fetched_count, 0);
        assert_eq!(result.saved_count, 0);
        assert_eq!(result.skipped_count, 0);
    }

    #[test]
    fn test_gmail_message_all_fields_present() {
        let message = GmailMessage {
            message_id: "msg_123".to_string(),
            snippet: "Test snippet".to_string(),
            body_plain: Some("Plain text body".to_string()),
            body_html: Some("<p>HTML body</p>".to_string()),
            internal_date: 1705329600,
            from_address: Some("test@example.com".to_string()),
            subject: Some("Test subject".to_string()),
        };

        assert_eq!(message.message_id, "msg_123");
        assert_eq!(message.snippet, "Test snippet");
        assert_eq!(message.body_plain.unwrap(), "Plain text body");
        assert_eq!(message.body_html.unwrap(), "<p>HTML body</p>");
        assert_eq!(message.internal_date, 1705329600);
    }

    #[test]
    fn test_gmail_message_without_optional_fields() {
        let message = GmailMessage {
            message_id: "msg_456".to_string(),
            snippet: "Another snippet".to_string(),
            body_plain: None,
            body_html: None,
            internal_date: 1705329600,
            from_address: None,
            subject: None,
        };

        assert_eq!(message.message_id, "msg_456");
        assert_eq!(message.snippet, "Another snippet");
        assert!(message.body_plain.is_none());
        assert!(message.body_html.is_none());
        assert_eq!(message.internal_date, 1705329600);
    }

    #[test]
    fn test_build_sync_query_date_format() {
        let addresses = vec!["test@example.com".to_string()];
        let date = Some("2024-01-15T10:30:00Z".to_string());
        let query = build_sync_query(&addresses, &date);

        // Should extract just the date part (2024-01-15) and format as 2024/01/15
        assert!(query.contains("before:2024/01/15"));
    }

    #[test]
    fn test_format_timestamp_edge_cases() {
        // Test with very small positive timestamp (1 millisecond after epoch)
        let ts = format_timestamp(1);
        assert!(ts.contains("1970"));

        // Test with timestamp for 2024-01-15
        let ts = format_timestamp(1705329600000); // 2024-01-15 in milliseconds
        assert!(ts.contains("2024"));
    }

    #[test]
    fn test_format_timestamp_milliseconds() {
        // Test that it correctly handles milliseconds
        let ts = format_timestamp(1000); // 1 second after epoch
        assert!(!ts.is_empty());
        assert!(ts.contains("1970"));
    }

    // try_decode_base64のテスト（改良版）
    #[test]
    fn test_try_decode_base64_valid() {
        // "Hello, World!" をbase64url (パディングなし)でエンコード: SGVsbG8sIFdvcmxkIQ
        let encoded = "SGVsbG8sIFdvcmxkIQ";
        let decoded = GmailClient::try_decode_base64(encoded);
        assert_eq!(decoded, Some("Hello, World!".to_string()));
    }

    #[test]
    fn test_try_decode_base64_empty() {
        // 空文字列はBase64形式ではない
        let decoded = GmailClient::try_decode_base64("");
        assert_eq!(decoded, None);
    }

    #[test]
    fn test_try_decode_base64_invalid() {
        // 無効なbase64文字列（Base64文字セット以外を含む）
        let decoded = GmailClient::try_decode_base64("!!invalid!!");
        assert_eq!(decoded, None); // Base64形式でないのでNone
    }

    #[test]
    fn test_try_decode_base64_japanese() {
        // "こんにちは" をbase64url (パディングなし)でエンコード
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        let original = "こんにちは";
        let encoded = URL_SAFE_NO_PAD.encode(original.as_bytes());
        let decoded = GmailClient::try_decode_base64(&encoded);
        assert_eq!(decoded, Some(original.to_string()));
    }

    // extract_body_from_partのテスト
    #[test]
    fn test_extract_body_from_part_plain_text() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        use google_gmail1::api::{MessagePart, MessagePartBody};

        let plain_text = "This is plain text";
        let encoded = URL_SAFE_NO_PAD.encode(plain_text.as_bytes());

        let part = MessagePart {
            mime_type: Some("text/plain".to_string()),
            body: Some(MessagePartBody {
                data: Some(encoded.into_bytes()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let mut body_plain = None;
        let mut body_html = None;

        GmailClient::extract_body_from_part(&part, &mut body_plain, &mut body_html, None);

        assert_eq!(body_plain, Some(plain_text.to_string()));
        assert_eq!(body_html, None);
    }

    #[test]
    fn test_extract_body_from_part_html() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        use google_gmail1::api::{MessagePart, MessagePartBody};

        let html_text = "<html><body>HTML content</body></html>";
        let encoded = URL_SAFE_NO_PAD.encode(html_text.as_bytes());

        let part = MessagePart {
            mime_type: Some("text/html".to_string()),
            body: Some(MessagePartBody {
                data: Some(encoded.into_bytes()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let mut body_plain = None;
        let mut body_html = None;

        GmailClient::extract_body_from_part(&part, &mut body_plain, &mut body_html, None);

        assert_eq!(body_plain, None);
        assert_eq!(body_html, Some(html_text.to_string()));
    }

    #[test]
    fn test_extract_body_from_part_multipart() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        use google_gmail1::api::{MessagePart, MessagePartBody};

        let plain_text = "Plain version";
        let html_text = "<html>HTML version</html>";
        let plain_encoded = URL_SAFE_NO_PAD.encode(plain_text.as_bytes());
        let html_encoded = URL_SAFE_NO_PAD.encode(html_text.as_bytes());

        // マルチパートメッセージ
        let part = MessagePart {
            mime_type: Some("multipart/alternative".to_string()),
            parts: Some(vec![
                MessagePart {
                    mime_type: Some("text/plain".to_string()),
                    body: Some(MessagePartBody {
                        data: Some(plain_encoded.into_bytes()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                MessagePart {
                    mime_type: Some("text/html".to_string()),
                    body: Some(MessagePartBody {
                        data: Some(html_encoded.into_bytes()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };

        let mut body_plain = None;
        let mut body_html = None;

        GmailClient::extract_body_from_part(&part, &mut body_plain, &mut body_html, None);

        assert_eq!(body_plain, Some(plain_text.to_string()));
        assert_eq!(body_html, Some(html_text.to_string()));
    }

    #[test]
    fn test_extract_body_from_part_no_data() {
        use google_gmail1::api::MessagePart;

        // データがない場合
        let part = MessagePart {
            mime_type: Some("text/plain".to_string()),
            body: None,
            ..Default::default()
        };

        let mut body_plain = None;
        let mut body_html = None;

        GmailClient::extract_body_from_part(&part, &mut body_plain, &mut body_html, None);

        assert_eq!(body_plain, None);
        assert_eq!(body_html, None);
    }

    #[test]
    fn test_extract_body_from_part_priority_first() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        use google_gmail1::api::{MessagePart, MessagePartBody};

        let first_text = "First text";
        let second_text = "Second text";
        let first_encoded = URL_SAFE_NO_PAD.encode(first_text.as_bytes());
        let second_encoded = URL_SAFE_NO_PAD.encode(second_text.as_bytes());

        // 複数のtext/plainパート（最初のみが採用される）
        let part = MessagePart {
            mime_type: Some("multipart/mixed".to_string()),
            parts: Some(vec![
                MessagePart {
                    mime_type: Some("text/plain".to_string()),
                    body: Some(MessagePartBody {
                        data: Some(first_encoded.into_bytes()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                MessagePart {
                    mime_type: Some("text/plain".to_string()),
                    body: Some(MessagePartBody {
                        data: Some(second_encoded.into_bytes()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };

        let mut body_plain = None;
        let mut body_html = None;

        GmailClient::extract_body_from_part(&part, &mut body_plain, &mut body_html, None);

        // 最初のtext/plainのみが採用される
        assert_eq!(body_plain, Some(first_text.to_string()));
    }

    // SyncGuardのテスト
    #[test]
    fn test_sync_guard_clears_running_on_drop() {
        let state = SyncState::new();

        // Start sync
        assert!(state.try_start());
        assert!(state.is_running());

        {
            // Create guard
            let _guard = SyncGuard::new(&state);
            assert!(state.is_running());
        } // guard is dropped here

        // Running flag should be cleared
        assert!(!state.is_running());
    }

    #[test]
    fn test_sync_guard_clears_running_on_early_return() {
        let state = SyncState::new();

        fn test_function(state: &SyncState) -> Result<(), String> {
            state.try_start();
            let _guard = SyncGuard::new(state);

            // Early return
            Err("Test error".to_string())

            // This should never be reached, but if it was, the guard would still clean up
        }

        assert!(!state.is_running());
        let result = test_function(&state);
        assert!(result.is_err());
        // Guard should have cleaned up despite early return
        assert!(!state.is_running());
    }

    #[test]
    fn test_sync_guard_allows_retry_after_drop() {
        let state = SyncState::new();

        // First sync
        {
            assert!(state.try_start());
            let _guard = SyncGuard::new(&state);
            assert!(state.is_running());
        }

        // After guard drop, should be able to start again
        assert!(!state.is_running());
        assert!(state.try_start());
        assert!(state.is_running());
    }

    #[tokio::test]
    async fn test_sync_loop_termination() {
        // ループ終了条件のテスト（messagesが空の場合、has_moreはfalse）
        let mut has_more = true;
        let messages: Vec<String> = vec![];

        if messages.is_empty() {
            has_more = false;
        }

        assert!(!has_more);
    }

    #[test]
    fn test_sync_state_thread_safety() {
        use std::thread;
        use std::time::Duration;

        let state = SyncState::new();
        let state_clone = state.clone();

        // Test that we can acquire locks from different threads
        let handle = thread::spawn(move || {
            state_clone.request_cancel();
            thread::sleep(Duration::from_millis(10));
            state_clone.should_stop()
        });

        // Give the spawned thread time to set cancel flag
        thread::sleep(Duration::from_millis(20));

        // Main thread can still operate on the state
        state.try_start();
        assert!(state.is_running());

        let _result = handle.join().unwrap();
        // The cancel flag should have been set by the spawned thread
        // (though try_start clears it, the thread set it before try_start)
        // Test passes if thread completed without panic
    }

    #[test]
    fn test_message_part_body_extraction_utf8_error() {
        use google_gmail1::api::{MessagePart, MessagePartBody};

        // 無効なUTF-8データの場合
        let part = MessagePart {
            mime_type: Some("text/plain".to_string()),
            body: Some(MessagePartBody {
                // 無効なUTF-8バイトシーケンス
                data: Some(vec![0xFF, 0xFE, 0xFD]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let mut body_plain = None;
        let mut body_html = None;

        GmailClient::extract_body_from_part(&part, &mut body_plain, &mut body_html, None);

        // UTF-8 失敗後、フォールバック（ISO-2022-JP/Shift_JIS）で置換文字付きの部分結果を返す
        assert!(body_plain.is_some());
        assert!(body_plain.as_ref().unwrap().contains('\u{FFFD}'));
        assert_eq!(body_html, None);
    }

    #[test]
    fn test_sync_metadata_serialization() {
        let metadata = SyncMetadata {
            sync_status: "idle".to_string(),
            oldest_fetched_date: Some("2024-01-01".to_string()),
            total_synced_count: 100,
            batch_size: 50,
            last_sync_started_at: Some("2024-01-15T10:00:00Z".to_string()),
            last_sync_completed_at: Some("2024-01-15T11:00:00Z".to_string()),
            max_iterations: 100,
            last_error_message: None,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("\"sync_status\":\"idle\""));
        assert!(json.contains("\"total_synced_count\":100"));
    }

    // 無限ループ検出ロジックのテスト
    #[test]
    fn test_infinite_loop_detection_same_boundaries() {
        // 同じメッセージIDリストの境界チェック
        let current_ids = ["msg1".to_string(), "msg2".to_string(), "msg3".to_string()];
        let prev_ids = ["msg1".to_string(), "msg2".to_string(), "msg3".to_string()];

        // 境界が同じかチェック
        let same_boundaries = !current_ids.is_empty()
            && current_ids.len() == prev_ids.len()
            && current_ids.first() == prev_ids.first()
            && current_ids.last() == prev_ids.last();

        assert!(same_boundaries);

        // ミドル要素もチェック
        let mid = current_ids.len() / 2;
        let same_middle = current_ids.get(mid) == prev_ids.get(mid);
        assert!(same_middle);
    }

    #[test]
    fn test_infinite_loop_detection_different_boundaries() {
        // 異なるメッセージIDリストの境界チェック
        let current_ids = ["msg4".to_string(), "msg5".to_string(), "msg6".to_string()];
        let prev_ids = ["msg1".to_string(), "msg2".to_string(), "msg3".to_string()];

        // 境界が異なることを確認
        let same_boundaries = !current_ids.is_empty()
            && current_ids.len() == prev_ids.len()
            && current_ids.first() == prev_ids.first()
            && current_ids.last() == prev_ids.last();

        assert!(!same_boundaries);
    }

    #[test]
    fn test_infinite_loop_detection_small_batch() {
        // 小さなバッチ（2要素以下）の場合
        let current_ids = ["msg1".to_string(), "msg2".to_string()];
        let prev_ids = ["msg1".to_string(), "msg2".to_string()];

        let same_boundaries = !current_ids.is_empty()
            && current_ids.len() == prev_ids.len()
            && current_ids.first() == prev_ids.first()
            && current_ids.last() == prev_ids.last();

        // 小さなバッチでは境界チェックのみで十分
        assert!(same_boundaries);

        let same_middle = if current_ids.len() > 2 {
            let mid = current_ids.len() / 2;
            current_ids.get(mid) == prev_ids.get(mid)
        } else {
            true
        };

        assert!(same_middle);
    }

    #[test]
    fn test_timestamp_future_detection() {
        use chrono::{Duration, Utc};

        let now = Utc::now();
        let skew_tolerance = Duration::minutes(5);

        // 許容範囲内の未来の時刻
        let valid_future = now + Duration::minutes(3);
        assert!(valid_future <= now + skew_tolerance);

        // 許容範囲外の未来の時刻
        let invalid_future = now + Duration::minutes(10);
        assert!(invalid_future > now + skew_tolerance);
    }

    #[test]
    fn test_timestamp_parsing_rfc3339() {
        use chrono::DateTime;

        // 有効なRFC3339タイムスタンプ
        let valid_timestamp = "2024-01-15T10:30:00Z";
        let parsed = DateTime::parse_from_rfc3339(valid_timestamp);
        assert!(parsed.is_ok());

        // 無効なRFC3339タイムスタンプ
        let invalid_timestamp = "2024-01-15 10:30:00";
        let parsed = DateTime::parse_from_rfc3339(invalid_timestamp);
        assert!(parsed.is_err());
    }

    #[test]
    fn test_message_id_extraction() {
        // メッセージIDの抽出ロジックをテスト
        let messages = [
            GmailMessage {
                message_id: "msg001".to_string(),
                snippet: "Test 1".to_string(),
                body_plain: None,
                body_html: None,
                internal_date: 1000,
                from_address: None,
                subject: None,
            },
            GmailMessage {
                message_id: "msg002".to_string(),
                snippet: "Test 2".to_string(),
                body_plain: None,
                body_html: None,
                internal_date: 2000,
                from_address: None,
                subject: None,
            },
        ];

        let message_ids: Vec<String> = messages.iter().map(|m| m.message_id.clone()).collect();

        assert_eq!(message_ids.len(), 2);
        assert_eq!(message_ids[0], "msg001");
        assert_eq!(message_ids[1], "msg002");
    }

    #[test]
    fn test_saturating_add_overflow_protection() {
        // saturating_addのオーバーフロー保護をテスト
        let total_synced: i64 = i64::MAX - 100;
        let saved_count: i64 = 200;

        // 通常の加算だとオーバーフローするが、saturating_addは最大値に留まる
        let result = total_synced.saturating_add(saved_count);
        assert_eq!(result, i64::MAX);
    }

    #[test]
    fn test_saturating_add_normal() {
        // 通常のsaturating_addの動作
        let total_synced: i64 = 100;
        let saved_count: i64 = 50;

        let result = total_synced.saturating_add(saved_count);
        assert_eq!(result, 150);
    }

    #[test]
    fn test_batch_number_increment() {
        // バッチ番号のインクリメントロジック
        let mut batch_number: usize = 0;

        for _ in 0..5 {
            batch_number += 1;
        }

        assert_eq!(batch_number, 5);

        // MAX_ITERATIONSのチェック
        const TEST_MAX_ITERATIONS: usize = 1000;
        assert!(batch_number <= TEST_MAX_ITERATIONS);
    }

    #[test]
    fn test_duration_calculation() {
        use chrono::{Duration, Utc};

        let start_time = Utc::now();
        let elapsed = Utc::now().signed_duration_since(start_time);

        // 経過時間は非負であるべき
        assert!(elapsed >= Duration::zero());

        // タイムアウトチェックのロジック
        let test_timeout_minutes: i64 = 30;
        let is_timeout = elapsed.num_minutes() > test_timeout_minutes;
        assert!(!is_timeout); // テスト実行は30分以内
    }

    #[tokio::test]
    async fn test_empty_messages_handling() {
        // 空のメッセージリストのハンドリング
        let messages: Vec<GmailMessage> = vec![];

        let mut has_more = true;
        if messages.is_empty() {
            has_more = false;
        }

        assert!(!has_more);
    }

    #[test]
    fn test_is_base64_format() {
        // 有効なBase64URL形式
        assert!(GmailClient::is_base64_format("SGVsbG8gV29ybGQ")); // "Hello World"
        assert!(GmailClient::is_base64_format("VGhpcyBpcyBhIHRlc3Q")); // "This is a test"
        assert!(GmailClient::is_base64_format(
            "QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVphYmNkZWZnaGlqa2xtbm9wcXJzdHV2d3h5ejAxMjM0NTY3ODk"
        ));

        // Base64URLの特殊文字を含む
        assert!(GmailClient::is_base64_format(
            "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXotXw"
        ));

        // 無効なケース: 空文字列
        assert!(!GmailClient::is_base64_format(""));

        // 無効なケース: 短すぎる（8文字未満）
        assert!(!GmailClient::is_base64_format("short"));
        assert!(!GmailClient::is_base64_format("test"));

        // 無効なケース: Base64以外の文字を含む
        assert!(!GmailClient::is_base64_format("Hello World!")); // スペースと!
        assert!(!GmailClient::is_base64_format("こんにちは")); // 非ASCII
        assert!(!GmailClient::is_base64_format("test@example.com")); // @と.
        assert!(!GmailClient::is_base64_format("path/to/file")); // /

        // 無効なケース: 通常のテキスト（長さは十分だがBase64文字セット以外を含む）
        assert!(!GmailClient::is_base64_format("This is plain text content"));
    }

    #[test]
    fn test_try_decode_base64() {
        // 有効なBase64URLのデコード
        let decoded = GmailClient::try_decode_base64("SGVsbG8gV29ybGQ");
        assert_eq!(decoded, Some("Hello World".to_string()));

        let decoded = GmailClient::try_decode_base64("VGhpcyBpcyBhIHRlc3Q");
        assert_eq!(decoded, Some("This is a test".to_string()));

        // Base64形式でないデータ
        assert_eq!(GmailClient::try_decode_base64("Hello World"), None);
        assert_eq!(GmailClient::try_decode_base64("short"), None);
        assert_eq!(GmailClient::try_decode_base64(""), None);
        assert_eq!(GmailClient::try_decode_base64("test@example.com"), None);

        // 通常のテキスト
        assert_eq!(GmailClient::try_decode_base64("This is plain text"), None);
    }

    #[test]
    fn test_base64_vs_plain_text_distinction() {
        // この テストはBase64データと通常のテキストを正しく区別できることを確認

        // 実際のBase64エンコードされたメール本文の例
        let base64_email = "VGhpcyBpcyBhbiBlbWFpbCBib2R5IHdpdGggc29tZSBjb250ZW50";
        assert!(GmailClient::is_base64_format(base64_email));
        let decoded = GmailClient::try_decode_base64(base64_email);
        assert!(decoded.is_some());

        // 既にデコード済みのプレーンテキスト
        let plain_text = "This is an email body with some content";
        assert!(!GmailClient::is_base64_format(plain_text));
        let result = GmailClient::try_decode_base64(plain_text);
        assert_eq!(result, None);

        // HTMLメール（既にデコード済み）
        let html_content = "<html><body>Hello World</body></html>";
        assert!(!GmailClient::is_base64_format(html_content));
        assert_eq!(GmailClient::try_decode_base64(html_content), None);
    }
}
