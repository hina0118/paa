//! メールパース用 BatchTask 実装
//!
//! `BatchRunner` を使用してメールパースを実行するための `BatchTask` 実装。
//!
//! # 設計
//! 現在のメールパース処理は複雑なフローを持つため、BatchTaskトレイトに完全に準拠させるのは
//! 段階的に行います。このモジュールでは以下を提供します：
//!
//! - `EmailParseInput`: パース対象メールの入力データ
//! - `EmailParseOutput`: パース結果（注文情報）
//! - `EmailParseContext`: パースに必要なコンテキスト
//! - `EmailParseTask`: BatchTaskトレイト実装
//!
//! # フック活用
//! - `before_batch`: shop_settings の取得（バッチごとにキャッシュ）
//! - `process_batch`: メールの正規表現パース
//! - `after_batch`: パース結果のDB保存

use crate::batch_runner::BatchTask;
use crate::logic::email_parser::extract_domain;
use crate::logic::sync_logic::extract_email_address;
use crate::parsers::{EmailRow, OrderInfo, ParseState};
use crate::plugins::{
    build_registry, find_plugin, save_images_for_order, DispatchError, DispatchOutcome,
};
use crate::repository::{ParseRepository, ShopSettingsRepository};
use async_trait::async_trait;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::Mutex;

/// メールパースタスクの入力
#[derive(Debug, Clone)]
pub struct EmailParseInput {
    /// メールID
    pub email_id: i64,
    /// メッセージID
    pub message_id: String,
    /// メール本文（プレーンテキスト）
    pub body_plain: String,
    /// 送信元アドレス
    pub from_address: Option<String>,
    /// 件名
    pub subject: Option<String>,
    /// 内部日付（UNIXタイムスタンプミリ秒）
    pub internal_date: Option<i64>,
}

impl From<EmailRow> for EmailParseInput {
    fn from(row: EmailRow) -> Self {
        let body = crate::parsers::get_body_for_parse(&row);
        Self {
            email_id: row.email_id,
            message_id: row.message_id,
            body_plain: body,
            from_address: row.from_address,
            subject: row.subject,
            internal_date: row.internal_date,
        }
    }
}

/// メールパースタスクの出力
#[derive(Debug, Clone)]
pub struct EmailParseOutput {
    /// メールID
    pub email_id: i64,
    /// パースされた注文情報（キャンセル適用時はダミー）
    pub order_info: OrderInfo,
    /// ショップ名
    pub shop_name: String,
    /// ショップドメイン
    pub shop_domain: Option<String>,
    /// キャンセルメールを適用済み（apply_cancel 済みのため save_order 不要）
    pub cancel_applied: bool,
}

/// パース失敗の理由
#[derive(Debug, Clone)]
pub struct ParseFailure {
    /// メールID
    pub email_id: i64,
    /// 失敗理由
    pub reason: String,
}

/// ショップ設定のキャッシュ
#[derive(Debug, Clone, Default)]
pub struct ShopSettingsCache {
    /// (sender_address, parser_type, subject_filters, shop_name) のリスト
    pub settings: Vec<(String, String, Option<String>, String)>,
}

/// メールパースのコンテキスト
pub struct EmailParseContext<P, S>
where
    P: ParseRepository + 'static,
    S: ShopSettingsRepository + 'static,
{
    /// SQLite 接続プール（dispatch() 用トランザクション生成に使用）
    pub pool: Arc<sqlx::SqlitePool>,
    /// Parse リポジトリ
    pub parse_repo: Arc<P>,
    /// ShopSettings リポジトリ
    pub shop_settings_repo: Arc<S>,
    /// ショップ設定キャッシュ
    pub shop_settings_cache: Arc<Mutex<ShopSettingsCache>>,
    /// パース状態（キャンセル用）
    pub parse_state: Arc<ParseState>,
    /// 画像保存用: (pool, images_dir)。None の場合は画像登録をスキップ
    pub image_save_ctx: Option<(std::sync::Arc<sqlx::SqlitePool>, std::path::PathBuf)>,
}

/// メールパースタスク
///
/// 型パラメータ:
/// - `P`: Parse リポジトリ
/// - `S`: ShopSettings リポジトリ
pub struct EmailParseTask<P, S>
where
    P: ParseRepository + 'static,
    S: ShopSettingsRepository + 'static,
{
    _phantom: PhantomData<(P, S)>,
}

/// タスク名
pub const EMAIL_PARSE_TASK_NAME: &str = "メールパース";
/// イベント名
pub const EMAIL_PARSE_EVENT_NAME: &str = "batch-progress";
/// パーサー非マッチエラーの固定プレフィックス（batch_runner でスキップ判定に使用）
pub const NO_MATCHING_PARSER_PREFIX: &str = "No matching parser";

impl<P, S> EmailParseTask<P, S>
where
    P: ParseRepository + 'static,
    S: ShopSettingsRepository + 'static,
{
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<P, S> Default for EmailParseTask<P, S>
where
    P: ParseRepository + 'static,
    S: ShopSettingsRepository + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// ショップ設定から候補パーサーを取得
///
/// 旧実装 (`logic/email_parser.rs`) と同じロジックを使用:
/// - from_address からメールアドレスを抽出して正規化
/// - sender_address と完全一致（大文字小文字無視）でチェック
fn get_candidate_parsers(
    settings: &[(String, String, Option<String>, String)],
    from_address: Option<&str>,
    subject: Option<&str>,
) -> Vec<(String, String)> {
    // from_addressからメールアドレスを抽出して正規化
    let normalized_from = match from_address {
        Some(addr) => match extract_email_address(addr) {
            Some(email) => email,
            None => return vec![], // 有効なメールアドレスが抽出できない場合は空を返す
        },
        None => return vec![],
    };

    settings
        .iter()
        .filter(|(sender_address, _, subject_filters, _)| {
            // 送信元アドレスが完全一致するか確認（大文字小文字無視）
            if !sender_address.eq_ignore_ascii_case(&normalized_from) {
                return false;
            }

            // 件名フィルターがある場合はチェック
            if let Some(filters) = subject_filters {
                // JSON形式のフィルターをパース
                let filter_list: Vec<String> = match serde_json::from_str(filters) {
                    Ok(list) => list,
                    Err(_) => {
                        // JSONパースエラー時はフィルター無視（旧実装と同じ）
                        return true;
                    }
                };

                // 空のフィルターリストは「フィルターなし＝全許可」
                if filter_list.is_empty() {
                    return true;
                }

                // 件名がない場合は除外
                let subj = match subject {
                    Some(s) => s,
                    None => return false,
                };

                // いずれかのフィルターに一致すればOK
                if !filter_list.iter().any(|filter| subj.contains(filter)) {
                    return false;
                }
            }

            true
        })
        .map(|(_, parser_type, _, shop_name)| (parser_type.clone(), shop_name.clone()))
        .collect()
}

/// `DispatchOutcome` に含まれるすべての `OrderInfo` に対して画像保存を実行する
///
/// `tx.commit()` 後に呼び出すことで、トランザクションの RESERVED LOCK と
/// 画像 INSERT の競合（SQLITE_BUSY / "database is locked"）を回避する。
///
/// - `OrderSaved` → 1件分の画像を登録
/// - `MultiOrderSaved` → 全注文の画像を登録（先頭だけでなく全件）
/// - `CancelApplied` / `OrderNumberChanged` / `ConsolidationApplied` → 画像なしのためスキップ
async fn save_images_for_dispatch_outcome(
    outcome: &DispatchOutcome,
    image_save_ctx: &Option<(std::sync::Arc<sqlx::SqlitePool>, std::path::PathBuf)>,
) {
    match outcome {
        DispatchOutcome::OrderSaved(order_info) => {
            save_images_for_order(order_info, image_save_ctx).await;
        }
        DispatchOutcome::MultiOrderSaved(orders) => {
            for order_info in orders {
                save_images_for_order(order_info, image_save_ctx).await;
            }
        }
        _ => {}
    }
}

/// `DispatchOutcome` を `EmailParseOutput` 組み立てに必要な `(OrderInfo, cancel_applied)` に変換する
///
/// - `OrderSaved` / `MultiOrderSaved` → cancel_applied = false（通常保存）
/// - `CancelApplied` / `OrderNumberChanged` / `ConsolidationApplied` → cancel_applied = true（特殊適用済み）
fn outcome_to_order_info(outcome: DispatchOutcome, email_id: i64) -> (OrderInfo, bool) {
    match outcome {
        DispatchOutcome::OrderSaved(order_info) => (*order_info, false),
        DispatchOutcome::CancelApplied { order_number } => {
            let info = OrderInfo {
                order_number,
                order_date: None,
                delivery_address: None,
                delivery_info: None,
                items: vec![],
                subtotal: None,
                shipping_fee: None,
                total_amount: None,
            };
            (info, true)
        }
        DispatchOutcome::OrderNumberChanged { new_order_number } => {
            let info = OrderInfo {
                order_number: new_order_number,
                order_date: None,
                delivery_address: None,
                delivery_info: None,
                items: vec![],
                subtotal: None,
                shipping_fee: None,
                total_amount: None,
            };
            (info, true)
        }
        DispatchOutcome::ConsolidationApplied { new_order_number } => {
            let info = OrderInfo {
                order_number: new_order_number,
                order_date: None,
                delivery_address: None,
                delivery_info: None,
                items: vec![],
                subtotal: None,
                shipping_fee: None,
                total_amount: None,
            };
            (info, true)
        }
        DispatchOutcome::MultiOrderSaved(orders) => {
            let first = orders.into_iter().next().unwrap_or_else(|| {
                log::warn!(
                    "[email_parse_task] MultiOrderSaved with empty orders (email_id={})",
                    email_id
                );
                OrderInfo {
                    order_number: String::new(),
                    order_date: None,
                    delivery_address: None,
                    delivery_info: None,
                    items: vec![],
                    subtotal: None,
                    shipping_fee: None,
                    total_amount: None,
                }
            });
            (first, false)
        }
    }
}

#[async_trait]
impl<P, S> BatchTask for EmailParseTask<P, S>
where
    P: ParseRepository + 'static,
    S: ShopSettingsRepository + 'static,
{
    type Input = EmailParseInput;
    type Output = EmailParseOutput;
    type Context = EmailParseContext<P, S>;

    fn name(&self) -> &str {
        EMAIL_PARSE_TASK_NAME
    }

    fn event_name(&self) -> &str {
        EMAIL_PARSE_EVENT_NAME
    }

    /// バッチ処理前にショップ設定を取得してキャッシュ
    async fn before_batch(
        &self,
        _inputs: &[Self::Input],
        context: &Self::Context,
    ) -> Result<(), String> {
        log::debug!("[{}] before_batch: Loading shop settings", self.name());

        // ショップ設定を取得
        let enabled_settings = context
            .shop_settings_repo
            .get_enabled()
            .await
            .map_err(|e| format!("Failed to fetch shop settings: {e}"))?;

        if enabled_settings.is_empty() {
            return Err("No enabled shop settings found".to_string());
        }

        let settings: Vec<(String, String, Option<String>, String)> = enabled_settings
            .into_iter()
            .map(|s| {
                (
                    s.sender_address,
                    s.parser_type,
                    s.subject_filters,
                    s.shop_name,
                )
            })
            .collect();

        // キャッシュに保存
        let mut cache = context.shop_settings_cache.lock().await;
        cache.settings = settings;

        log::info!(
            "[{}] Shop settings loaded: {} entries",
            self.name(),
            cache.settings.len()
        );

        Ok(())
    }

    /// メールをパース（VendorPlugin レジストリ経由）
    async fn process_batch(
        &self,
        inputs: Vec<Self::Input>,
        context: &Self::Context,
    ) -> Vec<Result<Self::Output, String>> {
        let mut results: Vec<Result<Self::Output, String>> = Vec::with_capacity(inputs.len());
        let cache = context.shop_settings_cache.lock().await;
        let settings = &cache.settings;
        let registry = build_registry();

        'input_loop: for input in inputs {
            // 候補パーサーを取得
            let candidate_parsers = get_candidate_parsers(
                settings,
                input.from_address.as_deref(),
                input.subject.as_deref(),
            );

            if candidate_parsers.is_empty() {
                log::debug!(
                    "No parser found for address: {:?} with subject: {:?}",
                    input.from_address.as_deref().unwrap_or("(null)"),
                    input.subject.as_deref(),
                );
                results.push(Err(format!(
                    "{} for email {} (from: {:?})",
                    NO_MATCHING_PARSER_PREFIX, input.email_id, input.from_address
                )));
                continue;
            }

            let mut last_error = String::new();
            let mut dispatch_outcome: Option<(DispatchOutcome, String)> = None; // (outcome, shop_name)

            'parser_loop: for (parser_type, shop_name) in &candidate_parsers {
                let plugin = match find_plugin(&registry, parser_type) {
                    Some(p) => p,
                    None => {
                        log::warn!(
                            "No plugin for parser_type: {} (email_id={})",
                            parser_type,
                            input.email_id
                        );
                        last_error = format!("No plugin for parser_type: {}", parser_type);
                        continue 'parser_loop;
                    }
                };

                // パーサー試行ごとにトランザクションを開始。
                // ParseFailed 時は tx を drop してロールバック（通常は DB 未書き込みだが安全のため）。
                // SaveFailed 時も tx を drop してロールバック（部分書き込みを破棄）。
                let mut tx = match context.pool.begin().await {
                    Ok(tx) => tx,
                    Err(e) => {
                        results.push(Err(format!(
                            "Failed to begin transaction for email {}: {}",
                            input.email_id, e
                        )));
                        continue 'input_loop;
                    }
                };

                match plugin
                    .dispatch(
                        parser_type,
                        input.email_id,
                        input.from_address.as_deref(),
                        shop_name,
                        input.internal_date,
                        &input.body_plain,
                        &mut tx,
                    )
                    .await
                {
                    Ok(outcome) => {
                        // コミット。失敗時は保存エラーとして扱いリトライ対象にする。
                        if let Err(e) = tx.commit().await {
                            results.push(Err(format!(
                                "Failed to commit transaction for email {}: {}",
                                input.email_id, e
                            )));
                            continue 'input_loop;
                        }
                        log::debug!(
                            "dispatch succeeded: parser_type={} email_id={}",
                            parser_type,
                            input.email_id
                        );
                        dispatch_outcome = Some((outcome, shop_name.clone()));
                        break 'parser_loop;
                    }
                    Err(DispatchError::ParseFailed(e)) => {
                        // パース失敗 → tx を drop（自動ロールバック）して次のパーサーを試す
                        log::debug!(
                            "Parser {} failed (email_id={}): {}",
                            parser_type,
                            input.email_id,
                            e
                        );
                        last_error = e;
                        continue 'parser_loop;
                    }
                    Err(DispatchError::SaveFailed(e)) => {
                        // 保存 / 適用失敗 → tx を drop（自動ロールバック）してリトライ対象にする
                        log::error!(
                            "Save/apply failed for email {} (parser_type={}): {}",
                            input.email_id,
                            parser_type,
                            e
                        );
                        results.push(Err(format!(
                            "Save failed for email {}: {}",
                            input.email_id, e
                        )));
                        continue 'input_loop;
                    }
                }
            }

            // dispatch_outcome が None の場合は全パーサーが ParseFailed
            let from_address = input.from_address.as_deref().unwrap_or("");
            let shop_domain = extract_email_address(from_address)
                .and_then(|email| extract_domain(&email).map(|s| s.to_string()));

            match dispatch_outcome {
                Some((outcome, shop_name)) => {
                    // tx.commit() 後に画像登録を実行する。
                    // dispatch() 内ではトランザクションの RESERVED LOCK が保持されており、
                    // 別コネクションからの INSERT が SQLITE_BUSY になるため、
                    // コミット完了後のここで行う必要がある。
                    save_images_for_dispatch_outcome(&outcome, &context.image_save_ctx).await;

                    let (order_info, cancel_applied) =
                        outcome_to_order_info(outcome, input.email_id);
                    results.push(Ok(EmailParseOutput {
                        email_id: input.email_id,
                        order_info,
                        shop_name,
                        shop_domain,
                        cancel_applied,
                    }));
                }
                None => {
                    log::error!(
                        "All parsers failed for email {}. Last error: {}",
                        input.email_id,
                        last_error
                    );
                    results.push(Err(format!(
                        "All parsers failed for email {}: {}",
                        input.email_id, last_error
                    )));
                }
            }
        }

        results
    }

    /// パース結果をDBに保存
    async fn after_batch(
        &self,
        batch_number: usize,
        results: &[Result<Self::Output, String>],
        _context: &Self::Context,
    ) -> Result<(), String> {
        log::debug!(
            "[{}] after_batch: batch {} with {} results",
            self.name(),
            batch_number,
            results.len()
        );

        let mut saved_count = 0;

        for result in results {
            match result {
                Ok(_output) => {
                    // 確認・変更は process_batch で即時 save_order 済み。
                    // キャンセルは apply_cancel 済み。いずれも after_batch での保存は不要。
                    saved_count += 1;
                }
                Err(e) => {
                    // パース失敗: BatchRunnerがすでに失敗をカウントしているのでここではログのみ
                    log::debug!("[{}] Parse failed: {}", self.name(), e);
                }
            }
        }

        // 成功件数と失敗件数をログ
        let success = results.iter().filter(|r| r.is_ok()).count();
        let failed = results.iter().filter(|r| r.is_err()).count();
        log::info!(
            "[{}] Batch {} complete: {} parsed, {} failed, {} saved",
            self.name(),
            batch_number,
            success,
            failed,
            saved_count
        );

        Ok(())
    }

    /// 単一アイテムの処理（process_batch がオーバーライドされているため通常は呼ばれない）
    async fn process(
        &self,
        input: Self::Input,
        context: &Self::Context,
    ) -> Result<Self::Output, String> {
        // process_batch を1件で呼び出す
        let results = self.process_batch(vec![input], context).await;
        results
            .into_iter()
            .next()
            .unwrap_or(Err("No result".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gmail::ShopSettings;
    use crate::repository::{MockParseRepository, MockShopSettingsRepository};
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    async fn setup_test_pool() -> sqlx::SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test pool")
    }

    #[test]
    fn test_get_candidate_parsers_no_match() {
        let settings = vec![(
            "shop@example.com".to_string(),
            "hobbysearch_confirm".to_string(),
            None,
            "TestShop".to_string(),
        )];

        let result = get_candidate_parsers(&settings, Some("other@test.com"), None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_candidate_parsers_match() {
        let settings = vec![(
            "shop@example.com".to_string(),
            "hobbysearch_confirm".to_string(),
            None,
            "TestShop".to_string(),
        )];

        let result = get_candidate_parsers(&settings, Some("shop@example.com"), None);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0],
            ("hobbysearch_confirm".to_string(), "TestShop".to_string())
        );
    }

    #[test]
    fn test_get_candidate_parsers_with_subject_filter() {
        let settings = vec![(
            "shop@example.com".to_string(),
            "hobbysearch_confirm".to_string(),
            Some(r#"["注文確認","発送"]"#.to_string()), // JSON形式
            "TestShop".to_string(),
        )];

        // 件名が一致
        let result = get_candidate_parsers(
            &settings,
            Some("shop@example.com"),
            Some("【注文確認】ありがとうございます"),
        );
        assert_eq!(result.len(), 1);

        // 件名が不一致
        let result = get_candidate_parsers(
            &settings,
            Some("shop@example.com"),
            Some("キャンセルのお知らせ"),
        );
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_candidate_parsers_from_address_none_returns_empty() {
        let settings = vec![(
            "shop@example.com".to_string(),
            "hobbysearch_confirm".to_string(),
            None,
            "TestShop".to_string(),
        )];
        let result = get_candidate_parsers(&settings, None, Some("x"));
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_candidate_parsers_invalid_email_returns_empty() {
        let settings = vec![(
            "shop@example.com".to_string(),
            "hobbysearch_confirm".to_string(),
            None,
            "TestShop".to_string(),
        )];
        let result = get_candidate_parsers(&settings, Some("not-an-email"), None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_candidate_parsers_sender_case_insensitive() {
        let settings = vec![(
            "Shop@Example.com".to_string(),
            "hobbysearch_confirm".to_string(),
            None,
            "TestShop".to_string(),
        )];
        let result = get_candidate_parsers(&settings, Some("shop@example.com"), None);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_get_candidate_parsers_subject_filter_invalid_json_is_ignored() {
        let settings = vec![(
            "shop@example.com".to_string(),
            "hobbysearch_confirm".to_string(),
            Some("not json".to_string()),
            "TestShop".to_string(),
        )];

        // JSON パースエラー時はフィルター無視（旧実装互換）→ sender が合えば通す
        let result = get_candidate_parsers(&settings, Some("shop@example.com"), None);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_get_candidate_parsers_subject_filter_empty_list_allows_all() {
        let settings = vec![(
            "shop@example.com".to_string(),
            "hobbysearch_confirm".to_string(),
            Some("[]".to_string()),
            "TestShop".to_string(),
        )];

        let result = get_candidate_parsers(&settings, Some("shop@example.com"), None);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_get_candidate_parsers_subject_filter_requires_subject_when_non_empty() {
        let settings = vec![(
            "shop@example.com".to_string(),
            "hobbysearch_confirm".to_string(),
            Some(r#"["注文確認"]"#.to_string()),
            "TestShop".to_string(),
        )];

        // 件名が無い場合は除外
        let result = get_candidate_parsers(&settings, Some("shop@example.com"), None);
        assert!(result.is_empty());
    }

    fn dummy_shop_settings(
        sender_address: &str,
        parser_type: &str,
        subject_filters: Option<String>,
        shop_name: &str,
    ) -> ShopSettings {
        ShopSettings {
            id: 1,
            shop_name: shop_name.to_string(),
            sender_address: sender_address.to_string(),
            parser_type: parser_type.to_string(),
            is_enabled: true,
            subject_filters,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn before_batch_returns_error_when_no_enabled_settings() {
        let mut shop_repo = MockShopSettingsRepository::new();
        shop_repo
            .expect_get_enabled()
            .times(1)
            .returning(|| Ok(vec![]));

        let context = EmailParseContext {
            pool: Arc::new(setup_test_pool().await),
            parse_repo: Arc::new(MockParseRepository::new()),
            shop_settings_repo: Arc::new(shop_repo),
            shop_settings_cache: Arc::new(Mutex::new(ShopSettingsCache::default())),
            parse_state: Arc::new(ParseState::new()),
            image_save_ctx: None,
        };

        let task: EmailParseTask<MockParseRepository, MockShopSettingsRepository> =
            EmailParseTask::new();

        let err = task.before_batch(&[], &context).await.unwrap_err();
        assert_eq!(err, "No enabled shop settings found");
    }

    #[tokio::test]
    async fn before_batch_caches_settings_fields() {
        let mut shop_repo = MockShopSettingsRepository::new();
        shop_repo.expect_get_enabled().times(1).returning(|| {
            Ok(vec![
                dummy_shop_settings(
                    "shop@example.com",
                    "hobbysearch_confirm",
                    Some(r#"["注文確認"]"#.to_string()),
                    "TestShop",
                ),
                dummy_shop_settings("other@example.com", "dmm_confirm", None, "OtherShop"),
            ])
        });

        let context = EmailParseContext {
            pool: Arc::new(setup_test_pool().await),
            parse_repo: Arc::new(MockParseRepository::new()),
            shop_settings_repo: Arc::new(shop_repo),
            shop_settings_cache: Arc::new(Mutex::new(ShopSettingsCache::default())),
            parse_state: Arc::new(ParseState::new()),
            image_save_ctx: None,
        };

        let task: EmailParseTask<MockParseRepository, MockShopSettingsRepository> =
            EmailParseTask::new();

        task.before_batch(&[], &context).await.unwrap();

        let cache = context.shop_settings_cache.lock().await;
        assert_eq!(cache.settings.len(), 2);
        assert_eq!(cache.settings[0].0, "shop@example.com");
        assert_eq!(cache.settings[0].1, "hobbysearch_confirm");
        assert_eq!(cache.settings[0].3, "TestShop");
    }

    #[tokio::test]
    async fn process_batch_returns_no_matching_parser_error_when_from_address_missing() {
        let context = EmailParseContext {
            pool: Arc::new(setup_test_pool().await),
            parse_repo: Arc::new(MockParseRepository::new()),
            shop_settings_repo: Arc::new(MockShopSettingsRepository::new()),
            shop_settings_cache: Arc::new(Mutex::new(ShopSettingsCache {
                settings: vec![(
                    "shop@example.com".to_string(),
                    "hobbysearch_confirm".to_string(),
                    None,
                    "TestShop".to_string(),
                )],
            })),
            parse_state: Arc::new(ParseState::new()),
            image_save_ctx: None,
        };

        let task: EmailParseTask<MockParseRepository, MockShopSettingsRepository> =
            EmailParseTask::new();

        let results = task
            .process_batch(
                vec![EmailParseInput {
                    email_id: 1,
                    message_id: "m".to_string(),
                    body_plain: "body".to_string(),
                    from_address: None,
                    subject: Some("x".to_string()),
                    internal_date: None,
                }],
                &context,
            )
            .await;

        assert_eq!(results.len(), 1);
        let err = results[0].as_ref().unwrap_err();
        assert!(err.starts_with(NO_MATCHING_PARSER_PREFIX));
        assert!(err.contains("email 1"));
    }

    #[test]
    fn test_email_parse_input_from_email_row() {
        let row = EmailRow {
            email_id: 123,
            message_id: "msg-001".to_string(),
            body_plain: Some("Hello".to_string()),
            body_html: None,
            from_address: Some("test@example.com".to_string()),
            subject: Some("Test Subject".to_string()),
            internal_date: Some(1700000000000),
        };

        let input: EmailParseInput = row.into();
        assert_eq!(input.email_id, 123);
        assert_eq!(input.message_id, "msg-001");
        assert_eq!(input.body_plain, "Hello");
        assert_eq!(input.from_address, Some("test@example.com".to_string()));
        assert_eq!(input.subject, Some("Test Subject".to_string()));
        assert_eq!(input.internal_date, Some(1700000000000));
    }
}
