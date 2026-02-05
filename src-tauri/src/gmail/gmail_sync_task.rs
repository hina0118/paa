//! Gmail同期用 BatchTask 実装
//!
//! `BatchRunner` を使用してGmail同期を実行するための `BatchTask` 実装。
//!
//! # 設計
//! Gmail同期は現在ページング処理を行っているため、BatchTaskに完全に準拠させるには
//! 事前に全メッセージIDを取得する必要があります。
//!
//! このモジュールでは以下を提供します：
//!
//! - `GmailSyncInput`: 同期対象メッセージIDの入力データ
//! - `GmailSyncOutput`: 同期結果（保存されたメッセージ）
//! - `GmailSyncContext`: 同期に必要なコンテキスト
//! - `GmailSyncTask`: BatchTaskトレイト実装
//!
//! # フック活用
//! - `before_batch`: ショップ設定の取得、同期ステータスの更新
//! - `process_batch`: メッセージの取得（Gmail API）
//! - `after_batch`: メッセージのDB保存

use crate::batch_runner::BatchTask;
use crate::gmail::client::GmailMessage;
use crate::gmail_client::GmailClientTrait;
use crate::repository::{EmailRepository, ShopSettingsRepository};
use async_trait::async_trait;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Gmail同期タスクの入力（メッセージID）
#[derive(Debug, Clone)]
pub struct GmailSyncInput {
    /// メッセージID
    pub message_id: String,
}

/// Gmail同期タスクの出力（取得したメッセージ）
#[derive(Debug, Clone)]
pub struct GmailSyncOutput {
    /// 取得したメッセージ
    pub message: GmailMessage,
    /// DB保存が成功したか
    pub saved: bool,
}

/// ショップ設定のキャッシュ
#[derive(Debug, Clone, Default)]
pub struct ShopSettingsCacheForSync {
    /// 有効なショップ設定のリスト
    pub enabled_shops: Vec<crate::gmail::ShopSettings>,
}

/// Gmail同期のコンテキスト
pub struct GmailSyncContext<C, E, S>
where
    C: GmailClientTrait + 'static,
    E: EmailRepository + 'static,
    S: ShopSettingsRepository + 'static,
{
    /// Gmail クライアント
    pub gmail_client: Arc<C>,
    /// Email リポジトリ
    pub email_repo: Arc<E>,
    /// ShopSettings リポジトリ
    pub shop_settings_repo: Arc<S>,
    /// ショップ設定キャッシュ
    pub shop_settings_cache: Arc<Mutex<ShopSettingsCacheForSync>>,
}

/// Gmail同期タスク
///
/// 型パラメータ:
/// - `C`: Gmail クライアント
/// - `E`: Email リポジトリ
/// - `S`: ShopSettings リポジトリ
pub struct GmailSyncTask<C, E, S>
where
    C: GmailClientTrait + 'static,
    E: EmailRepository + 'static,
    S: ShopSettingsRepository + 'static,
{
    _phantom: PhantomData<(C, E, S)>,
}

/// タスク名
pub const GMAIL_SYNC_TASK_NAME: &str = "メール同期";
/// イベント名
pub const GMAIL_SYNC_EVENT_NAME: &str = "batch-progress";

impl<C, E, S> GmailSyncTask<C, E, S>
where
    C: GmailClientTrait + 'static,
    E: EmailRepository + 'static,
    S: ShopSettingsRepository + 'static,
{
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<C, E, S> Default for GmailSyncTask<C, E, S>
where
    C: GmailClientTrait + 'static,
    E: EmailRepository + 'static,
    S: ShopSettingsRepository + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// 全メッセージIDを取得するヘルパー関数
///
/// ページングを使用して全メッセージIDを取得します。
/// BatchRunner で処理する前に呼び出すことで、全メッセージIDを事前に取得できます。
pub async fn fetch_all_message_ids<C: GmailClientTrait>(
    client: &C,
    query: &str,
    max_results_per_page: u32,
    max_total: Option<usize>,
) -> Result<Vec<String>, String> {
    let mut all_ids: Vec<String> = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let (ids, next_token) = client
            .list_message_ids(query, max_results_per_page, page_token)
            .await?;

        if ids.is_empty() {
            break;
        }

        all_ids.extend(ids);

        // 最大件数に達したら終了
        if let Some(max) = max_total {
            if all_ids.len() >= max {
                all_ids.truncate(max);
                break;
            }
        }

        match next_token {
            Some(token) => page_token = Some(token),
            None => break,
        }
    }

    log::info!(
        "[Gmail Sync] Fetched {} message IDs (query: {}...)",
        all_ids.len(),
        query.chars().take(50).collect::<String>()
    );

    Ok(all_ids)
}

#[async_trait]
impl<C, E, S> BatchTask for GmailSyncTask<C, E, S>
where
    C: GmailClientTrait + 'static,
    E: EmailRepository + 'static,
    S: ShopSettingsRepository + 'static,
{
    type Input = GmailSyncInput;
    type Output = GmailSyncOutput;
    type Context = GmailSyncContext<C, E, S>;

    fn name(&self) -> &str {
        GMAIL_SYNC_TASK_NAME
    }

    fn event_name(&self) -> &str {
        GMAIL_SYNC_EVENT_NAME
    }

    /// バッチ処理前にショップ設定を取得
    async fn before_batch(
        &self,
        _inputs: &[Self::Input],
        context: &Self::Context,
    ) -> Result<(), String> {
        log::debug!("[{}] before_batch: Loading shop settings", self.name());

        // ショップ設定を取得
        let enabled_shops = context
            .shop_settings_repo
            .get_enabled()
            .await
            .map_err(|e| format!("Failed to fetch shop settings: {e}"))?;

        // キャッシュに保存
        let mut cache = context.shop_settings_cache.lock().await;
        cache.enabled_shops = enabled_shops;

        log::info!(
            "[{}] Shop settings loaded: {} entries",
            self.name(),
            cache.enabled_shops.len()
        );

        Ok(())
    }

    /// メッセージを取得
    async fn process_batch(
        &self,
        inputs: Vec<Self::Input>,
        context: &Self::Context,
    ) -> Vec<Result<Self::Output, String>> {
        let mut results: Vec<Result<Self::Output, String>> = Vec::with_capacity(inputs.len());

        for input in inputs {
            match context.gmail_client.get_message(&input.message_id).await {
                Ok(message) => {
                    results.push(Ok(GmailSyncOutput {
                        message,
                        saved: false, // after_batch で保存
                    }));
                }
                Err(e) => {
                    log::warn!(
                        "[{}] Failed to fetch message {}: {}",
                        self.name(),
                        input.message_id,
                        e
                    );
                    results.push(Err(format!(
                        "Failed to fetch message {}: {}",
                        input.message_id, e
                    )));
                }
            }
        }

        results
    }

    /// 取得したメッセージをDBに保存
    async fn after_batch(
        &self,
        batch_number: usize,
        results: &[Result<Self::Output, String>],
        context: &Self::Context,
    ) -> Result<(), String> {
        log::debug!(
            "[{}] after_batch: batch {} with {} results",
            self.name(),
            batch_number,
            results.len()
        );

        let cache = context.shop_settings_cache.lock().await;
        let enabled_shops = &cache.enabled_shops;

        let mut saved_count = 0;
        let mut save_errors = 0;

        // 成功したメッセージを収集
        let messages: Vec<GmailMessage> = results
            .iter()
            .filter_map(|r| r.as_ref().ok())
            .map(|o| o.message.clone())
            .collect();

        if messages.is_empty() {
            log::info!(
                "[{}] Batch {} complete: no messages to save",
                self.name(),
                batch_number
            );
            return Ok(());
        }

        // DBに保存
        match crate::gmail::client::save_messages_to_db_with_repo(
            context.email_repo.as_ref(),
            messages,
            enabled_shops,
        )
        .await
        {
            Ok(fetch_result) => {
                saved_count = fetch_result.saved_count;
                log::info!(
                    "[{}] Batch {} complete: {} saved, {} skipped",
                    self.name(),
                    batch_number,
                    fetch_result.saved_count,
                    fetch_result.skipped_count
                );
            }
            Err(e) => {
                log::error!(
                    "[{}] Failed to save messages in batch {}: {}",
                    self.name(),
                    batch_number,
                    e
                );
                save_errors += 1;
            }
        }

        // 成功件数と失敗件数をログ
        let success = results.iter().filter(|r| r.is_ok()).count();
        let failed = results.iter().filter(|r| r.is_err()).count();
        log::info!(
            "[{}] Batch {} summary: {} fetched, {} failed, {} saved, {} save_errors",
            self.name(),
            batch_number,
            success,
            failed,
            saved_count,
            save_errors
        );

        Ok(())
    }

    /// 単一アイテムの処理
    async fn process(
        &self,
        input: Self::Input,
        context: &Self::Context,
    ) -> Result<Self::Output, String> {
        let message = context.gmail_client.get_message(&input.message_id).await?;

        Ok(GmailSyncOutput {
            message,
            saved: false,
        })
    }
}

/// 入力データを生成するヘルパー関数
pub fn create_sync_input(message_id: String) -> GmailSyncInput {
    GmailSyncInput { message_id }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sync_input() {
        let input = create_sync_input("msg-12345".to_string());
        assert_eq!(input.message_id, "msg-12345");
    }

    #[test]
    fn test_task_name_and_event() {
        use crate::gmail_client::MockGmailClientTrait;
        use crate::repository::{MockEmailRepository, MockShopSettingsRepository};

        let task: GmailSyncTask<
            MockGmailClientTrait,
            MockEmailRepository,
            MockShopSettingsRepository,
        > = GmailSyncTask::new();
        assert_eq!(task.name(), "メール同期");
        assert_eq!(task.event_name(), "batch-progress");
    }
}
