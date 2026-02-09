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
use crate::parsers::{get_parser, is_cancel_parser, parse_cancel_with_parser, EmailRow, OrderInfo, ParseState};
use crate::repository::{OrderRepository, ParseRepository, ShopSettingsRepository};
use async_trait::async_trait;
use chrono::DateTime;
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
pub struct EmailParseContext<O, P, S>
where
    O: OrderRepository + 'static,
    P: ParseRepository + 'static,
    S: ShopSettingsRepository + 'static,
{
    /// Order リポジトリ
    pub order_repo: Arc<O>,
    /// Parse リポジトリ
    pub parse_repo: Arc<P>,
    /// ShopSettings リポジトリ
    pub shop_settings_repo: Arc<S>,
    /// ショップ設定キャッシュ
    pub shop_settings_cache: Arc<Mutex<ShopSettingsCache>>,
    /// パース状態（キャンセル用）
    pub parse_state: Arc<ParseState>,
}

/// メールパースタスク
///
/// 型パラメータ:
/// - `O`: Order リポジトリ
/// - `P`: Parse リポジトリ
/// - `S`: ShopSettings リポジトリ
pub struct EmailParseTask<O, P, S>
where
    O: OrderRepository + 'static,
    P: ParseRepository + 'static,
    S: ShopSettingsRepository + 'static,
{
    _phantom: PhantomData<(O, P, S)>,
}

/// タスク名
pub const EMAIL_PARSE_TASK_NAME: &str = "メールパース";
/// イベント名
pub const EMAIL_PARSE_EVENT_NAME: &str = "batch-progress";

impl<O, P, S> EmailParseTask<O, P, S>
where
    O: OrderRepository + 'static,
    P: ParseRepository + 'static,
    S: ShopSettingsRepository + 'static,
{
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<O, P, S> Default for EmailParseTask<O, P, S>
where
    O: OrderRepository + 'static,
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

#[async_trait]
impl<O, P, S> BatchTask for EmailParseTask<O, P, S>
where
    O: OrderRepository + 'static,
    P: ParseRepository + 'static,
    S: ShopSettingsRepository + 'static,
{
    type Input = EmailParseInput;
    type Output = EmailParseOutput;
    type Context = EmailParseContext<O, P, S>;

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

    /// メールをパース
    async fn process_batch(
        &self,
        inputs: Vec<Self::Input>,
        context: &Self::Context,
    ) -> Vec<Result<Self::Output, String>> {
        let mut results: Vec<Result<Self::Output, String>> = Vec::with_capacity(inputs.len());
        let cache = context.shop_settings_cache.lock().await;
        let settings = &cache.settings;

        for input in inputs {
            // 候補パーサーを取得
            let candidate_parsers = get_candidate_parsers(
                settings,
                input.from_address.as_deref(),
                input.subject.as_deref(),
            );

            if candidate_parsers.is_empty() {
                results.push(Err(format!(
                    "No matching parser for email {} (from: {:?})",
                    input.email_id, input.from_address
                )));
                continue;
            }

            // 複数のパーサーを順番に試す
            let mut parse_result: Option<(OrderInfo, String, String)> = None; // (order_info, shop_name, parser_type)
            let mut last_error = String::new();
            let mut cancel_applied = false;

            for (parser_type, shop_name) in &candidate_parsers {
                // キャンセルメールは専用パーサーで処理（OrderInfo を返さない）
                if is_cancel_parser(parser_type) {
                    log::debug!(
                        "[cancel] trying {} email_id={} subject={:?}",
                        parser_type,
                        input.email_id,
                        input.subject
                    );
                    match parse_cancel_with_parser(parser_type, &input.body_plain) {
                        Ok(cancel_info) => {
                            log::debug!(
                                "[cancel] email_id={} internal_date={:?} order_number={} subject={:?}",
                                input.email_id,
                                input.internal_date,
                                cancel_info.order_number,
                                input.subject
                            );
                            let from_address = input.from_address.as_deref().unwrap_or("");
                            let shop_domain = extract_email_address(from_address)
                                .and_then(|email| extract_domain(&email).map(|s| s.to_string()));

                            match context
                                .order_repo
                                .apply_cancel(
                                    &cancel_info,
                                    input.email_id,
                                    shop_domain.clone(),
                                    Some(shop_name.clone()),
                                )
                                .await
                            {
                                Ok(order_id) => {
                                    log::debug!(
                                        "Successfully applied cancel for order {} (email {})",
                                        order_id,
                                        input.email_id
                                    );
                                    results.push(Ok(EmailParseOutput {
                                        email_id: input.email_id,
                                        order_info: OrderInfo {
                                            order_number: cancel_info.order_number,
                                            order_date: None,
                                            delivery_address: None,
                                            delivery_info: None,
                                            items: vec![],
                                            subtotal: None,
                                            shipping_fee: None,
                                            total_amount: None,
                                        },
                                        shop_name: shop_name.clone(),
                                        shop_domain,
                                        cancel_applied: true,
                                    }));
                                    cancel_applied = true;
                                }
                                Err(e) => {
                                    // 注文未存在（キャンセルが confirm/change より先に来た等）の場合。
                                    // Err を返すと該当メールは未パースのまま残り、次回 run で再試行される。
                                    log::info!(
                                        "[cancel] apply_cancel failed email_id={} order_number={}: {}",
                                        input.email_id,
                                        cancel_info.order_number,
                                        e
                                    );
                                    last_error = e;
                                    break;
                                }
                            }
                            break;
                        }
                        Err(e) => {
                            log::debug!("{} parser failed: {}", parser_type, e);
                            last_error = e;
                            continue;
                        }
                    }
                }

                let parser = match get_parser(parser_type) {
                    Some(p) => p,
                    None => {
                        log::warn!("Unknown parser type: {}", parser_type);
                        continue;
                    }
                };

                match parser.parse(&input.body_plain) {
                    Ok(mut order_info) => {
                        log::debug!("Successfully parsed with parser: {}", parser_type);

                        // confirm, confirm_yoyaku, change の場合はメール受信日を order_date に使用
                        if order_info.order_date.is_none()
                            && input.internal_date.is_some()
                            && matches!(
                                parser_type.as_str(),
                                "hobbysearch_confirm"
                                    | "hobbysearch_confirm_yoyaku"
                                    | "hobbysearch_change"
                                    | "hobbysearch_change_yoyaku"
                            )
                        {
                            if let Some(ts_ms) = input.internal_date {
                                let dt = match DateTime::from_timestamp_millis(ts_ms) {
                                    Some(d) => d,
                                    None => {
                                        log::warn!(
                                            "Failed to parse internal_date {} for email {}",
                                            ts_ms,
                                            input.email_id
                                        );
                                        chrono::Utc::now()
                                    }
                                };
                                order_info.order_date =
                                    Some(dt.format("%Y-%m-%d %H:%M:%S").to_string());
                            }
                        }

                        parse_result = Some((order_info, shop_name.clone(), parser_type.clone()));
                        break;
                    }
                    Err(e) => {
                        log::debug!("Parser {} failed: {}", parser_type, e);
                        last_error = e;
                        continue;
                    }
                }
            }

            if cancel_applied {
                // 結果は既に push 済み
            } else if let Some((order_info, shop_name, parser_type)) = parse_result {
                // ドメインを抽出
                let from_address = input.from_address.as_deref().unwrap_or("");
                let shop_domain = extract_email_address(from_address)
                    .and_then(|email| extract_domain(&email).map(|s| s.to_string()));

                // 組み換えメールの場合、同一トランザクションで元注文削除＋新注文登録（データ欠損を防ぐ）
                // internal_date が無効値の場合は安全のため組み換え処理をスキップし、単純な save_order にフォールバックする
                let change_email_internal_date = input
                    .internal_date
                    .and_then(|ts| DateTime::from_timestamp_millis(ts).map(|_| ts));
                let save_result = if parser_type == "hobbysearch_change"
                    || parser_type == "hobbysearch_change_yoyaku"
                {
                    if let Some(change_email_internal_date) = change_email_internal_date {
                        context
                            .order_repo
                            .apply_change_items_and_save_order(
                                &order_info,
                                Some(input.email_id),
                                shop_domain.clone(),
                                Some(shop_name.clone()),
                                Some(change_email_internal_date),
                            )
                            .await
                    } else {
                        log::warn!(
                            "Invalid internal_date for change email {}, fallback to save_order without applying change items",
                            input.email_id
                        );
                        context
                            .order_repo
                            .save_order(
                                &order_info,
                                Some(input.email_id),
                                shop_domain.clone(),
                                Some(shop_name.clone()),
                            )
                            .await
                    }
                } else {
                    context
                        .order_repo
                        .save_order(
                            &order_info,
                            Some(input.email_id),
                            shop_domain.clone(),
                            Some(shop_name.clone()),
                        )
                        .await
                };

                // キャンセルメールが同一バッチ内の後続で apply_cancel するため、
                // 確認・変更メールはここで即時 save_order する（after_batch に遅延すると注文が未コミットで見つからない）
                match save_result {
                    Ok(order_id) => {
                        log::debug!(
                            "Saved order {} for email {} (in-batch)",
                            order_id,
                            input.email_id
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to save order for email {}: {}", input.email_id, e);
                        results.push(Err(format!("Failed to save order: {e}")));
                        continue;
                    }
                }

                results.push(Ok(EmailParseOutput {
                    email_id: input.email_id,
                    order_info,
                    shop_name,
                    shop_domain,
                    cancel_applied: false,
                }));
            } else {
                results.push(Err(format!(
                    "All parsers failed for email {}: {}",
                    input.email_id, last_error
                )));
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
