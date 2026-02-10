use crate::batch_runner::BatchProgressEvent;
use crate::logic::email_parser::extract_domain;
use tauri::Manager;
use crate::logic::sync_logic::extract_email_address;
use crate::parsers::cancel_info::CancelInfo;
use crate::repository::{
    OrderRepository, ParseRepository, ShopSettingsRepository, SqliteOrderRepository,
    SqliteParseRepository, SqliteShopSettingsRepository,
};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

// 定数はemail_parse_taskモジュールからエクスポート
pub use email_parse_task::{EMAIL_PARSE_EVENT_NAME, EMAIL_PARSE_TASK_NAME};

/// パース対象メールの情報（get_unparsed_emails の戻り値）
#[derive(Debug, Clone, FromRow)]
pub struct EmailRow {
    #[sqlx(rename = "id")]
    pub email_id: i64,
    pub message_id: String,
    pub body_plain: Option<String>,
    pub body_html: Option<String>,
    pub from_address: Option<String>,
    pub subject: Option<String>,
    pub internal_date: Option<i64>,
}

/// 注文検索で試す追加ドメインを返す（店舗固有）。
/// DMM: 注文確認は mail.dmm.com / mono.dmm.com のどちらかから届く。キャンセル・注文番号変更は mail.dmm.com から。
/// 呼び出し元（apply_cancel / apply_order_number_change）で alternate_domains として渡す。
pub fn order_lookup_alternate_domains(shop_domain: &Option<String>) -> Option<Vec<String>> {
    match shop_domain.as_deref() {
        Some("mail.dmm.com") => Some(vec!["mono.dmm.com".into()]),
        Some("mono.dmm.com") => Some(vec!["mail.dmm.com".into()]),
        _ => None,
    }
}

/// body_html があれば使用、なければ body_plain を返す（タグ除去は行わない）。
/// DMM 等は HTML から直接パースするため、HTML 優先で精度が上がる。
pub fn get_body_for_parse(row: &EmailRow) -> String {
    let html = row.body_html.as_deref().unwrap_or("").trim();
    if !html.is_empty() {
        return html.to_string();
    }
    row.body_plain
        .as_deref()
        .unwrap_or("")
        .to_string()
}

// キャンセル情報（全店舗共通）
pub mod cancel_info;
// 注文番号変更情報（全店舗共通）
pub mod order_number_change_info;
// まとめ完了情報（全店舗共通）
pub mod consolidation_info;

// ホビーサーチ用の共通パースユーティリティ関数
mod hobbysearch_common;

// ホビーサーチ用パーサー
pub mod hobbysearch_cancel;
pub mod hobbysearch_change;
pub mod hobbysearch_change_yoyaku;
pub mod hobbysearch_confirm;
pub mod hobbysearch_confirm_yoyaku;
pub mod hobbysearch_send;
pub mod dmm_confirm;
pub mod dmm_cancel;
pub mod dmm_order_number_change;
pub mod dmm_merge_complete;
pub mod dmm_split_complete;

// BatchTask 実装
pub mod email_parse_task;
pub use email_parse_task::{
    EmailParseContext, EmailParseInput, EmailParseOutput, EmailParseTask, ShopSettingsCache,
};

/// パース状態管理
///
/// 進捗テーブル削除後はメモリのみで状態を管理する。
/// last_error はエラー時に設定され、次回 start でクリアされる。
#[derive(Clone)]
pub struct ParseState {
    pub should_cancel: Arc<Mutex<bool>>,
    pub is_running: Arc<Mutex<bool>>,
    /// 直近のエラーメッセージ（エラー時のみ。start でクリア）
    pub last_error: Arc<Mutex<Option<String>>>,
}

impl Default for ParseState {
    fn default() -> Self {
        Self {
            should_cancel: Arc::new(Mutex::new(false)),
            is_running: Arc::new(Mutex::new(false)),
            last_error: Arc::new(Mutex::new(None)),
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

    /// エラーを記録（get_parse_status で error として返す）
    pub fn set_error(&self, msg: &str) {
        if let Ok(mut err) = self.last_error.lock() {
            *err = Some(msg.to_string());
        }
    }

    /// エラーをクリア
    pub fn clear_error(&self) {
        if let Ok(mut err) = self.last_error.lock() {
            *err = None;
        }
    }

    /// 強制的に idle にリセット
    pub fn force_idle(&self) {
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }
        self.clear_error();
    }

    pub fn start(&self) -> Result<(), String> {
        let mut running = self.is_running.lock().map_err(|e| e.to_string())?;
        if *running {
            return Err("Parse is already running".to_string());
        }
        *running = true;
        *self.should_cancel.lock().map_err(|e| e.to_string())? = false;
        self.clear_error();
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
    /// 商品画像URL（注文確認メールに含まれる場合、images テーブルへ登録する）
    pub image_url: Option<String>,
}

/// メールパーサーのトレイト
pub trait EmailParser {
    /// メール本文から注文情報をパースする
    fn parse(&self, email_body: &str) -> Result<OrderInfo, String>;

    /// 1通のメールから複数注文を返す場合に実装する。
    /// デフォルトは None（単一注文として parse() を使用）。
    /// Some(Ok(orders)) を返すとバッチで各注文を save_order する。
    fn parse_multi(&self, _email_body: &str) -> Option<Result<Vec<OrderInfo>, String>> {
        None
    }
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

/// パーサータイプがキャンセル専用かどうか
pub(crate) fn is_cancel_parser(parser_type: &str) -> bool {
    matches!(parser_type, "hobbysearch_cancel" | "dmm_cancel")
}

/// パーサータイプが注文番号変更専用かどうか
pub(crate) fn is_order_number_change_parser(parser_type: &str) -> bool {
    matches!(parser_type, "dmm_order_number_change")
}

/// パーサータイプがまとめ完了専用かどうか
pub(crate) fn is_merge_complete_parser(parser_type: &str) -> bool {
    matches!(parser_type, "dmm_merge_complete")
}

/// キャンセルパーサーから CancelInfo を抽出（失敗時は Err）
pub(crate) fn parse_cancel_with_parser(parser_type: &str, body: &str) -> Result<CancelInfo, String> {
    match parser_type {
        "hobbysearch_cancel" => hobbysearch_cancel::HobbySearchCancelParser.parse_cancel(body),
        "dmm_cancel" => dmm_cancel::DmmCancelParser.parse_cancel(body),
        _ => Err(format!("Unknown cancel parser: {}", parser_type)),
    }
}

/// 注文番号変更パーサーから OrderNumberChangeInfo を抽出（失敗時は Err）
pub(crate) fn parse_order_number_change_with_parser(
    parser_type: &str,
    body: &str,
) -> Result<order_number_change_info::OrderNumberChangeInfo, String> {
    match parser_type {
        "dmm_order_number_change" => {
            dmm_order_number_change::DmmOrderNumberChangeParser.parse_order_number_change(body)
        }
        _ => Err(format!("Unknown order number change parser: {}", parser_type)),
    }
}

/// まとめ完了パーサーから ConsolidationInfo を抽出（失敗時は Err）
pub(crate) fn parse_consolidation_with_parser(
    parser_type: &str,
    body: &str,
) -> Result<consolidation_info::ConsolidationInfo, String> {
    match parser_type {
        "dmm_merge_complete" => {
            dmm_merge_complete::DmmMergeCompleteParser.parse_consolidation(body)
        }
        _ => Err(format!("Unknown merge complete parser: {}", parser_type)),
    }
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
        "dmm_confirm" => Some(Box::new(dmm_confirm::DmmConfirmParser)),
        "dmm_split_complete" => Some(Box::new(dmm_split_complete::DmmSplitCompleteParser)),
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

/// バッチパース処理（メールから注文情報を抽出）
/// NOTE: 商品名のAI解析（Gemini API）は別コマンド start_product_name_parse で実行
pub async fn batch_parse_emails(
    app_handle: &tauri::AppHandle,
    pool: &SqlitePool,
    parse_state: &ParseState,
    batch_size: usize,
) -> Result<(), String> {
    log::info!("Starting batch parse with batch_size: {}", batch_size);

    // パース状態をチェック・開始（start で is_running=true, clear_error）
    parse_state.start()?;

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
        if !emails.is_empty() {
            let first = &emails[0];
            let last = emails.last().unwrap();
            log::debug!(
                "[DEBUG] batch order: first email_id={} internal_date={:?} subject={:?}",
                first.email_id,
                first.internal_date,
                first.subject.as_deref()
            );
            if emails.len() > 1 {
                log::debug!(
                    "[DEBUG] batch order: last email_id={} internal_date={:?} subject={:?}",
                    last.email_id,
                    last.internal_date,
                    last.subject.as_deref()
                );
            }
        }

        // OrderRepositoryインスタンスをループの外で作成（効率化のため）
        let order_repo = SqliteOrderRepository::new(pool.clone());

        // 全メールをパース（confirm/change は即時 save_order、cancel は apply_cancel）
        for row in emails.iter() {
            // 送信元アドレスと件名フィルターから候補のパーサー(parser_type, shop_name)を全て取得
            let candidate_parsers = get_candidate_parsers_for_batch(
                &shop_settings,
                row.from_address.as_deref(),
                row.subject.as_deref(),
            );

            if candidate_parsers.is_empty() {
                log::debug!(
                    "No parser found for address: {:?} with subject: {:?}",
                    row.from_address.as_deref().unwrap_or("(null)"),
                    row.subject.as_deref(),
                );
                failed_count += 1;
                overall_parsed_count += 1;
                continue;
            }

            // 複数のパーサーを順番に試す（最初に成功したものを使用）
            let mut parse_result: Option<Result<(OrderInfo, String, String), String>> = None; // (order_info, shop_name, parser_type)
            let mut last_error = String::new();
            // キャンセルメールとして処理した（成功・失敗いずれも）。通常の OrderInfo フローをスキップする。
            let mut handled_as_cancel = false;
            // 複数注文パーサーで保存済み（後段の save_order をスキップする）
            let mut handled_as_multi = false;

            for (parser_type, shop_name) in &candidate_parsers {
                // キャンセルメールは専用パーサーで処理（OrderInfo を返さない）
                if is_cancel_parser(parser_type) {
                    let body = get_body_for_parse(row);
                    match parse_cancel_with_parser(parser_type, &body) {
                        Ok(cancel_info) => {
                            log::debug!(
                                "[DEBUG] cancel email_id={} internal_date={:?} order_number={} subject={:?}",
                                row.email_id,
                                row.internal_date,
                                cancel_info.order_number,
                                row.subject
                            );
                            let from_address = row.from_address.as_deref().unwrap_or("");
                            let shop_domain = extract_email_address(from_address)
                                .and_then(|email| extract_domain(&email).map(|s| s.to_string()));

                            match order_repo
                                .apply_cancel(
                                    &cancel_info,
                                    row.email_id,
                                    shop_domain.clone(),
                                    Some(shop_name.clone()),
                                    order_lookup_alternate_domains(&shop_domain),
                                )
                                .await
                            {
                                Ok(order_id) => {
                                    log::info!(
                                        "Successfully applied cancel for order {} (email {})",
                                        order_id,
                                        row.email_id
                                    );
                                    success_count += 1;
                                    overall_parsed_count += 1;
                                }
                                Err(e) => {
                                    // 注文未作成等で失敗した場合、メールは未パースのまま残り次回 run で再試行される
                                    log::warn!(
                                        "Failed to apply cancel for email {} (will retry next run): {}",
                                        row.email_id,
                                        e
                                    );
                                    // failed_count は加算しない（リトライ前提のため）
                                }
                            }
                            handled_as_cancel = true;
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

                let body = get_body_for_parse(row);

                // 複数注文を返すパーサー（dmm_split_complete 等）
                if let Some(multi_result) = parser.parse_multi(&body) {
                    match multi_result {
                        Ok(orders) if !orders.is_empty() => {
                            let from_address = row.from_address.as_deref().unwrap_or("");
                            let shop_domain = extract_email_address(from_address)
                                .and_then(|email| extract_domain(&email).map(|s| s.to_string()));
                            let alternate_domains = order_lookup_alternate_domains(&shop_domain);
                            let mut first_order_info = None;
                            for (idx, mut order_info) in orders.into_iter().enumerate() {
                                if order_info.order_date.is_none()
                                    && row.internal_date.is_some()
                                    && matches!(parser_type.as_str(), "dmm_split_complete")
                                {
                                    if let Some(ts_ms) = row.internal_date {
                                        if let Some(dt) = DateTime::from_timestamp_millis(ts_ms) {
                                            order_info.order_date =
                                                Some(dt.format("%Y-%m-%d %H:%M:%S").to_string());
                                        }
                                    }
                                }
                                let save_result = if idx == 0
                                    && parser_type.as_str() == "dmm_split_complete"
                                {
                                    order_repo
                                        .apply_split_first_order(
                                            &order_info,
                                            Some(row.email_id),
                                            shop_domain.clone(),
                                            Some(shop_name.clone()),
                                            alternate_domains.clone(),
                                        )
                                        .await
                                } else {
                                    order_repo
                                        .save_order(
                                            &order_info,
                                            Some(row.email_id),
                                            shop_domain.clone(),
                                            Some(shop_name.clone()),
                                        )
                                        .await
                                };
                                match save_result {
                                    Ok(order_id) => {
                                        log::info!(
                                            "Saved split order {} ({} of N) for email {}",
                                            order_id,
                                            idx + 1,
                                            row.email_id
                                        );
                                        success_count += 1;
                                        if first_order_info.is_none() {
                                            first_order_info = Some(order_info);
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!(
                                            "Failed to save split order {} for email {}: {}",
                                            idx + 1,
                                            row.email_id,
                                            e
                                        );
                                        last_error = e;
                                        break;
                                    }
                                }
                            }
                            if first_order_info.is_some() {
                                handled_as_multi = true;
                            }
                            overall_parsed_count += 1;
                            break;
                        }
                        Ok(_) => {
                            last_error = "Parser returned empty orders".to_string();
                            continue;
                        }
                        Err(e) => {
                            log::debug!("Parser {} parse_multi failed: {}", parser_type, e);
                            last_error = e;
                            continue;
                        }
                    }
                } else {
                    match parser.parse(&body) {
                        Ok(mut order_info) => {
                            log::debug!("Successfully parsed with parser: {}", parser_type);

                            // confirm, confirm_yoyaku, change, dmm_confirm の場合はメール受信日を order_date に使用
                            if order_info.order_date.is_none()
                                && row.internal_date.is_some()
                                && matches!(
                                    parser_type.as_str(),
                                    "hobbysearch_confirm"
                                        | "hobbysearch_confirm_yoyaku"
                                        | "hobbysearch_change"
                                        | "hobbysearch_change_yoyaku"
                                        | "dmm_confirm"
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

                            parse_result =
                                Some(Ok((order_info, shop_name.clone(), parser_type.clone())));
                            break;
                        }
                        Err(e) => {
                            log::debug!("Parser {} failed: {}", parser_type, e);
                            last_error = e;
                            continue;
                        }
                    }
                }
            }

            if handled_as_cancel {
                continue;
            }
            if handled_as_multi {
                continue;
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
                Ok((order_info, shop_name, parser_type)) => {
                    // ドメインを抽出（extract_email_address で <> 形式に対応、extract_domain でドメイン部分のみ取得）
                    let from_address = row.from_address.as_deref().unwrap_or("");
                    let shop_domain = extract_email_address(from_address)
                        .and_then(|email| extract_domain(&email).map(|s| s.to_string()));

                    // 組み換えメールの場合、同一トランザクションで元注文削除＋新注文登録（データ欠損を防ぐ）
                    // internal_date が無効値の場合、cutoff に使わず None を渡す
                    let change_email_internal_date = row
                        .internal_date
                        .and_then(|ts| DateTime::from_timestamp_millis(ts).map(|_| ts));
                    let save_result = if parser_type == "hobbysearch_change"
                        || parser_type == "hobbysearch_change_yoyaku"
                    {
                        order_repo
                            .apply_change_items_and_save_order(
                                &order_info,
                                Some(row.email_id),
                                shop_domain.clone(),
                                Some(shop_name.clone()),
                                change_email_internal_date,
                            )
                            .await
                    } else {
                        order_repo
                            .save_order(
                                &order_info,
                                Some(row.email_id),
                                shop_domain.clone(),
                                Some(shop_name.clone()),
                            )
                            .await
                    };

                    match save_result {
                        Ok(order_id) => {
                            log::info!("Successfully parsed and saved order: {}", order_id);
                            success_count += 1;
                            // 商品画像URLがある場合、images テーブルに登録
                            if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
                                let images_dir = app_data_dir.join("images");
                                for item in &order_info.items {
                                    if let Some(ref url) = item.image_url {
                                        let normalized =
                                            crate::gemini::normalize_product_name(&item.name);
                                        if !normalized.is_empty() {
                                            if let Err(e) = crate::image_utils::save_image_from_url_for_item(
                                                &pool,
                                                &images_dir,
                                                &normalized,
                                                url,
                                                true, // パース: 既存レコードがあればスキップ
                                            )
                                            .await
                                            {
                                                log::warn!(
                                                    "[batch_parse] Failed to save image for item '{}': {}",
                                                    item.name,
                                                    e
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to save order: {}", e);
                            failed_count += 1;
                        }
                    }
                    overall_parsed_count += 1;
                }
                Err(e) => {
                    log::error!("Failed to parse email {}: {}", row.email_id, e);
                    failed_count += 1;
                    overall_parsed_count += 1;
                }
            }
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

    parse_state.finish();

    log::info!(
        "Batch parse completed: success={}, failed={}",
        overall_success_count,
        overall_failed_count
    );

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

    // ==================== get_body_for_parse Tests ====================

    #[test]
    fn test_get_body_for_parse_prefers_html() {
        let row = EmailRow {
            email_id: 1,
            message_id: "m1".to_string(),
            body_plain: Some("plain text".to_string()),
            body_html: Some("<p>html</p>".to_string()),
            from_address: None,
            subject: None,
            internal_date: None,
        };
        assert_eq!(get_body_for_parse(&row), "<p>html</p>");
    }

    #[test]
    fn test_get_body_for_parse_fallback_to_plain() {
        let row = EmailRow {
            email_id: 1,
            message_id: "m1".to_string(),
            body_plain: Some("plain text".to_string()),
            body_html: None,
            from_address: None,
            subject: None,
            internal_date: None,
        };
        assert_eq!(get_body_for_parse(&row), "plain text");
    }

    #[test]
    fn test_get_body_for_parse_html_only() {
        let row = EmailRow {
            email_id: 1,
            message_id: "m1".to_string(),
            body_plain: None,
            body_html: Some("<p>注文番号:12345</p>".to_string()),
            from_address: None,
            subject: None,
            internal_date: None,
        };
        let body = get_body_for_parse(&row);
        assert!(body.contains("注文番号:12345"));
        assert!(body.contains("<p>")); // HTML は生のまま返す（DMM 等が HTML からパースするため）
    }

    #[test]
    fn test_get_body_for_parse_empty_plain_uses_html() {
        let row = EmailRow {
            email_id: 1,
            message_id: "m1".to_string(),
            body_plain: Some("".to_string()),
            body_html: Some("<div>内容</div>".to_string()),
            from_address: None,
            subject: None,
            internal_date: None,
        };
        let body = get_body_for_parse(&row);
        assert!(body.contains("内容"));
        assert!(body.contains("<div>")); // HTML は生のまま返す
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
    fn test_get_parser_dmm_confirm() {
        let parser = get_parser("dmm_confirm");
        assert!(parser.is_some());
    }

    #[test]
    fn test_get_parser_dmm_split_complete() {
        let parser = get_parser("dmm_split_complete");
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
            image_url: None,
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
    fn test_order_item_serialization() {
        let item = OrderItem {
            name: "ガンプラ HG".to_string(),
            manufacturer: Some("バンダイ".to_string()),
            model_number: Some("BAN-001".to_string()),
            unit_price: 2500,
            quantity: 1,
            subtotal: 2500,
            image_url: None,
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
                image_url: None,
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
