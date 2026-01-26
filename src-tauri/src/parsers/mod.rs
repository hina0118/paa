use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::{Arc, Mutex};
use tauri::Emitter;

// 型エイリアス：パース対象メールの情報
type EmailRow = (i64, String, String, Option<String>, Option<String>);

// ホビーサーチ用の共通パースユーティリティ関数
mod hobbysearch_common;

// ホビーサーチ用パーサー
pub mod hobbysearch_change;
pub mod hobbysearch_confirm;
pub mod hobbysearch_confirm_yoyaku;
pub mod hobbysearch_send;

/// パース状態管理
#[derive(Clone)]
pub struct ParseState {
    pub should_cancel: Arc<Mutex<bool>>,
    pub is_running: Arc<Mutex<bool>>,
}

impl ParseState {
    pub fn new() -> Self {
        Self {
            should_cancel: Arc::new(Mutex::new(false)),
            is_running: Arc::new(Mutex::new(false)),
        }
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

/// パーサータイプから適切なパーサーを取得する
pub fn get_parser(parser_type: &str) -> Option<Box<dyn EmailParser>> {
    match parser_type {
        "hobbysearch_confirm" => Some(Box::new(hobbysearch_confirm::HobbySearchConfirmParser)),
        "hobbysearch_confirm_yoyaku" => Some(Box::new(
            hobbysearch_confirm_yoyaku::HobbySearchConfirmYoyakuParser,
        )),
        "hobbysearch_change" => Some(Box::new(hobbysearch_change::HobbySearchChangeParser)),
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

/// パース進捗イベント
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// パース結果をデータベースに保存する
pub async fn save_order_to_db(
    pool: &SqlitePool,
    order_info: &OrderInfo,
    email_id: Option<i64>,
    shop_domain: Option<&str>,
) -> Result<i64, String> {
    // トランザクション開始
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    // 1. 既存の注文を検索（同じorder_numberとshop_domainの組み合わせ）
    let existing_order: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT id FROM orders
        WHERE order_number = ? AND shop_domain = ?
        LIMIT 1
        "#,
    )
    .bind(&order_info.order_number)
    .bind(shop_domain)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Failed to check existing order: {}", e))?;

    let order_id = if let Some((existing_id,)) = existing_order {
        // 既存の注文が見つかった場合、そのIDを使用
        log::debug!("Found existing order with id: {}", existing_id);
        existing_id
    } else {
        // 新規注文を作成
        let new_order_id = sqlx::query(
            r#"
            INSERT INTO orders (order_number, order_date, shop_domain)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&order_info.order_number)
        .bind(&order_info.order_date)
        .bind(shop_domain)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to insert order: {}", e))?
        .last_insert_rowid();

        log::debug!("Created new order with id: {}", new_order_id);
        new_order_id
    };

    // 2. 既存注文の場合は注文日を更新（より新しい情報で更新）
    if existing_order.is_some() && order_info.order_date.is_some() {
        sqlx::query(
            r#"
            UPDATE orders
            SET order_date = COALESCE(?, order_date)
            WHERE id = ?
            "#,
        )
        .bind(&order_info.order_date)
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to update order date: {}", e))?;

        log::debug!("Updated order {} with new date info", order_id);
    }

    // 3. itemsテーブルに商品を保存（重複チェック付き）
    for item in &order_info.items {
        // 同じitem_nameとbrandの商品が既に存在するかチェック
        let existing_item: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT id FROM items
            WHERE order_id = ? AND item_name = ? AND COALESCE(brand, '') = COALESCE(?, '')
            LIMIT 1
            "#,
        )
        .bind(order_id)
        .bind(&item.name)
        .bind(&item.manufacturer)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Failed to check existing item: {}", e))?;

        if existing_item.is_none() {
            // 新しい商品を追加
            sqlx::query(
                r#"
                INSERT INTO items (order_id, item_name, brand, price, quantity)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(order_id)
            .bind(&item.name)
            .bind(&item.manufacturer)
            .bind(item.unit_price)
            .bind(item.quantity)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert item: {}", e))?;

            log::debug!("Added new item '{}' to order {}", item.name, order_id);
        } else {
            log::debug!("Item '{}' already exists for order {}", item.name, order_id);
        }
    }

    // 4. deliveriesテーブルに配送情報を保存（重複チェック・更新付き）
    if let Some(delivery_info) = &order_info.delivery_info {
        // 同じtracking_numberの配送情報が既に存在するかチェック
        let existing_delivery: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT id FROM deliveries
            WHERE order_id = ? AND tracking_number = ?
            LIMIT 1
            "#,
        )
        .bind(order_id)
        .bind(&delivery_info.tracking_number)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Failed to check existing delivery: {}", e))?;

        if existing_delivery.is_none() {
            // 新しい配送情報を追加
            sqlx::query(
                r#"
                INSERT INTO deliveries (order_id, tracking_number, carrier, delivery_status)
                VALUES (?, ?, ?, 'shipped')
                "#,
            )
            .bind(order_id)
            .bind(&delivery_info.tracking_number)
            .bind(&delivery_info.carrier)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert delivery: {}", e))?;

            log::debug!("Added new delivery info for order {}", order_id);
        } else {
            // 既存の配送情報を更新（より詳細な情報で上書き）
            sqlx::query(
                r#"
                UPDATE deliveries
                SET carrier = COALESCE(?, carrier),
                    delivery_status = 'shipped'
                WHERE order_id = ? AND tracking_number = ?
                "#,
            )
            .bind(&delivery_info.carrier)
            .bind(order_id)
            .bind(&delivery_info.tracking_number)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to update delivery: {}", e))?;

            log::debug!("Updated delivery info for order {}", order_id);
        }
    }

    // 5. order_emailsテーブルにメールとの関連を保存（重複チェック）
    if let Some(email_id) = email_id {
        // 既に同じ関連が存在するかチェック
        let existing_link: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT order_id FROM order_emails
            WHERE order_id = ? AND email_id = ?
            LIMIT 1
            "#,
        )
        .bind(order_id)
        .bind(email_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Failed to check existing order_email link: {}", e))?;

        if existing_link.is_none() {
            // 新しい関連を作成
            sqlx::query(
                r#"
                INSERT INTO order_emails (order_id, email_id)
                VALUES (?, ?)
                "#,
            )
            .bind(order_id)
            .bind(email_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to link order to email: {}", e))?;

            log::debug!("Linked order {} to email {}", order_id, email_id);
        } else {
            log::debug!("Order {} is already linked to email {}", order_id, email_id);
        }
    }

    // トランザクションをコミット
    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    Ok(order_id)
}

/// バッチパース処理
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
    sqlx::query(
        "UPDATE parse_metadata SET parse_status = 'running', last_parse_started_at = ?1 WHERE id = 1"
    )
    .bind(Utc::now().to_rfc3339())
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update parse status: {}", e))?;

    // order_emails, deliveries, items, orders テーブルをクリア（パースやり直しのため）
    // 外部キー制約により、order_emails -> deliveries -> items -> orders の順でクリア
    // NOTE: ユーザーには事前にUI（Parse画面）で警告と確認ダイアログを表示済み
    log::info!("Clearing order_emails, deliveries, items, and orders tables for fresh parse...");

    sqlx::query("DELETE FROM order_emails")
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to clear order_emails table: {}", e))?;

    sqlx::query("DELETE FROM deliveries")
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to clear deliveries table: {}", e))?;

    sqlx::query("DELETE FROM items")
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to clear items table: {}", e))?;

    sqlx::query("DELETE FROM orders")
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to clear orders table: {}", e))?;

    // shop_settingsから有効な店舗とパーサータイプ、件名フィルターを取得
    let shop_settings: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT sender_address, parser_type, subject_filters FROM shop_settings WHERE is_enabled = 1"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to fetch shop settings: {}", e))?;

    if shop_settings.is_empty() {
        log::warn!("No enabled shop settings found");
        return Err("No enabled shop settings found".to_string());
    }

    // パース対象の全メール数を取得
    let total_email_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM emails
        WHERE body_plain IS NOT NULL
        AND from_address IS NOT NULL
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to count emails: {}", e))?;

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
            let cancel_event = ParseProgressEvent {
                batch_number: iteration,
                total_emails: total_email_count as usize,
                parsed_count: overall_parsed_count,
                success_count: overall_success_count,
                failed_count: overall_failed_count,
                status_message: "パースがキャンセルされました".to_string(),
                is_complete: true,
                error: Some("Cancelled by user".to_string()),
            };
            let _ = app_handle.emit("parse-progress", cancel_event);

            // ステータスをidleに戻す
            let _ = sqlx::query("UPDATE parse_metadata SET parse_status = 'idle' WHERE id = 1")
                .execute(pool)
                .await;

            return Ok(());
        }

        iteration += 1;

        // パース対象のメールを取得（既にパース済みのものを除外）
        // order_emailsテーブルにemail_idが存在しないメールのみ取得
        // メール送信日時（internal_date）の古い順（ASC）でパースすることで、時系列に沿って注文情報が更新される
        let emails: Vec<EmailRow> = sqlx::query_as(
            r#"
            SELECT e.id, e.message_id, e.body_plain, e.from_address, e.subject
            FROM emails e
            LEFT JOIN order_emails oe ON e.id = oe.email_id
            WHERE e.body_plain IS NOT NULL
            AND e.from_address IS NOT NULL
            AND oe.email_id IS NULL
            ORDER BY e.internal_date ASC
            LIMIT ?
            "#,
        )
        .bind(batch_size as i64)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to fetch emails: {}", e))?;

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

        for (email_id, _message_id, body_plain, from_address_opt, subject_opt) in emails.iter() {
            let from_address = match from_address_opt {
                Some(addr) => addr,
                None => {
                    failed_count += 1;
                    continue;
                }
            };

            // 送信元アドレスと件名フィルターから候補のパーサータイプを全て取得
            let candidate_parsers: Vec<&str> = shop_settings
                .iter()
                .filter_map(|(addr, parser_type, subject_filters_json)| {
                    // 送信元アドレスが一致するか確認
                    if !from_address.contains(addr) {
                        return None;
                    }

                    // 件名フィルターがない場合は、アドレス一致だけでOK
                    let Some(filters_json) = subject_filters_json else {
                        return Some(parser_type.as_str());
                    };

                    // 件名フィルターがある場合は、件名も確認
                    let Ok(filters) = serde_json::from_str::<Vec<String>>(filters_json) else {
                        return Some(parser_type.as_str()); // JSONパースエラー時はフィルター無視
                    };

                    // 件名がない場合は除外
                    let Some(subject) = subject_opt else {
                        return None;
                    };

                    // いずれかのフィルターに一致すればOK
                    if filters.iter().any(|filter| subject.contains(filter)) {
                        Some(parser_type.as_str())
                    } else {
                        None
                    }
                })
                .collect();

            if candidate_parsers.is_empty() {
                log::debug!(
                    "No parser found for address: {} with subject: {:?}",
                    from_address,
                    subject_opt
                );
                failed_count += 1;
                continue;
            }

            // 複数のパーサーを順番に試す（最初に成功したものを使用）
            let mut parse_result: Option<Result<OrderInfo, String>> = None;
            let mut last_error = String::new();

            for parser_type in &candidate_parsers {
                let parser = match get_parser(parser_type) {
                    Some(p) => p,
                    None => {
                        log::warn!("Unknown parser type: {}", parser_type);
                        continue;
                    }
                };

                match parser.parse(body_plain) {
                    Ok(order_info) => {
                        log::debug!("Successfully parsed with parser: {}", parser_type);
                        parse_result = Some(Ok(order_info));
                        break;
                    }
                    Err(e) => {
                        log::debug!("Parser {} failed: {}", parser_type, e);
                        last_error = e;
                        // 次のパーサーを試す
                        continue;
                    }
                }
            }

            let parse_result = match parse_result {
                Some(result) => result,
                None => {
                    log::error!(
                        "All parsers failed for email {}. Last error: {}",
                        email_id,
                        last_error
                    );
                    Err(last_error)
                }
            };

            match parse_result {
                Ok(order_info) => {
                    // ドメインを抽出
                    let shop_domain = from_address.split('@').nth(1);

                    // データベースに保存
                    match save_order_to_db(pool, &order_info, Some(*email_id), shop_domain).await {
                        Ok(order_id) => {
                            log::info!("Successfully parsed and saved order: {}", order_id);
                            success_count += 1;
                        }
                        Err(e) => {
                            log::error!("Failed to save order: {}", e);
                            failed_count += 1;
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to parse email {}: {}", email_id, e);
                    failed_count += 1;
                }
            }

            overall_parsed_count += 1;
        }

        // バッチ処理完了後に進捗イベントを送信（バッチごとに1回）
        overall_success_count += success_count;
        overall_failed_count += failed_count;

        let progress = ParseProgressEvent {
            batch_number: iteration,
            total_emails: total_email_count as usize,
            parsed_count: overall_parsed_count,
            success_count: overall_success_count,
            failed_count: overall_failed_count,
            status_message: format!(
                "パース中... ({}/{})",
                overall_parsed_count, total_email_count
            ),
            is_complete: false,
            error: None,
        };

        let _ = app_handle.emit("parse-progress", progress);

        log::info!(
            "Iteration {} completed: success={}, failed={}",
            iteration,
            success_count,
            failed_count
        );
    }

    // 完了イベントを送信
    let final_progress = ParseProgressEvent {
        batch_number: iteration,
        total_emails: total_email_count as usize,
        parsed_count: overall_parsed_count,
        success_count: overall_success_count,
        failed_count: overall_failed_count,
        status_message: format!(
            "パース完了: 成功 {}, 失敗 {}",
            overall_success_count, overall_failed_count
        ),
        is_complete: true,
        error: None,
    };

    let _ = app_handle.emit("parse-progress", final_progress);

    // メタデータを更新
    sqlx::query(
        r#"
        UPDATE parse_metadata
        SET parse_status = 'completed',
            last_parse_completed_at = ?1,
            total_parsed_count = ?2
        WHERE id = 1
        "#,
    )
    .bind(Utc::now().to_rfc3339())
    .bind(overall_success_count as i64)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to update parse metadata: {}", e))?;

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
}
