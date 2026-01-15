use google_gmail1::{hyper_rustls, Gmail};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use yup_oauth2 as oauth2;

// カスタムInstalledFlowDelegateでブラウザを自動的に開く
struct CustomFlowDelegate;

impl oauth2::authenticator_delegate::InstalledFlowDelegate for CustomFlowDelegate {
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        need_code: bool,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            log::info!("Opening browser with URL: {}", url);

            // ブラウザで認証URLを開く
            if let Err(e) = webbrowser::open(url) {
                log::warn!("Failed to open browser automatically: {}", e);
                log::warn!("Please open this URL manually in your browser:");
                log::warn!("{}", url);
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
    pub body_plain: Option<String>,
    pub body_html: Option<String>,
    pub internal_date: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FetchResult {
    pub fetched_count: usize,
    pub saved_count: usize,
    pub skipped_count: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct SyncProgressEvent {
    pub batch_number: usize,
    pub batch_size: usize,
    pub total_synced: usize,
    pub newly_saved: usize,
    pub status_message: String,
    pub is_complete: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncMetadata {
    pub sync_status: String,
    pub oldest_fetched_date: Option<String>,
    pub total_synced_count: i64,
    pub batch_size: i64,
    pub last_sync_started_at: Option<String>,
    pub last_sync_completed_at: Option<String>,
}

/// Synchronization state for Gmail sync operations
///
/// # Lock Ordering
/// To prevent deadlock, always acquire locks in this order:
/// 1. should_cancel
/// 2. is_running
///
/// This ordering must be maintained consistently throughout the codebase.
#[derive(Clone)]
pub struct SyncState {
    pub should_cancel: Arc<Mutex<bool>>,
    pub is_running: Arc<Mutex<bool>>,
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            should_cancel: Arc::new(Mutex::new(false)),
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    pub fn request_cancel(&self) {
        if let Ok(mut cancel) = self.should_cancel.lock() {
            *cancel = true;
        }
    }

    pub fn should_stop(&self) -> bool {
        self.should_cancel.lock().map(|c| *c).unwrap_or(false)
    }

    pub fn reset(&self) {
        if let Ok(mut cancel) = self.should_cancel.lock() {
            *cancel = false;
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running.lock().map(|r| *r).unwrap_or(false)
    }

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
        let mut cancel = match self.should_cancel.lock() {
            Ok(guard) => guard,
            Err(_) => {
                log::error!(
                    "Failed to acquire should_cancel lock in try_start (mutex poisoned or unavailable)"
                );
                return false;
            }
        };

        // Then, acquire the running state lock. Lock order is consistent to avoid deadlocks.
        let mut is_running = match self.is_running.lock() {
            Ok(guard) => guard,
            Err(_) => {
                log::error!(
                    "Failed to acquire is_running lock in try_start (mutex poisoned or unavailable)"
                );
                return false;
            }
        };

        // If we're already running, do not change any flags.
        if *is_running {
            return false;
        }

        // Start running and clear any pending cancellation atomically with respect to
        // request_cancel().
        *is_running = true;
        *cancel = false;

        true
    }
}

/// RAII guard that automatically resets the running flag when dropped
/// This ensures cleanup happens even on early returns or panics
struct SyncGuard<'a> {
    sync_state: &'a SyncState,
}

impl<'a> SyncGuard<'a> {
    fn new(sync_state: &'a SyncState) -> Self {
        Self { sync_state }
    }
}

impl<'a> Drop for SyncGuard<'a> {
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
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;

        std::fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Failed to create app data dir: {}", e))?;

        // DBファイルと同じディレクトリに配置
        let token_path = app_data_dir.join("gmail_token.json");
        let client_secret_path = app_data_dir.join("client_secret.json");

        if !client_secret_path.exists() {
            return Err(format!(
                "Client secret file not found. Please place client_secret.json at: {}\n\nThis is the same directory where paa_data.db is stored.",
                client_secret_path.display()
            ));
        }

        let auth = Self::authenticate(&client_secret_path, &token_path).await?;

        // トークンを取得して認証を確実にする
        // gmail.readonlyスコープのみを使用（デスクトップアプリケーションに必要な最小限の権限）
        log::info!("Requesting OAuth token...");
        let _token = auth
            .token(&["https://www.googleapis.com/auth/gmail.readonly"])
            .await
            .map_err(|e| format!("Failed to get OAuth token: {}", e))?;
        log::info!("OAuth token obtained successfully");

        // Gmail Hub用のHTTPコネクタとクライアントを作成
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|e| format!("Failed to create HTTPS connector: {}", e))?
            .https_or_http()
            .enable_http1()
            .build();

        let client = Client::builder(TokioExecutor::new()).build(https);

        let hub = Gmail::new(client, auth);

        Ok(Self { hub })
    }

    async fn authenticate(
        client_secret_path: &PathBuf,
        token_path: &PathBuf,
    ) -> Result<oauth2::authenticator::Authenticator<hyper_rustls::HttpsConnector<HttpConnector>>, String>
    {
        let secret = oauth2::read_application_secret(client_secret_path)
            .await
            .map_err(|e| format!("Failed to read client secret: {}", e))?;

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
                "Failed to create authenticator: {}\n\n\
                If a browser window didn't open, please check the console for the authentication URL and open it manually.\n\
                URL format: https://accounts.google.com/o/oauth2/auth?...",
                e
            )
        })?;

        Ok(auth)
    }

    pub async fn fetch_messages(&self, query: &str) -> Result<Vec<GmailMessage>, String> {
        let mut all_messages = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut req = self.hub.users().messages_list("me").q(query);

            if let Some(token) = page_token {
                req = req.page_token(&token);
            }

            let (_, result) = req
                .doit()
                .await
                .map_err(|e| format!("Failed to list messages: {}", e))?;

            if let Some(messages) = result.messages {
                // メッセージIDを収集
                let message_ids: Vec<String> = messages
                    .iter()
                    .filter_map(|msg| msg.id.clone())
                    .collect();

                log::info!("Fetching {} messages in parallel batches", message_ids.len());

                // 順次処理でメッセージを取得
                // 注: 並列処理はライフタイムの問題とGmail API制限により複雑
                // 将来的な改善: tokio::spawn + Arc<Mutex<Hub>>の使用を検討
                for message_id in message_ids {
                    match self.get_message(&message_id).await {
                        Ok(msg) => all_messages.push(msg),
                        Err(e) => log::warn!("Failed to fetch message {}: {}", message_id, e),
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
        log::debug!("Fetching message: {}", message_id);

        let (response, message) = self
            .hub
            .users()
            .messages_get("me", message_id)
            .format("full")
            .doit()
            .await
            .map_err(|e| format!("Failed to get message {}: {}", message_id, e))?;

        log::debug!("Response status: {:?}", response.status());

        let snippet = message.snippet.unwrap_or_default();
        let internal_date = message.internal_date.unwrap_or(0);

        let mut body_plain: Option<String> = None;
        let mut body_html: Option<String> = None;

        // 再帰的にMIMEパートを解析
        if let Some(payload) = &message.payload {
            Self::extract_body_from_part(payload, &mut body_plain, &mut body_html);
        }

        Ok(GmailMessage {
            message_id: message_id.to_string(),
            snippet,
            body_plain,
            body_html,
            internal_date,
        })
    }

    fn decode_base64(data: &str) -> String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

        // Gmail APIはbase64url形式（パディングなし）でエンコードされた文字列を返す
        match URL_SAFE_NO_PAD.decode(data) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
            Err(_) => {
                log::warn!("Failed to decode base64 data, returning empty string");
                String::new()
            }
        }
    }

    // 再帰的にMIMEパートを解析する
    fn extract_body_from_part(
        part: &google_gmail1::api::MessagePart,
        body_plain: &mut Option<String>,
        body_html: &mut Option<String>,
    ) {
        // 現在のパートのbodyをチェック
        if let Some(mime_type) = &part.mime_type {
            if let Some(body) = &part.body {
                if let Some(data) = &body.data {
                    if let Ok(data_str) = std::str::from_utf8(data) {
                        let decoded = Self::decode_base64(data_str);
                        match mime_type.as_ref() {
                            "text/plain" if body_plain.is_none() => {
                                *body_plain = Some(decoded);
                            }
                            "text/html" if body_html.is_none() => {
                                *body_html = Some(decoded);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // 子パートを再帰的に処理
        if let Some(parts) = &part.parts {
            for child_part in parts {
                Self::extract_body_from_part(child_part, body_plain, body_html);
            }
        }
    }
}

pub async fn save_messages_to_db(
    pool: &SqlitePool,
    messages: &[GmailMessage],
) -> Result<FetchResult, String> {
    log::info!("Saving {} messages to database using sqlx", messages.len());

    let mut saved_count = 0;
    let mut skipped_count = 0;

    // トランザクションを使用してバッチ処理
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to begin transaction: {}", e))?;

    for msg in messages {
        let result = sqlx::query(
            r#"
            INSERT INTO emails (message_id, body_plain, body_html, internal_date)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(message_id) DO NOTHING
            "#,
        )
        .bind(&msg.message_id)
        .bind(&msg.body_plain)
        .bind(&msg.body_html)
        .bind(msg.internal_date)
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
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    log::info!(
        "Saved {} messages, skipped {} duplicates",
        saved_count,
        skipped_count
    );

    Ok(FetchResult {
        fetched_count: messages.len(),
        saved_count,
        skipped_count,
    })
}

// Helper function to build Gmail query with date constraint
fn build_sync_query(oldest_date: &Option<String>) -> String {
    let base_query = r#"subject:(注文 OR 予約 OR ありがとうございます)"#;

    if let Some(date) = oldest_date {
        // Parse and format for Gmail query (YYYY/MM/DD).
        // This ensures the date is validated and formatted correctly; if parsing fails,
        // the date filter is omitted and the base query is used without a date constraint.
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date) {
            let before_date = dt.format("%Y/%m/%d");
            return format!("{} before:{}", base_query, before_date);
        } else {
            // If parsing fails, log warning and use base query without date filter
            log::warn!("Invalid date format in oldest_date, ignoring date constraint: {}", date);
        }
    }

    base_query.to_string()
}

// Helper function to fetch a batch of messages
async fn fetch_batch(
    client: &GmailClient,
    query: &str,
    max_results: usize,
) -> Result<Vec<GmailMessage>, String> {
    let req = client.hub.users().messages_list("me")
        .q(query)
        .max_results(max_results as u32);

    let (_, result) = req.doit().await
        .map_err(|e| format!("Failed to list messages: {}", e))?;

    let mut messages = Vec::new();

    if let Some(msg_list) = result.messages {
        for msg in msg_list {
            if let Some(id) = msg.id {
                match client.get_message(&id).await {
                    Ok(full_msg) => messages.push(full_msg),
                    Err(e) => log::warn!("Failed to fetch message {}: {}", id, e),
                }
            }
        }
    }

    Ok(messages)
}

// Helper function to format timestamp as RFC3339
fn format_timestamp(internal_date: i64) -> String {
    chrono::DateTime::from_timestamp_millis(internal_date)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
}

// Helper function to update sync status to 'error' on early exit
async fn update_sync_error_status(pool: &SqlitePool) {
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(e) = sqlx::query(
        "UPDATE sync_metadata SET sync_status = 'error', last_sync_completed_at = ?1 WHERE id = 1"
    )
    .bind(&now)
    .execute(pool)
    .await
    {
        log::error!("Failed to update error status: {}", e);
    }
}

// Main incremental sync function
pub async fn sync_gmail_incremental(
    app_handle: &tauri::AppHandle,
    pool: &SqlitePool,
    sync_state: &SyncState,
    batch_size: usize,
) -> Result<(), String> {
    const DEFAULT_BATCH_SIZE: usize = 50;
    // NOTE: MAX_ITERATIONS and SYNC_TIMEOUT_MINUTES are intentionally hard-coded safety limits.
    // - MAX_ITERATIONS prevents a logic error from causing an infinite incremental sync loop.
    // - SYNC_TIMEOUT_MINUTES bounds how long a single sync attempt may run to avoid monopolizing resources.
    //
    // The effective upper bound on how many messages a single incremental sync invocation will process is:
    //     MAX_ITERATIONS * batch_size
    // With the current defaults (MAX_ITERATIONS = 1000 and DEFAULT_BATCH_SIZE = 50), a single run is
    // therefore expected to handle up to approximately 50,000 messages.
    //
    // For deployments with significantly larger mailboxes, consider:
    //   * Increasing `batch_size` via the existing configuration/caller of this function, and/or
    //   * Relying on multiple incremental sync runs to cover the full mailbox.
    //
    // The constants below are intentionally conservative safety limits. Changing them affects behavior
    // (e.g., by allowing longer or larger sync runs) and should be treated as a deliberate behavior change
    // and tracked in the external issue tracker rather than via an in-code TODO.
    //
    // FUTURE ENHANCEMENT: Consider making these configurable via the database `sync_metadata` table
    // (similar to `batch_size`) or through application configuration, with these values as sensible
    // defaults. This would allow operators to adjust limits for their deployment without code changes.
    const MAX_ITERATIONS: usize = 1000; // Prevent infinite loops
    const SYNC_TIMEOUT_MINUTES: i64 = 30; // Maximum sync duration (in minutes) for a single sync attempt

    let batch_size = if batch_size > 0 { batch_size } else { DEFAULT_BATCH_SIZE };

    // Atomically check and set running flag (also resets cancellation flag internally)
    if !sync_state.try_start() {
        return Err("Sync is already in progress".to_string());
    }

    // Create RAII guard to ensure running flag is cleared on function exit
    let _guard = SyncGuard::new(sync_state);

    // Update sync status to 'syncing'
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(e) = sqlx::query(
        "UPDATE sync_metadata SET sync_status = 'syncing', last_sync_started_at = ?1 WHERE id = 1"
    )
    .bind(&now)
    .execute(pool)
    .await
    {
        update_sync_error_status(pool).await;
        return Err(format!("Failed to update sync status: {}", e));
    }

    // Get oldest fetched date and batch size from metadata
    let metadata: Option<(Option<String>, i64, i64)> = match sqlx::query_as(
        "SELECT oldest_fetched_date, total_synced_count, batch_size FROM sync_metadata WHERE id = 1"
    )
    .fetch_optional(pool)
    .await
    {
        Ok(m) => m,
        Err(e) => {
            update_sync_error_status(pool).await;
            return Err(format!("Failed to fetch sync metadata: {}", e));
        }
    };

    let (mut oldest_date, mut total_synced, db_batch_size) = metadata.unwrap_or((None, 0, batch_size as i64));
    let effective_batch_size = if db_batch_size > 0 { db_batch_size as usize } else { batch_size };

    // Initialize Gmail client
    let client = match GmailClient::new(app_handle).await {
        Ok(c) => c,
        Err(e) => {
            update_sync_error_status(pool).await;
            return Err(e);
        }
    };
    let mut batch_number = 0;
    let mut has_more = true;
    let sync_start_time = chrono::Utc::now();
    let mut previous_message_ids: Option<Vec<String>> = None;
    while has_more && !sync_state.should_stop() {
        batch_number += 1;
        // Check iteration limit to prevent infinite loops
        if batch_number > MAX_ITERATIONS {
            log::warn!("Reached maximum iteration limit ({}), stopping sync", MAX_ITERATIONS);
            break;
        }
        // Check timeout to prevent indefinite sync
        let elapsed = chrono::Utc::now().signed_duration_since(sync_start_time);
        if elapsed.num_minutes() > SYNC_TIMEOUT_MINUTES {
            log::warn!("Sync timeout reached ({} minutes), stopping sync", SYNC_TIMEOUT_MINUTES);
            break;
        }
        // Store the oldest_date before this fetch to detect infinite loop conditions
        let oldest_date_before_fetch = oldest_date.clone();
        // Build query with date constraint
        let query = build_sync_query(&oldest_date);
        log::info!("Batch {}: Fetching up to {} messages with query: {}", batch_number, effective_batch_size, query);
        // Fetch batch of messages
        let messages = match fetch_batch(&client, &query, effective_batch_size).await {
            Ok(m) => m,
            Err(e) => {
                update_sync_error_status(pool).await;
                return Err(e);
            }
        };
        if messages.is_empty() {
            has_more = false;
            log::info!("No more messages to fetch");
            break;
        }
        log::info!("Batch {}: Fetched {} messages", batch_number, messages.len());

        // Extract message IDs for infinite loop detection
        let current_message_ids: Vec<String> = messages.iter()
            .map(|m| m.message_id.clone())
            .collect();

        // Save to database
        let result = match save_messages_to_db(pool, &messages).await {
            Ok(r) => r,
            Err(e) => {
                update_sync_error_status(pool).await;
                return Err(e);
            }
        };
        total_synced = total_synced.saturating_add(result.saved_count as i64);
        // Update oldest fetched date
        // Note: messages is guaranteed to be non-empty at this point (checked above with messages.is_empty())
        // min_by_key returns Some because iterator is non-empty
        let new_oldest = match messages.iter()
            .min_by_key(|m| m.internal_date)
            .map(|m| format_timestamp(m.internal_date))
        {
            Some(ts) => ts,
            None => {
                update_sync_error_status(pool).await;
                return Err(format!(
                    "Logic error: min_by_key returned None on non-empty messages while updating sync metadata. batch_number={}, messages_len={}",
                    batch_number,
                    messages.len()
                ));
            }
        };

        if let Err(e) = sqlx::query(
            "UPDATE sync_metadata SET oldest_fetched_date = ?1, total_synced_count = ?2 WHERE id = 1"
        )
        .bind(&new_oldest)
        .bind(total_synced)
        .execute(pool)
        .await
        {
            update_sync_error_status(pool).await;
            return Err(format!("Failed to update metadata: {}", e));
        }

        // Detect infinite loop BEFORE updating oldest_date: same messages being returned repeatedly
        // This can happen when multiple messages have identical timestamps (common in batch imports)
        // For performance on large message sets, avoid full Vec equality and instead compare
        // length plus first/middle/last IDs as a heuristic for "same batch".
        // NOTE: This heuristic may produce false positives if different batches have the same
        // boundary IDs, but combined with the timestamp check this is extremely unlikely in practice.
        if let Some(ref prev_ids) = previous_message_ids {
            // Compare the new_oldest timestamp value with the timestamp from oldest_date_before_fetch
            let same_boundaries = !current_message_ids.is_empty()
                && current_message_ids.len() == prev_ids.len()
                && current_message_ids.first() == prev_ids.first()
                && current_message_ids.last() == prev_ids.last();

            // Also check middle element to reduce false positives
            let same_middle = if current_message_ids.len() > 2 {
                let mid = current_message_ids.len() / 2;
                current_message_ids.get(mid) == prev_ids.get(mid)
            } else {
                true // For small batches, boundary check is sufficient
            };

            if Some(&new_oldest) == oldest_date_before_fetch.as_ref()
                && same_boundaries
                && same_middle
            {
                log::warn!("Same message IDs returned despite fetching messages, stopping to prevent infinite loop");
                has_more = false;
            }
        }

        // Validate that new_oldest is a reasonable timestamp (not unreasonably far in the future)
        // This catches cases where format_timestamp falls back to Utc::now() for invalid timestamps
        match chrono::DateTime::parse_from_rfc3339(&new_oldest) {
            Ok(new_oldest_dt) => {
                let now = chrono::Utc::now();
                // Allow a small clock skew tolerance (e.g., 5 minutes) between client and Gmail servers
                let new_oldest_utc = new_oldest_dt.with_timezone(&chrono::Utc);
                let skew_tolerance = chrono::Duration::minutes(5);
                if new_oldest_utc > now + skew_tolerance {
                    log::error!(
                        "Invalid timestamp detected: new_oldest ({}) is significantly in the future, indicates timestamp parsing failure",
                        new_oldest
                    );
                    update_sync_error_status(pool).await;
                    return Err("Invalid message timestamp detected (future date beyond allowed clock skew). This indicates a data integrity issue.".to_string());
                }
            }
            Err(e) => {
                // Parsing failure here indicates a potential issue in timestamp formatting logic.
                // We continue as before but log a warning for observability.
                log::warn!(
                    "Failed to parse new_oldest timestamp as RFC3339 (value: '{}'): {}",
                    new_oldest,
                    e
                );
            }
        }
        previous_message_ids = Some(current_message_ids);

        // Update the oldest_date variable for the next iteration
        oldest_date = Some(new_oldest.clone());
        // Emit progress event
        let progress = SyncProgressEvent {
            batch_number,
            batch_size: messages.len(),
            total_synced: total_synced as usize,
            newly_saved: result.saved_count,
            status_message: format!("Batch {} complete: {} new emails", batch_number, result.saved_count),
            is_complete: false,
            error: None,
        };
        if let Err(e) = app_handle.emit("sync-progress", progress) {
            update_sync_error_status(pool).await;
            return Err(format!("Failed to emit progress: {}", e));
        }
        // Check if we got fewer messages than requested (end of results)
        if messages.len() < effective_batch_size {
            has_more = false;
            log::info!("Received fewer messages than batch size, sync complete");
        }
    }
    // Determine final status
    let final_status = if sync_state.should_stop() {
        "paused"
    } else {
        "idle"
    };
    // Update sync metadata
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(e) = sqlx::query(
        "UPDATE sync_metadata SET sync_status = ?1, last_sync_completed_at = ?2 WHERE id = 1"
    )
    .bind(final_status)
    .bind(&now)
    .execute(pool)
    .await
    {
        return Err(format!("Failed to update final status: {}", e));
    }
    // Emit completion event
    let completion = SyncProgressEvent {
        batch_number,
        batch_size: 0,
        total_synced: total_synced as usize,
        newly_saved: 0,
        status_message: if sync_state.should_stop() {
            "Sync cancelled by user".to_string()
        } else {
            "Sync completed successfully".to_string()
        },
        is_complete: true,
        error: None,
    };
    if let Err(e) = app_handle.emit("sync-progress", completion) {
        return Err(format!("Failed to emit completion: {}", e));
    }
    Ok(())
}
