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
    /// メタデータ段階でフィルタ除外されたか
    pub filtered_out: bool,
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

    /// メッセージを2段階で取得（メタデータ → フィルタ → 本文取得）
    ///
    /// Phase 1: メタデータのみ取得してショップ設定でフィルタリング
    /// Phase 2: 条件に合うメッセージのみ本文(full)を取得
    async fn process_batch(
        &self,
        inputs: Vec<Self::Input>,
        context: &Self::Context,
    ) -> Vec<Result<Self::Output, String>> {
        let mut results: Vec<Result<Self::Output, String>> = Vec::with_capacity(inputs.len());

        let cache = context.shop_settings_cache.lock().await;
        let enabled_shops = cache.enabled_shops.clone();
        drop(cache);

        // Phase 1: メタデータ取得 + フィルタリング
        let mut candidates: Vec<(String, usize)> = Vec::new(); // (message_id, results内のindex)

        for input in &inputs {
            match context
                .gmail_client
                .get_message_metadata(&input.message_id)
                .await
            {
                Ok(metadata) => {
                    if crate::logic::sync_logic::should_save_message(&metadata, &enabled_shops) {
                        // フィルタ通過 → Phase 2 で本文取得する候補
                        let idx = results.len();
                        results.push(Ok(GmailSyncOutput {
                            message: metadata,
                            saved: false,
                            filtered_out: false, // Phase 2 でメッセージ本文を含む完全版に上書き予定
                        }));
                        candidates.push((input.message_id.clone(), idx));
                    } else {
                        log::debug!(
                            "[{}] Message {} filtered out at metadata phase",
                            self.name(),
                            input.message_id,
                        );
                        results.push(Ok(GmailSyncOutput {
                            message: metadata,
                            saved: false,
                            filtered_out: true,
                        }));
                    }
                }
                Err(e) => {
                    log::warn!(
                        "[{}] Failed to fetch metadata for {}: {}",
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

        let total = inputs.len();
        let candidate_count = candidates.len();
        let filtered_out = total - candidate_count - results.iter().filter(|r| r.is_err()).count();
        log::info!(
            "[{}] Metadata phase: {} total, {} candidates, {} filtered out",
            self.name(),
            total,
            candidate_count,
            filtered_out,
        );

        // Phase 2: 候補のみ本文(full)を取得
        for (message_id, idx) in candidates {
            match context.gmail_client.get_message(&message_id).await {
                Ok(full_message) => {
                    results[idx] = Ok(GmailSyncOutput {
                        message: full_message,
                        saved: false,
                        filtered_out: false,
                    });
                }
                Err(e) => {
                    log::warn!(
                        "[{}] Failed to fetch full message {}: {}",
                        self.name(),
                        message_id,
                        e
                    );
                    results[idx] = Err(format!("Failed to fetch message {}: {}", message_id, e));
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

        // 成功したメッセージを収集（メタデータ段階でフィルタ除外されたものは除く）
        let messages: Vec<GmailMessage> = results
            .iter()
            .filter_map(|r| r.as_ref().ok())
            .filter(|o| !o.filtered_out)
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
            filtered_out: false,
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
    use crate::gmail::client::ShopSettings;
    use crate::gmail_client::MockGmailClientTrait;
    use crate::repository::{MockEmailRepository, MockShopSettingsRepository};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn test_create_sync_input() {
        let input = create_sync_input("msg-12345".to_string());
        assert_eq!(input.message_id, "msg-12345");
    }

    #[test]
    fn test_task_name_and_event() {
        let task: GmailSyncTask<
            MockGmailClientTrait,
            MockEmailRepository,
            MockShopSettingsRepository,
        > = GmailSyncTask::new();
        assert_eq!(task.name(), "メール同期");
        assert_eq!(task.event_name(), "batch-progress");
    }

    fn dummy_shop_settings(id: i64, sender: &str) -> ShopSettings {
        ShopSettings {
            id,
            shop_name: format!("shop-{id}"),
            sender_address: sender.to_string(),
            parser_type: "dmm_confirm".to_string(),
            is_enabled: true,
            subject_filters: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn dummy_message(id: &str) -> GmailMessage {
        GmailMessage {
            message_id: id.to_string(),
            snippet: "snippet".to_string(),
            subject: Some("subject".to_string()),
            body_plain: Some("plain".to_string()),
            body_html: None,
            internal_date: 1704067200000,
            from_address: Some("sender@example.com".to_string()),
        }
    }

    #[tokio::test]
    async fn fetch_all_message_ids_paginates_and_respects_max_total() {
        let mut client = MockGmailClientTrait::new();

        client
            .expect_list_message_ids()
            .withf(|q, max, token| q == "q" && *max == 10 && token.is_none())
            .times(1)
            .returning(|_, _, _| {
                Ok((
                    vec!["a".to_string(), "b".to_string(), "c".to_string()],
                    Some("t1".to_string()),
                ))
            });

        // max_total で truncate されるため、2ページ目は呼ばれないことを期待
        let ids = fetch_all_message_ids(&client, "q", 10, Some(2))
            .await
            .unwrap();
        assert_eq!(ids, vec!["a".to_string(), "b".to_string()]);
    }

    #[tokio::test]
    async fn fetch_all_message_ids_paginates_until_next_token_none() {
        let mut client = MockGmailClientTrait::new();

        client
            .expect_list_message_ids()
            .withf(|q, max, token| q == "q" && *max == 10 && token.is_none())
            .times(1)
            .returning(|_, _, _| {
                Ok((
                    vec!["a".to_string(), "b".to_string()],
                    Some("t1".to_string()),
                ))
            });

        client
            .expect_list_message_ids()
            .withf(|q, max, token| q == "q" && *max == 10 && token.as_deref() == Some("t1"))
            .times(1)
            .returning(|_, _, _| Ok((vec!["c".to_string()], None)));

        let ids = fetch_all_message_ids(&client, "q", 10, None).await.unwrap();
        assert_eq!(ids, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[tokio::test]
    async fn before_batch_loads_shop_settings_into_cache() {
        let mut shop_repo = MockShopSettingsRepository::new();
        shop_repo
            .expect_get_enabled()
            .times(1)
            .returning(|| Ok(vec![dummy_shop_settings(1, "a@example.com")]));

        let client = MockGmailClientTrait::new();
        let email_repo = MockEmailRepository::new();

        let context = GmailSyncContext {
            gmail_client: Arc::new(client),
            email_repo: Arc::new(email_repo),
            shop_settings_repo: Arc::new(shop_repo),
            shop_settings_cache: Arc::new(Mutex::new(ShopSettingsCacheForSync::default())),
        };

        let task: GmailSyncTask<
            MockGmailClientTrait,
            MockEmailRepository,
            MockShopSettingsRepository,
        > = GmailSyncTask::new();

        task.before_batch(&[], &context).await.unwrap();

        let cache = context.shop_settings_cache.lock().await;
        assert_eq!(cache.enabled_shops.len(), 1);
        assert_eq!(cache.enabled_shops[0].sender_address, "a@example.com");
    }

    #[tokio::test]
    async fn process_batch_two_phase_fetch_filters_and_fetches_full() {
        // ok-id: メタデータフィルタ通過 → full取得成功
        // ng-id: メタデータ取得失敗 → エラー
        let mut client = MockGmailClientTrait::new();

        // Phase 1: メタデータ取得
        client
            .expect_get_message_metadata()
            .withf(|id| id == "ok-id")
            .times(1)
            .returning(|_| Ok(dummy_message("ok-id")));
        client
            .expect_get_message_metadata()
            .withf(|id| id == "ng-id")
            .times(1)
            .returning(|_| Err("boom".to_string()));

        // Phase 2: ok-id のみ full 取得
        client
            .expect_get_message()
            .withf(|id| id == "ok-id")
            .times(1)
            .returning(|_| Ok(dummy_message("ok-id")));

        let shop_repo = MockShopSettingsRepository::new();
        let email_repo = MockEmailRepository::new();
        let context = GmailSyncContext {
            gmail_client: Arc::new(client),
            email_repo: Arc::new(email_repo),
            shop_settings_repo: Arc::new(shop_repo),
            shop_settings_cache: Arc::new(Mutex::new(ShopSettingsCacheForSync {
                enabled_shops: vec![dummy_shop_settings(1, "sender@example.com")],
            })),
        };

        let task: GmailSyncTask<
            MockGmailClientTrait,
            MockEmailRepository,
            MockShopSettingsRepository,
        > = GmailSyncTask::new();

        let results = task
            .process_batch(
                vec![
                    GmailSyncInput {
                        message_id: "ok-id".to_string(),
                    },
                    GmailSyncInput {
                        message_id: "ng-id".to_string(),
                    },
                ],
                &context,
            )
            .await;

        assert_eq!(results.len(), 2);
        let ok_result = results[0].as_ref().unwrap();
        assert!(!ok_result.saved);
        assert!(!ok_result.filtered_out);
        assert!(results[1].is_err());
        assert!(results[1]
            .as_ref()
            .unwrap_err()
            .contains("Failed to fetch message ng-id"));
    }

    #[tokio::test]
    async fn process_batch_skips_full_fetch_for_filtered_out_messages() {
        // match-id: sender matches → full取得される
        // nomatch-id: sender doesn't match → filtered_out, get_message は呼ばれない
        let mut client = MockGmailClientTrait::new();

        client
            .expect_get_message_metadata()
            .withf(|id| id == "match-id")
            .times(1)
            .returning(|_| Ok(dummy_message("match-id")));
        client
            .expect_get_message_metadata()
            .withf(|id| id == "nomatch-id")
            .times(1)
            .returning(|_| {
                Ok(GmailMessage {
                    message_id: "nomatch-id".to_string(),
                    snippet: "snippet".to_string(),
                    subject: Some("subject".to_string()),
                    body_plain: None,
                    body_html: None,
                    internal_date: 1704067200000,
                    from_address: Some("unknown@other.com".to_string()),
                })
            });

        // match-id のみ full 取得される（nomatch-id は呼ばれないことを expect_get_message で保証）
        client
            .expect_get_message()
            .withf(|id| id == "match-id")
            .times(1)
            .returning(|_| Ok(dummy_message("match-id")));

        let shop_repo = MockShopSettingsRepository::new();
        let email_repo = MockEmailRepository::new();
        let context = GmailSyncContext {
            gmail_client: Arc::new(client),
            email_repo: Arc::new(email_repo),
            shop_settings_repo: Arc::new(shop_repo),
            shop_settings_cache: Arc::new(Mutex::new(ShopSettingsCacheForSync {
                enabled_shops: vec![dummy_shop_settings(1, "sender@example.com")],
            })),
        };

        let task: GmailSyncTask<
            MockGmailClientTrait,
            MockEmailRepository,
            MockShopSettingsRepository,
        > = GmailSyncTask::new();

        let results = task
            .process_batch(
                vec![
                    GmailSyncInput {
                        message_id: "match-id".to_string(),
                    },
                    GmailSyncInput {
                        message_id: "nomatch-id".to_string(),
                    },
                ],
                &context,
            )
            .await;

        assert_eq!(results.len(), 2);

        // match-id: フィルタ通過、full取得済み
        let match_result = results[0].as_ref().unwrap();
        assert!(!match_result.filtered_out);

        // nomatch-id: フィルタ除外
        let nomatch_result = results[1].as_ref().unwrap();
        assert!(nomatch_result.filtered_out);
    }

    #[tokio::test]
    async fn process_batch_overwrites_temp_metadata_when_full_fetch_fails() {
        // Phase 1: メタデータ取得は成功し、フィルタを通過する
        // Phase 2: 本文(full)取得が失敗した場合に、results[0] が Err に上書きされることを検証する
        let mut client = MockGmailClientTrait::new();

        // Phase 1: メタデータ取得成功（from_address がショップ設定に合致 → フィルタ通過）
        client
            .expect_get_message_metadata()
            .withf(|id| id == "full-fetch-fail-id")
            .times(1)
            .returning(|_| {
                Ok(GmailMessage {
                    message_id: "full-fetch-fail-id".to_string(),
                    snippet: "snippet".to_string(),
                    subject: Some("subject".to_string()),
                    body_plain: None,
                    body_html: None,
                    internal_date: 1704067200000,
                    from_address: Some("a@example.com".to_string()),
                })
            });

        // Phase 2: 本文(full)取得が失敗
        client
            .expect_get_message()
            .withf(|id| id == "full-fetch-fail-id")
            .times(1)
            .returning(|_| Err("full fetch failed".to_string()));
        let email_repo = MockEmailRepository::new();
        let shop_repo = MockShopSettingsRepository::new();

        let context = GmailSyncContext {
            gmail_client: Arc::new(client),
            email_repo: Arc::new(email_repo),
            shop_settings_repo: Arc::new(shop_repo),
            shop_settings_cache: Arc::new(Mutex::new(ShopSettingsCacheForSync {
                // 既存テストと同様に有効なショップ設定を入れておき、
                // 対象メッセージがフィルタ除外されないようにする
                enabled_shops: vec![dummy_shop_settings(1, "a@example.com")],
            })),
        };

        let task: GmailSyncTask<
            MockGmailClientTrait,
            MockEmailRepository,
            MockShopSettingsRepository,
        > = GmailSyncTask::new();

        let inputs = vec![GmailSyncInput {
            message_id: "full-fetch-fail-id".to_string(),
        }];

        let results = task.process_batch(inputs, &context).await;

        assert_eq!(results.len(), 1);
        // Phase 2 の full 取得失敗により、一時的に格納されていたメタデータではなく
        // エラーが最終結果として格納されていることを確認する
        assert!(results[0].is_err());
    }

    #[tokio::test]
    async fn after_batch_returns_ok_when_no_messages_to_save() {
        let client = MockGmailClientTrait::new();
        let email_repo = MockEmailRepository::new();
        let shop_repo = MockShopSettingsRepository::new();

        let context = GmailSyncContext {
            gmail_client: Arc::new(client),
            email_repo: Arc::new(email_repo),
            shop_settings_repo: Arc::new(shop_repo),
            shop_settings_cache: Arc::new(Mutex::new(ShopSettingsCacheForSync {
                enabled_shops: vec![dummy_shop_settings(1, "a@example.com")],
            })),
        };

        let task: GmailSyncTask<
            MockGmailClientTrait,
            MockEmailRepository,
            MockShopSettingsRepository,
        > = GmailSyncTask::new();

        let results: Vec<Result<GmailSyncOutput, String>> = vec![Err("x".to_string())];
        task.after_batch(1, &results, &context).await.unwrap();
    }
}
