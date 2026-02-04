use crate::batch_runner::BatchProgressEvent;
use crate::logic::email_parser::extract_domain;
use crate::logic::sync_logic::extract_email_address;
use crate::repository::{
    OrderRepository, ParseMetadataRepository, ParseRepository, ShopSettingsRepository,
    SqliteOrderRepository, SqliteParseMetadataRepository, SqliteParseRepository,
    SqliteShopSettingsRepository,
};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

// 定数はemail_parse_taskモジュールからエクスポート
use email_parse_task::{EMAIL_PARSE_EVENT_NAME, EMAIL_PARSE_TASK_NAME};

/// パース対象メールの情報（get_unparsed_emails の戻り値）
#[derive(Debug, Clone, FromRow)]
pub struct EmailRow {
    #[sqlx(rename = "id")]
    pub email_id: i64,
    pub message_id: String,
    pub body_plain: String,
    pub from_address: Option<String>,
    pub subject: Option<String>,
    pub internal_date: Option<i64>,
}

// ホビーサーチ用の共通パースユーティリティ関数
mod hobbysearch_common;

// ホビーサーチ用パーサー
pub mod hobbysearch_change;
pub mod hobbysearch_change_yoyaku;
pub mod hobbysearch_confirm;
pub mod hobbysearch_confirm_yoyaku;
pub mod hobbysearch_send;

// BatchTask 実装
pub mod email_parse_task;
pub use email_parse_task::{
    EmailParseContext, EmailParseInput, EmailParseOutput, EmailParseTask,
    ShopSettingsCache,
};

/// パース状態管理
#[derive(Clone)]
pub struct ParseState {
    pub should_cancel: Arc<Mutex<bool>>,
    pub is_running: Arc<Mutex<bool>>,
}

impl Default for ParseState {
    fn default() -> Self {
        Self {
            should_cancel: Arc::new(Mutex::new(false)),
            is_running: Arc::new(Mutex::new(false)),
        }
    }
}

impl ParseState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_cancel(&self) {
        if let Ok(mut cancel) = self.should_cancel.lock() {
            *cancel = true;
            log::info!("Parse cancellation requested");
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.should_cancel.lock().map(|c| *c).unwrap_or(false)
    }

    pub fn start(&self) -> Result<(), String> {
        let mut running = self.is_running.lock().map_err(|e| e.to_string())?;
        if *running {
            return Err("Parse is already running".to_string());
        }
        *running = true;
        *self.should_cancel.lock().map_err(|e| e.to_string())? = false;
        Ok(())
    }

    pub fn finish(&self) {
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }
        if let Ok(mut cancel) = self.should_cancel.lock() {
            *cancel = false;
        }
    }
}

/// 注文情報を表す構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderInfo {
    /// 注文番号
    pub order_number: String,
    /// 注文日
    pub order_date: Option<String>,
    /// 配送先情報
    pub delivery_address: Option<DeliveryAddress>,
    /// 配送情報
    pub delivery_info: Option<DeliveryInfo>,
    /// 商品リスト
    pub items: Vec<OrderItem>,
    /// 小計
    pub subtotal: Option<i64>,
    /// 送料
    pub shipping_fee: Option<i64>,
    /// 合計金額
    pub total_amount: Option<i64>,
}

/// 配送先情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryAddress {
    /// 宛名
    pub name: String,
    /// 郵便番号
    pub postal_code: Option<String>,
    /// 住所
    pub address: Option<String>,
}

/// 配送情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryInfo {
    /// 配送会社
    pub carrier: String,
    /// 配送伝票番号
    pub tracking_number: String,
    /// 配送日
    pub delivery_date: Option<String>,
    /// 配送時間
    pub delivery_time: Option<String>,
    /// 配送会社URL
    pub carrier_url: Option<String>,
}

/// 商品情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    /// 商品名
    pub name: String,
    /// メーカー・ブランド
    pub manufacturer: Option<String>,
    /// 型番・品番
    pub model_number: Option<String>,
    /// 単価
    pub unit_price: i64,
    /// 個数
    pub quantity: i64,
    /// 小計
    pub subtotal: i64,
}

/// メールパーサーのトレイト
pub trait EmailParser {
    /// メール本文から注文情報をパースする
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String>;
}

/// バッチパース用: 送信元アドレスと件名から候補パーサー(parser_type, shop_name)を取得
///
/// 同一の送信元・件名に複数パーサーがマッチする場合がある（例: hobbysearch_change と hobbysearch_change_yoyaku）。
/// これらは本文構造が異なり（[ご購入内容] vs [ご予約内容]）、1通のメールに対してはどちらか一方のみが成功する。
/// shop_settings の ORDER BY shop_name, id により試行順序は一意に決まり、
/// 最初に成功したパーサーの結果が採用される。
///
/// # Arguments
/// * `shop_settings` - (sender_address, parser_type, subject_filters_json, shop_name) のタプルリスト
/// * `from_address_opt` - メールの送信元アドレス（None の場合は空を返す）
/// * `subject_opt` - メールの件名（オプション）
pub fn get_candidate_parsers_for_batch(
    shop_settings: &[(String, String, Option<String>, String)],
    from_address_opt: Option<&str>,
    subject_opt: Option<&str>,
) -> Vec<(String, String)> {
    let from_address = match from_address_opt {
        Some(addr) => addr,
        None => return Vec::new(),
    };
    let normalized_from = match extract_email_address(from_address) {
        Some(addr) => addr,
        None => return Vec::new(),
    };
    shop_settings
        .iter()
        .filter_map(|(addr, parser_type, subject_filters_json, shop_name)| {
            let normalized_addr = match extract_email_address(addr) {
                Some(addr) => addr,
                None => return None,
            };
            if !normalized_from.eq_ignore_ascii_case(&normalized_addr) {
                return None;
            }

            let Some(filters_json) = subject_filters_json else {
                return Some((parser_type.clone(), shop_name.clone()));
            };

            let Ok(filters) = serde_json::from_str::<Vec<String>>(filters_json) else {
                return Some((parser_type.clone(), shop_name.clone()));
            };

            let subject = subject_opt?;

            if filters.iter().any(|filter| subject.contains(filter)) {
                Some((parser_type.clone(), shop_name.clone()))
            } else {
                None
            }
        })
        .collect()
}

/// パーサータイプから適切なパーサーを取得する
pub fn get_parser(parser_type: &str) -> Option<Box<dyn EmailParser>> {
    match parser_type {
        "hobbysearch_confirm" => Some(Box::new(hobbysearch_confirm::HobbySearchConfirmParser)),
        "hobbysearch_confirm_yoyaku" => Some(Box::new(
            hobbysearch_confirm_yoyaku::HobbySearchConfirmYoyakuParser,
        )),
        "hobbysearch_change" => Some(Box::new(hobbysearch_change::HobbySearchChangeParser)),
        "hobbysearch_change_yoyaku" => Some(Box::new(
            hobbysearch_change_yoyaku::HobbySearchChangeYoyakuParser,
        )),
        "hobbysearch_send" => Some(Box::new(hobbysearch_send::HobbySearchSendParser)),
        _ => None,
    }
}

/// パースメタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseMetadata {
    pub parse_status: String,
    pub last_parse_started_at: Option<String>,
    pub last_parse_completed_at: Option<String>,
    pub total_parsed_count: i64,
    pub last_error_message: Option<String>,
    pub batch_size: i64,
}

/// パース進捗イベント（後方互換性のため残す）
/// 新しいコードでは BatchProgressEvent を使用してください
#[derive(Debug, Clone, Serialize, Deserialize)]
#[deprecated(note = "Use BatchProgressEvent instead")]
pub struct ParseProgressEvent {
    pub batch_number: usize,
    pub total_emails: usize,
    pub parsed_count: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub status_message: String,
    pub is_complete: bool,
    pub error: Option<String>,
}

/// バッチパース処理（メールから注文情報を抽出）
/// NOTE: 商品名のAI解析（Gemini API）は別コマンド start_product_name_parse で実行
pub async fn batch_parse_emails(
    app_handle: &tauri::AppHandle,
    pool: &SqlitePool,
    parse_state: &ParseState,
    batch_size: usize,
) -> Result<(), String> {
    use chrono::Utc;

    log::info!("Starting batch parse with batch_size: {}", batch_size);

    // パース状態をチェック・開始
    parse_state.start()?;

    // パース状態を「実行中」に更新
    let parse_metadata_repo = SqliteParseMetadataRepository::new(pool.clone());
    parse_metadata_repo
        .update_parse_status("running", Some(Utc::now().to_rfc3339()), None, None, None)
        .await
        .map_err(|e| format!("Failed to update parse status: {e}"))?;

    // order_emails, deliveries, items, orders テーブルをクリア（パースやり直しのため）
    // 外部キー制約により、order_emails -> deliveries -> items -> orders の順でクリア
    // NOTE: ユーザーには事前にUI（Parse画面）で警告と確認ダイアログを表示済み
    log::info!("Clearing order_emails, deliveries, items, and orders tables for fresh parse...");

    let parse_repo = SqliteParseRepository::new(pool.clone());
    parse_repo
        .clear_order_tables()
        .await
        .map_err(|e| format!("Failed to clear order tables: {e}"))?;

    // shop_settingsから有効な店舗とパーサータイプ、件名フィルターを取得
    let shop_settings_repo = SqliteShopSettingsRepository::new(pool.clone());
    let enabled_settings = shop_settings_repo
        .get_enabled()
        .await
        .map_err(|e| format!("Failed to fetch shop settings: {e}"))?;

    if enabled_settings.is_empty() {
        log::warn!("No enabled shop settings found");
        return Err("No enabled shop settings found".to_string());
    }

    let shop_settings: Vec<(String, String, Option<String>, String)> = enabled_settings
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

    // パース対象の全メール数を取得
    let total_email_count = parse_repo
        .get_total_email_count()
        .await
        .map_err(|e| format!("Failed to count emails: {e}"))?;

    log::info!("Total emails to parse: {}", total_email_count);

    let mut overall_success_count = 0;
    let mut overall_failed_count = 0;
    let mut overall_parsed_count = 0;
    let mut iteration = 0;

    // 全メールをパースするまでループ
    loop {
        // 中断チェック
        if parse_state.is_cancelled() {
            log::info!("Parse cancelled by user");
            parse_state.finish();

            // キャンセルイベントを送信
            let cancel_event = BatchProgressEvent::cancelled(
                EMAIL_PARSE_TASK_NAME,
                total_email_count as usize,
                overall_parsed_count,
                overall_success_count,
                overall_failed_count,
            );
            let _ = app_handle.emit(EMAIL_PARSE_EVENT_NAME, cancel_event);

            // ステータスをidleに戻す
            let parse_metadata_repo = SqliteParseMetadataRepository::new(pool.clone());
            let _ = parse_metadata_repo.reset_parse_status().await;

            return Ok(());
        }

        iteration += 1;

        // パース対象のメールを取得（既にパース済みのものを除外）
        // order_emailsテーブルにemail_idが存在しないメールのみ取得
        // メール送信日時（internal_date）の古い順（ASC）でパースすることで、時系列に沿って注文情報が更新される
        let emails = parse_repo
            .get_unparsed_emails(batch_size)
            .await
            .map_err(|e| format!("Failed to fetch emails: {e}"))?;

        if emails.is_empty() {
            log::info!("No more emails to parse");
            break;
        }

        let batch_email_count = emails.len();
        let mut success_count = 0;
        let mut failed_count = 0;

        log::info!(
            "Iteration {}: Found {} emails to parse",
            iteration,
            batch_email_count
        );

        // OrderRepositoryインスタンスをループの外で作成（効率化のため）
        let order_repo = SqliteOrderRepository::new(pool.clone());

        // ===============================================================
        // フェーズ1: 全メールを正規表現でパースし、成功した注文を収集
        // ===============================================================
        struct ParsedOrderData {
            email_id: i64,
            order_info: OrderInfo,
            shop_name: String,
            shop_domain: Option<String>,
        }

        let mut parsed_orders: Vec<ParsedOrderData> = Vec::new();

        for row in emails.iter() {
            // 送信元アドレスと件名フィルターから候補のパーサー(parser_type, shop_name)を全て取得
            let candidate_parsers = get_candidate_parsers_for_batch(
                &shop_settings,
                row.from_address.as_deref(),
                row.subject.as_deref(),
            );

            if candidate_parsers.is_empty() {
                let skip_reason = row
                    .from_address
                    .as_ref()
                    .map(|_| "No matching parser")
                    .unwrap_or("from_address is null");
                log::debug!(
                    "No parser found for address: {:?} with subject: {:?}",
                    row.from_address.as_deref().unwrap_or("(null)"),
                    row.subject
                );
                parse_repo
                    .mark_parse_skipped(row.email_id, skip_reason)
                    .await
                    .map_err(|e| {
                        parse_state.finish();
                        format!(
                            "Failed to mark email {} as skipped (DB error): {}. Aborting to prevent infinite parse loop.",
                            row.email_id, e
                        )
                    })?;
                failed_count += 1;
                overall_parsed_count += 1;
                continue;
            }

            // 複数のパーサーを順番に試す（最初に成功したものを使用）
            let mut parse_result: Option<Result<(OrderInfo, String), String>> = None;
            let mut last_error = String::new();

            for (parser_type, shop_name) in &candidate_parsers {
                let parser = match get_parser(parser_type) {
                    Some(p) => p,
                    None => {
                        log::warn!("Unknown parser type: {}", parser_type);
                        continue;
                    }
                };

                match parser.parse(&row.body_plain) {
                    Ok(mut order_info) => {
                        log::debug!("Successfully parsed with parser: {}", parser_type);

                        // confirm, confirm_yoyaku, change の場合はメール受信日を order_date に使用
                        if order_info.order_date.is_none()
                            && row.internal_date.is_some()
                            && matches!(
                                parser_type.as_str(),
                                "hobbysearch_confirm"
                                    | "hobbysearch_confirm_yoyaku"
                                    | "hobbysearch_change"
                                    | "hobbysearch_change_yoyaku"
                            )
                        {
                            if let Some(ts_ms) = row.internal_date {
                                let dt = match DateTime::from_timestamp_millis(ts_ms) {
                                    Some(d) => d,
                                    None => {
                                        log::warn!(
                                            "Failed to parse internal_date {} for email {} (invalid timestamp), using current time as order_date fallback",
                                            ts_ms, row.email_id
                                        );
                                        chrono::Utc::now()
                                    }
                                };
                                // internal_date は UTC ミリ秒タイムスタンプ。order_date は UTC の日時文字列として DB に保存。
                                // フロントはタイムゾーン未指定を UTC と解釈し JST で表示（README §4 規約）。
                                order_info.order_date =
                                    Some(dt.format("%Y-%m-%d %H:%M:%S").to_string());
                            }
                        }

                        parse_result = Some(Ok((order_info, shop_name.clone())));
                        break;
                    }
                    Err(e) => {
                        log::debug!("Parser {} failed: {}", parser_type, e);
                        last_error = e;
                        continue;
                    }
                }
            }

            let parse_result = match parse_result {
                Some(result) => result,
                None => {
                    log::error!(
                        "All parsers failed for email {}. Last error: {}",
                        row.email_id,
                        last_error
                    );
                    Err(last_error)
                }
            };

            match parse_result {
                Ok((order_info, shop_name)) => {
                    // ドメインを抽出（extract_email_address で <> 形式に対応、extract_domain でドメイン部分のみ取得）
                    let from_address = row.from_address.as_deref().unwrap_or("");
                    let shop_domain = extract_email_address(from_address)
                        .and_then(|email| extract_domain(&email).map(|s| s.to_string()));

                    parsed_orders.push(ParsedOrderData {
                        email_id: row.email_id,
                        order_info,
                        shop_name,
                        shop_domain,
                    });
                }
                Err(e) => {
                    log::error!("Failed to parse email {}: {}", row.email_id, e);
                    parse_repo
                        .mark_parse_skipped(row.email_id, &e)
                        .await
                        .map_err(|mark_err| {
                            parse_state.finish();
                            format!(
                                "Failed to mark email {} as skipped (DB error): {}. Aborting to prevent infinite parse loop.",
                                row.email_id, mark_err
                            )
                        })?;
                    failed_count += 1;
                    overall_parsed_count += 1;
                }
            }
        }

        // ===============================================================
        // フェーズ2: 全注文をDBに保存
        // ===============================================================
        for order_data in parsed_orders {
            match order_repo
                .save_order(
                    &order_data.order_info,
                    Some(order_data.email_id),
                    order_data.shop_domain,
                    Some(order_data.shop_name),
                )
                .await
            {
                Ok(order_id) => {
                    log::info!("Successfully parsed and saved order: {}", order_id);
                    success_count += 1;
                }
                Err(e) => {
                    log::error!("Failed to save order: {}", e);
                    parse_repo
                        .mark_parse_skipped(order_data.email_id, &e)
                        .await
                        .map_err(|mark_err| {
                            parse_state.finish();
                            format!(
                                "Failed to mark email {} as skipped (DB error): {}. Aborting to prevent infinite parse loop.",
                                order_data.email_id, mark_err
                            )
                        })?;
                    failed_count += 1;
                }
            }
            overall_parsed_count += 1;
        }

        // バッチ処理完了後に進捗イベントを送信（バッチごとに1回）
        overall_success_count += success_count;
        overall_failed_count += failed_count;

        let progress = BatchProgressEvent::progress(
            EMAIL_PARSE_TASK_NAME,
            iteration,
            batch_email_count,
            total_email_count as usize,
            overall_parsed_count,
            overall_success_count,
            overall_failed_count,
            format!(
                "パース中... ({}/{})",
                overall_parsed_count, total_email_count
            ),
        );

        let _ = app_handle.emit(EMAIL_PARSE_EVENT_NAME, progress);

        log::info!(
            "Iteration {} completed: success={}, failed={}",
            iteration,
            success_count,
            failed_count
        );
    }

    // 完了イベントを送信
    let final_progress = BatchProgressEvent::complete(
        EMAIL_PARSE_TASK_NAME,
        total_email_count as usize,
        overall_success_count,
        overall_failed_count,
        format!(
            "パース完了: 成功 {}, 失敗 {}",
            overall_success_count, overall_failed_count
        ),
    );

    let _ = app_handle.emit(EMAIL_PARSE_EVENT_NAME, final_progress);

    // メタデータを更新
    parse_metadata_repo
        .update_parse_status(
            "completed",
            None,
            Some(Utc::now().to_rfc3339()),
            Some(overall_success_count as i64),
            None,
        )
        .await
        .map_err(|e| format!("Failed to update parse metadata: {e}"))?;

    log::info!(
        "Batch parse completed: success={}, failed={}",
        overall_success_count,
        overall_failed_count
    );

    // パース状態をクリーンアップ
    parse_state.finish();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ParseState Tests ====================

    #[test]
    fn test_parse_state_new() {
        let state = ParseState::new();
        assert!(!state.is_cancelled());
        assert!(state.should_cancel.lock().unwrap().eq(&false));
        assert!(state.is_running.lock().unwrap().eq(&false));
    }

    #[test]
    fn test_parse_state_request_cancel() {
        let state = ParseState::new();
        assert!(!state.is_cancelled());

        state.request_cancel();
        assert!(state.is_cancelled());
    }

    #[test]
    fn test_parse_state_start_success() {
        let state = ParseState::new();
        let result = state.start();
        assert!(result.is_ok());
        assert!(*state.is_running.lock().unwrap());
    }

    #[test]
    fn test_parse_state_start_already_running() {
        let state = ParseState::new();

        // 最初のstart
        let result = state.start();
        assert!(result.is_ok());

        // 2回目のstartはエラー
        let result = state.start();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Parse is already running");
    }

    #[test]
    fn test_parse_state_start_resets_cancel_flag() {
        let state = ParseState::new();
        state.request_cancel();
        assert!(state.is_cancelled());

        let result = state.start();
        assert!(result.is_ok());
        assert!(!state.is_cancelled());
    }

    #[test]
    fn test_parse_state_finish() {
        let state = ParseState::new();
        state.start().unwrap();
        state.request_cancel();

        assert!(*state.is_running.lock().unwrap());
        assert!(state.is_cancelled());

        state.finish();

        assert!(!*state.is_running.lock().unwrap());
        assert!(!state.is_cancelled());
    }

    #[test]
    fn test_parse_state_finish_when_not_running() {
        let state = ParseState::new();
        // finishを呼んでもパニックしない
        state.finish();
        assert!(!*state.is_running.lock().unwrap());
    }

    #[test]
    fn test_parse_state_multiple_cycles() {
        let state = ParseState::new();

        // サイクル1
        state.start().unwrap();
        state.request_cancel();
        state.finish();

        // サイクル2
        state.start().unwrap();
        assert!(!state.is_cancelled());
        state.finish();

        // サイクル3
        state.start().unwrap();
        state.finish();

        assert!(!*state.is_running.lock().unwrap());
    }

    // ==================== get_parser Tests ====================

    #[test]
    fn test_get_parser_hobbysearch_confirm() {
        let parser = get_parser("hobbysearch_confirm");
        assert!(parser.is_some());
    }

    #[test]
    fn test_get_parser_hobbysearch_confirm_yoyaku() {
        let parser = get_parser("hobbysearch_confirm_yoyaku");
        assert!(parser.is_some());
    }

    #[test]
    fn test_get_parser_hobbysearch_change() {
        let parser = get_parser("hobbysearch_change");
        assert!(parser.is_some());
    }

    #[test]
    fn test_get_parser_hobbysearch_change_yoyaku() {
        let parser = get_parser("hobbysearch_change_yoyaku");
        assert!(parser.is_some());
    }

    #[test]
    fn test_get_parser_hobbysearch_send() {
        let parser = get_parser("hobbysearch_send");
        assert!(parser.is_some());
    }

    #[test]
    fn test_get_parser_unknown_type() {
        let parser = get_parser("unknown_parser");
        assert!(parser.is_none());
    }

    #[test]
    fn test_get_parser_empty_string() {
        let parser = get_parser("");
        assert!(parser.is_none());
    }

    // ==================== Data Structure Tests ====================

    #[test]
    fn test_order_info_structure() {
        let order = OrderInfo {
            order_number: "ORD-001".to_string(),
            order_date: Some("2024-01-01".to_string()),
            delivery_address: None,
            delivery_info: None,
            items: vec![],
            subtotal: Some(1000),
            shipping_fee: Some(500),
            total_amount: Some(1500),
        };

        assert_eq!(order.order_number, "ORD-001");
        assert_eq!(order.order_date, Some("2024-01-01".to_string()));
        assert_eq!(order.total_amount, Some(1500));
    }

    #[test]
    fn test_order_info_with_items() {
        let item = OrderItem {
            name: "Test Product".to_string(),
            manufacturer: Some("Test Maker".to_string()),
            model_number: Some("MODEL-001".to_string()),
            unit_price: 1000,
            quantity: 2,
            subtotal: 2000,
        };

        let order = OrderInfo {
            order_number: "ORD-002".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![item],
            subtotal: None,
            shipping_fee: None,
            total_amount: None,
        };

        assert_eq!(order.items.len(), 1);
        assert_eq!(order.items[0].name, "Test Product");
        assert_eq!(order.items[0].subtotal, 2000);
    }

    #[test]
    fn test_delivery_address_structure() {
        let addr = DeliveryAddress {
            name: "山田太郎".to_string(),
            postal_code: Some("123-4567".to_string()),
            address: Some("東京都渋谷区".to_string()),
        };

        assert_eq!(addr.name, "山田太郎");
        assert_eq!(addr.postal_code, Some("123-4567".to_string()));
    }

    #[test]
    fn test_delivery_info_structure() {
        let info = DeliveryInfo {
            carrier: "ヤマト運輸".to_string(),
            tracking_number: "1234-5678-9012".to_string(),
            delivery_date: Some("2024-01-15".to_string()),
            delivery_time: Some("14:00-16:00".to_string()),
            carrier_url: Some("https://example.com/track".to_string()),
        };

        assert_eq!(info.carrier, "ヤマト運輸");
        assert_eq!(info.tracking_number, "1234-5678-9012");
    }

    #[test]
    fn test_parse_metadata_structure() {
        let metadata = ParseMetadata {
            parse_status: "idle".to_string(),
            last_parse_started_at: None,
            last_parse_completed_at: None,
            total_parsed_count: 0,
            last_error_message: None,
            batch_size: 100,
        };

        assert_eq!(metadata.parse_status, "idle");
        assert_eq!(metadata.batch_size, 100);
    }

    #[test]
    fn test_parse_progress_event_structure() {
        let event = ParseProgressEvent {
            batch_number: 1,
            total_emails: 100,
            parsed_count: 50,
            success_count: 45,
            failed_count: 5,
            status_message: "Processing...".to_string(),
            is_complete: false,
            error: None,
        };

        assert_eq!(event.batch_number, 1);
        assert_eq!(event.total_emails, 100);
        assert!(!event.is_complete);
    }

    #[test]
    fn test_parse_progress_event_with_error() {
        let event = ParseProgressEvent {
            batch_number: 2,
            total_emails: 100,
            parsed_count: 30,
            success_count: 25,
            failed_count: 5,
            status_message: "Error occurred".to_string(),
            is_complete: true,
            error: Some("Database connection failed".to_string()),
        };

        assert!(event.is_complete);
        assert!(event.error.is_some());
    }

    // ==================== Serialization Tests ====================

    #[test]
    fn test_order_info_serialization() {
        let order = OrderInfo {
            order_number: "ORD-003".to_string(),
            order_date: Some("2024-01-01".to_string()),
            delivery_address: None,
            delivery_info: None,
            items: vec![],
            subtotal: Some(1000),
            shipping_fee: None,
            total_amount: Some(1000),
        };

        let json = serde_json::to_string(&order).unwrap();
        assert!(json.contains("ORD-003"));

        let deserialized: OrderInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.order_number, "ORD-003");
    }

    #[test]
    fn test_parse_metadata_serialization() {
        let metadata = ParseMetadata {
            parse_status: "running".to_string(),
            last_parse_started_at: Some("2024-01-01T10:00:00Z".to_string()),
            last_parse_completed_at: None,
            total_parsed_count: 50,
            last_error_message: None,
            batch_size: 200,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: ParseMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.parse_status, "running");
        assert_eq!(deserialized.total_parsed_count, 50);
    }

    #[test]
    fn test_parse_progress_event_serialization() {
        let event = ParseProgressEvent {
            batch_number: 5,
            total_emails: 500,
            parsed_count: 250,
            success_count: 240,
            failed_count: 10,
            status_message: "Half done".to_string(),
            is_complete: false,
            error: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ParseProgressEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.batch_number, 5);
        assert_eq!(deserialized.success_count, 240);
    }

    #[test]
    fn test_order_item_serialization() {
        let item = OrderItem {
            name: "ガンプラ HG".to_string(),
            manufacturer: Some("バンダイ".to_string()),
            model_number: Some("BAN-001".to_string()),
            unit_price: 2500,
            quantity: 1,
            subtotal: 2500,
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("ガンプラ HG"));
        assert!(json.contains("バンダイ"));

        let deserialized: OrderItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.unit_price, 2500);
    }

    #[test]
    fn test_delivery_info_serialization() {
        let info = DeliveryInfo {
            carrier: "佐川急便".to_string(),
            tracking_number: "9999-8888-7777".to_string(),
            delivery_date: None,
            delivery_time: None,
            carrier_url: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: DeliveryInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.carrier, "佐川急便");
        assert_eq!(deserialized.tracking_number, "9999-8888-7777");
    }

    // ==================== Clone Tests ====================

    #[test]
    fn test_parse_state_clone() {
        let state = ParseState::new();
        state.start().unwrap();

        let cloned = state.clone();
        // クローンは同じArcを共有
        assert!(*cloned.is_running.lock().unwrap());

        state.finish();
        assert!(!*cloned.is_running.lock().unwrap());
    }

    #[test]
    fn test_order_info_clone() {
        let order = OrderInfo {
            order_number: "ORD-CLONE".to_string(),
            order_date: None,
            delivery_address: None,
            delivery_info: None,
            items: vec![OrderItem {
                name: "Item".to_string(),
                manufacturer: None,
                model_number: None,
                unit_price: 100,
                quantity: 1,
                subtotal: 100,
            }],
            subtotal: None,
            shipping_fee: None,
            total_amount: None,
        };

        let cloned = order.clone();
        assert_eq!(cloned.order_number, "ORD-CLONE");
        assert_eq!(cloned.items.len(), 1);
    }

    // ==================== get_candidate_parsers_for_batch Tests ====================

    fn make_shop_setting(
        addr: &str,
        parser_type: &str,
        subject_filters: Option<Vec<String>>,
        shop_name: &str,
    ) -> (String, String, Option<String>, String) {
        let filters_json = subject_filters.map(|f| serde_json::to_string(&f).unwrap());
        (
            addr.to_string(),
            parser_type.to_string(),
            filters_json,
            shop_name.to_string(),
        )
    }

    #[test]
    fn test_get_candidate_parsers_for_batch_address_match_no_filter() {
        let settings = vec![make_shop_setting(
            "order@hobbysearch.co.jp",
            "hobbysearch_confirm",
            None,
            "ホビーサーチ",
        )];
        let result = get_candidate_parsers_for_batch(
            &settings,
            Some("order@hobbysearch.co.jp"),
            Some("件名"),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "hobbysearch_confirm");
        assert_eq!(result[0].1, "ホビーサーチ");
    }

    #[test]
    fn test_get_candidate_parsers_for_batch_from_address_none_returns_empty() {
        let settings = vec![make_shop_setting(
            "order@hobbysearch.co.jp",
            "hobbysearch_confirm",
            None,
            "ホビーサーチ",
        )];
        let result = get_candidate_parsers_for_batch(&settings, None, Some("件名"));
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_candidate_parsers_for_batch_address_no_match() {
        let settings = vec![make_shop_setting(
            "order@hobbysearch.co.jp",
            "hobbysearch_confirm",
            None,
            "ホビーサーチ",
        )];
        let result =
            get_candidate_parsers_for_batch(&settings, Some("other@example.com"), Some("件名"));
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_candidate_parsers_for_batch_exact_match_angle_bracket() {
        // "Name <email>" 形式から正規化して完全一致で比較
        let settings = vec![make_shop_setting(
            "order@hobbysearch.co.jp",
            "hobbysearch_confirm",
            None,
            "ホビーサーチ",
        )];
        let result = get_candidate_parsers_for_batch(
            &settings,
            Some("Hobby Search <order@hobbysearch.co.jp>"),
            Some("件名"),
        );
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_get_candidate_parsers_for_batch_no_partial_match() {
        // 部分一致は許可しない（myshop@example.com が shop@example.com にマッチしない）
        let settings = vec![make_shop_setting(
            "shop@example.com",
            "hobbysearch_confirm",
            None,
            "ショップ",
        )];
        let result =
            get_candidate_parsers_for_batch(&settings, Some("myshop@example.com"), Some("件名"));
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_candidate_parsers_for_batch_subject_filter_match() {
        let settings = vec![make_shop_setting(
            "order@hobbysearch.co.jp",
            "hobbysearch_confirm",
            Some(vec!["注文確認".to_string()]),
            "ホビーサーチ",
        )];
        let result = get_candidate_parsers_for_batch(
            &settings,
            Some("order@hobbysearch.co.jp"),
            Some("【ホビーサーチ】注文確認メール"),
        );
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_get_candidate_parsers_for_batch_subject_filter_no_match() {
        let settings = vec![make_shop_setting(
            "order@hobbysearch.co.jp",
            "hobbysearch_confirm",
            Some(vec!["注文確認".to_string()]),
            "ホビーサーチ",
        )];
        let result = get_candidate_parsers_for_batch(
            &settings,
            Some("order@hobbysearch.co.jp"),
            Some("【ホビーサーチ】発送通知"),
        );
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_candidate_parsers_for_batch_subject_filter_no_subject_excluded() {
        let settings = vec![make_shop_setting(
            "order@hobbysearch.co.jp",
            "hobbysearch_confirm",
            Some(vec!["注文確認".to_string()]),
            "ホビーサーチ",
        )];
        let result =
            get_candidate_parsers_for_batch(&settings, Some("order@hobbysearch.co.jp"), None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_candidate_parsers_for_batch_invalid_json_ignores_filter() {
        let settings = vec![(
            "order@hobbysearch.co.jp".to_string(),
            "hobbysearch_confirm".to_string(),
            Some("invalid json".to_string()),
            "ホビーサーチ".to_string(),
        )];
        // JSONパース失敗時はフィルターを無視してパーサーを返す
        let result = get_candidate_parsers_for_batch(
            &settings,
            Some("order@hobbysearch.co.jp"),
            Some("任意の件名"),
        );
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_get_candidate_parsers_for_batch_multiple_matches() {
        let settings = vec![
            make_shop_setting(
                "order@hobbysearch.co.jp",
                "hobbysearch_confirm",
                None,
                "店1",
            ),
            make_shop_setting("order@hobbysearch.co.jp", "hobbysearch_send", None, "店2"),
        ];
        let result = get_candidate_parsers_for_batch(
            &settings,
            Some("order@hobbysearch.co.jp"),
            Some("件名"),
        );
        assert_eq!(result.len(), 2);
    }

    // ==================== batch_parse_emails Tests ====================
    //
    // batch_parse_emails関数のテストは、AppHandleを必要とするため、
    // 統合テストとして実装する必要があります。
    // 統合テストは tests/parser_integration_tests.rs に実装されています。
}
