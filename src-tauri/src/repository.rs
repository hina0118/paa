//! リポジトリパターンによるDB操作の抽象化
//!
//! このモジュールはデータベース操作を抽象化し、テスト時にモック可能にします。

use crate::gmail::{
    CreateShopSettings, GmailMessage, ShopSettings, SyncMetadata, UpdateShopSettings,
};
use crate::parsers::{EmailRow, OrderInfo, ParseMetadata as ParserParseMetadata};
use async_trait::async_trait;
use chrono::Utc;
#[cfg(test)]
use mockall::automock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

/// parse_skipped に保存する前にエラーメッセージをサニタイズ（パス・接続文字列等をマスク）
fn sanitize_error_for_parse_skipped(msg: &str) -> String {
    const MAX_LEN: usize = 500;
    let mut s = msg.chars().take(MAX_LEN).collect::<String>();
    if msg.chars().count() > MAX_LEN {
        s.push_str("...");
    }
    // パスや接続文字列をマスク（テーブルビューアで機密情報が露出しないよう）
    let patterns = [
        (r"(?i)[A-Za-z]:\\[^\s]*", "[PATH]"), // Windows: C:\...
        (r"sqlite:file:[^\s]*", "[DB_PATH]"), // sqlite:file:...
        // Unix: 代表的な絶対パスのみマスク（/home, /var, /etc, /usr 等。スペースが \ でエスケープされている場合も含む）
        (
            r#"/(?:home|var|etc|usr|opt|tmp|root|srv|mnt|media|run)(?:/[^\s"']+)+"#,
            "[PATH]",
        ),
    ];
    for (pat, repl) in patterns {
        if let Ok(re) = Regex::new(pat) {
            s = re.replace_all(&s, repl).into_owned();
        }
    }
    s
}

/// ウィンドウ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    pub width: i64,
    pub height: i64,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub maximized: bool,
}

/// メール統計情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailStats {
    pub total_emails: i64,
    pub with_body_plain: i64,
    pub with_body_html: i64,
    pub without_body: i64,
    pub avg_plain_length: f64,
    pub avg_html_length: f64,
}

/// メール関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait EmailRepository: Send + Sync {
    /// メッセージをDBに保存
    ///
    /// # Returns
    /// `(saved, skipped)` - saved: rows_affected>0（INSERT または ON CONFLICT DO UPDATE）、
    /// skipped: rows_affected=0。
    ///
    /// # セマンティクス（ON CONFLICT DO UPDATE 使用時）
    /// 重複（既存 message_id）の場合も UPDATE が実行されるため、saved にカウントされる。
    /// skipped は rows_affected=0 のときのみ（SQLite の changes() は UPDATE でマッチ行数を返すため、通常は 0）。
    /// ログ／メトリクスで saved を「新規のみ」と解釈しないこと。
    async fn save_messages(&self, messages: &[GmailMessage]) -> Result<(usize, usize), String>;

    /// 既存のメッセージIDを取得
    async fn get_existing_message_ids(&self) -> Result<Vec<String>, String>;

    /// メッセージ数を取得
    async fn get_message_count(&self) -> Result<i64, String>;

    /// 同期メタデータを取得
    async fn get_sync_metadata(&self) -> Result<SyncMetadata, String>;

    /// 同期メタデータを更新
    /// ライフタイム問題を回避するためにOption<String>を使用
    async fn update_sync_metadata(
        &self,
        oldest_date: Option<String>,
        total_synced: i64,
        status: &str,
    ) -> Result<(), String>;

    /// 同期開始日時を更新
    async fn update_sync_started_at(&self) -> Result<(), String>;

    /// 同期完了日時を更新
    async fn update_sync_completed_at(&self) -> Result<(), String>;

    /// 同期ステータスのみを更新
    async fn update_sync_status(&self, status: &str) -> Result<(), String>;

    /// エラーステータスに更新（last_sync_completed_atも更新）
    async fn update_sync_error_status(&self) -> Result<(), String>;

    /// 同期開始（ステータスと開始日時をアトミックに更新）
    async fn start_sync(&self) -> Result<(), String>;

    /// 同期完了（ステータスと完了日時をアトミックに更新）
    async fn complete_sync(&self, status: &str) -> Result<(), String>;
}

/// 同期メタデータ専用のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait SyncMetadataRepository: Send + Sync {
    /// 同期メタデータを取得
    async fn get_sync_metadata(&self) -> Result<SyncMetadata, String>;

    /// バッチサイズを更新
    async fn update_batch_size(&self, batch_size: i64) -> Result<(), String>;

    /// 最大イテレーション回数を更新
    async fn update_max_iterations(&self, max_iterations: i64) -> Result<(), String>;

    /// 同期ステータスをidleにリセット（syncingのときのみ）
    async fn reset_sync_status(&self) -> Result<(), String>;

    /// oldest_fetched_dateをNULLにリセット
    async fn reset_sync_date(&self) -> Result<(), String>;

    /// エラーステータスとエラーメッセージを更新
    async fn update_error_status(&self, error_message: &str) -> Result<(), String>;
}

/// パースメタデータ専用のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait ParseMetadataRepository: Send + Sync {
    /// パースメタデータを取得
    async fn get_parse_metadata(&self) -> Result<ParserParseMetadata, String>;

    /// バッチサイズを取得
    async fn get_batch_size(&self) -> Result<i64, String>;

    /// バッチサイズを更新
    async fn update_batch_size(&self, batch_size: i64) -> Result<(), String>;

    /// パースステータスと各種メタ情報を更新
    async fn update_parse_status(
        &self,
        status: &str,
        started_at: Option<String>,
        completed_at: Option<String>,
        total_parsed: Option<i64>,
        error_message: Option<String>,
    ) -> Result<(), String>;

    /// パースステータスをidleにリセット
    async fn reset_parse_status(&self) -> Result<(), String>;
}

/// ウィンドウ設定関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait WindowSettingsRepository: Send + Sync {
    /// ウィンドウ設定を取得
    async fn get_window_settings(&self) -> Result<WindowSettings, String>;

    /// ウィンドウ設定を保存
    async fn save_window_settings(&self, settings: WindowSettings) -> Result<(), String>;
}

/// メール統計関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait EmailStatsRepository: Send + Sync {
    /// メール統計情報を取得
    async fn get_email_stats(&self) -> Result<EmailStats, String>;
}

/// 注文関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait OrderRepository: Send + Sync {
    /// 注文情報を保存（orders, items, deliveries, order_emailsテーブル）
    async fn save_order(
        &self,
        order_info: &OrderInfo,
        email_id: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
    ) -> Result<i64, String>;
}

/// パース関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait ParseRepository: Send + Sync {
    /// 未パースのメールを取得（order_emails・parse_skippedに存在しないメール）
    async fn get_unparsed_emails(&self, batch_size: usize) -> Result<Vec<EmailRow>, String>;

    /// パース失敗したメールを記録（無限ループ防止）
    async fn mark_parse_skipped(&self, email_id: i64, error_message: &str) -> Result<(), String>;

    /// 注文関連テーブルをクリア（order_emails, parse_skipped, deliveries, items, orders）
    async fn clear_order_tables(&self) -> Result<(), String>;

    /// パース対象の全メール数を取得
    async fn get_total_email_count(&self) -> Result<i64, String>;
}

/// ショップ設定関連のDB操作を抽象化するトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait ShopSettingsRepository: Send + Sync {
    /// 全ショップ設定を取得
    async fn get_all(&self) -> Result<Vec<ShopSettings>, String>;

    /// 有効なショップ設定のみを取得（ORDER BY shop_name, id で返す。parsers が試行順序に依存）
    async fn get_enabled(&self) -> Result<Vec<ShopSettings>, String>;

    /// ショップ設定を作成
    async fn create(&self, settings: CreateShopSettings) -> Result<ShopSettings, String>;

    /// ショップ設定を更新
    async fn update(&self, id: i64, settings: UpdateShopSettings) -> Result<ShopSettings, String>;

    /// ショップ設定を削除
    async fn delete(&self, id: i64) -> Result<(), String>;
}

/// SQLiteを使用したEmailRepositoryの実装
pub struct SqliteEmailRepository {
    pool: SqlitePool,
}

impl SqliteEmailRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

/// SQLiteを使用したSyncMetadataRepositoryの実装
pub struct SqliteSyncMetadataRepository {
    pool: SqlitePool,
}

impl SqliteSyncMetadataRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SyncMetadataRepository for SqliteSyncMetadataRepository {
    async fn get_sync_metadata(&self) -> Result<SyncMetadata, String> {
        let row: (
            String,
            Option<String>,
            i64,
            i64,
            Option<String>,
            Option<String>,
            i64,
        ) = sqlx::query_as(
            r#"
                SELECT
                    sync_status,
                    oldest_fetched_date,
                    total_synced_count,
                    batch_size,
                    last_sync_started_at,
                    last_sync_completed_at,
                    max_iterations
                FROM sync_metadata
                WHERE id = 1
                "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to get sync metadata: {e}"))?;

        Ok(SyncMetadata {
            sync_status: row.0,
            oldest_fetched_date: row.1,
            total_synced_count: row.2,
            batch_size: row.3,
            last_sync_started_at: row.4,
            last_sync_completed_at: row.5,
            max_iterations: row.6,
        })
    }

    async fn update_batch_size(&self, batch_size: i64) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE sync_metadata
            SET batch_size = ?
            WHERE id = 1
            "#,
        )
        .bind(batch_size)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update batch size: {e}"))?;

        Ok(())
    }

    async fn update_max_iterations(&self, max_iterations: i64) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE sync_metadata
            SET max_iterations = ?
            WHERE id = 1
            "#,
        )
        .bind(max_iterations)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update max iterations: {e}"))?;

        Ok(())
    }

    async fn reset_sync_status(&self) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE sync_metadata
            SET sync_status = 'idle'
            WHERE id = 1 AND sync_status = 'syncing'
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to reset sync status: {e}"))?;

        Ok(())
    }

    async fn reset_sync_date(&self) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE sync_metadata
            SET oldest_fetched_date = NULL
            WHERE id = 1
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to reset sync date: {e}"))?;

        Ok(())
    }

    async fn update_error_status(&self, error_message: &str) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE sync_metadata
            SET sync_status = 'error',
                last_error_message = ?,
                last_sync_completed_at = ?
            WHERE id = 1
            "#,
        )
        .bind(error_message)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update error status: {e}"))?;

        Ok(())
    }
}

/// SQLiteを使用したParseMetadataRepositoryの実装
pub struct SqliteParseMetadataRepository {
    pool: SqlitePool,
}

impl SqliteParseMetadataRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ParseMetadataRepository for SqliteParseMetadataRepository {
    async fn get_parse_metadata(&self) -> Result<ParserParseMetadata, String> {
        let row: (
            String,
            Option<String>,
            Option<String>,
            i64,
            Option<String>,
            i64,
        ) = sqlx::query_as(
            r#"
                SELECT
                    parse_status,
                    last_parse_started_at,
                    last_parse_completed_at,
                    total_parsed_count,
                    last_error_message,
                    batch_size
                FROM parse_metadata
                WHERE id = 1
                "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to get parse metadata: {e}"))?;

        Ok(ParserParseMetadata {
            parse_status: row.0,
            last_parse_started_at: row.1,
            last_parse_completed_at: row.2,
            total_parsed_count: row.3,
            last_error_message: row.4,
            batch_size: row.5,
        })
    }

    async fn get_batch_size(&self) -> Result<i64, String> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT batch_size
            FROM parse_metadata
            WHERE id = 1
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to get parse batch size: {e}"))?;

        Ok(row.0)
    }

    async fn update_batch_size(&self, batch_size: i64) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE parse_metadata
            SET batch_size = ?
            WHERE id = 1
            "#,
        )
        .bind(batch_size)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update parse batch size: {e}"))?;

        Ok(())
    }

    async fn update_parse_status(
        &self,
        status: &str,
        started_at: Option<String>,
        completed_at: Option<String>,
        total_parsed: Option<i64>,
        error_message: Option<String>,
    ) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE parse_metadata
            SET parse_status = ?,
                last_parse_started_at = COALESCE(?, last_parse_started_at),
                last_parse_completed_at = COALESCE(?, last_parse_completed_at),
                total_parsed_count = COALESCE(?, total_parsed_count),
                last_error_message = COALESCE(?, last_error_message)
            WHERE id = 1
            "#,
        )
        .bind(status)
        .bind(started_at)
        .bind(completed_at)
        .bind(total_parsed)
        .bind(error_message)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update parse status: {e}"))?;

        Ok(())
    }

    async fn reset_parse_status(&self) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE parse_metadata
            SET parse_status = 'idle',
                last_error_message = NULL
            WHERE id = 1
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to reset parse status: {e}"))?;

        Ok(())
    }
}

/// SQLiteを使用したWindowSettingsRepositoryの実装
pub struct SqliteWindowSettingsRepository {
    pool: SqlitePool,
}

impl SqliteWindowSettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WindowSettingsRepository for SqliteWindowSettingsRepository {
    async fn get_window_settings(&self) -> Result<WindowSettings, String> {
        let row: (i64, i64, Option<i64>, Option<i64>, i64) = sqlx::query_as(
            r#"
            SELECT width, height, x, y, maximized
            FROM window_settings
            WHERE id = 1
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to get window settings: {e}"))?;

        Ok(WindowSettings {
            width: row.0,
            height: row.1,
            x: row.2,
            y: row.3,
            maximized: row.4 != 0,
        })
    }

    async fn save_window_settings(&self, settings: WindowSettings) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE window_settings
            SET width = ?,
                height = ?,
                x = ?,
                y = ?,
                maximized = ?
            WHERE id = 1
            "#,
        )
        .bind(settings.width)
        .bind(settings.height)
        .bind(settings.x)
        .bind(settings.y)
        .bind(i32::from(settings.maximized))
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to save window settings: {e}"))?;

        Ok(())
    }
}

/// SQLiteを使用したEmailStatsRepositoryの実装
pub struct SqliteEmailStatsRepository {
    pool: SqlitePool,
}

impl SqliteEmailStatsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EmailStatsRepository for SqliteEmailStatsRepository {
    async fn get_email_stats(&self) -> Result<EmailStats, String> {
        let stats: (i64, i64, i64, i64, Option<f64>, Option<f64>) = sqlx::query_as(
            r#"
            WITH email_lengths AS (
                SELECT
                    body_plain,
                    body_html,
                    CASE WHEN body_plain IS NOT NULL THEN LENGTH(body_plain) ELSE 0 END AS plain_length,
                    CASE WHEN body_html IS NOT NULL THEN LENGTH(body_html) ELSE 0 END AS html_length
                FROM emails
            )
            SELECT
                COUNT(*) AS total,
                COUNT(CASE WHEN body_plain IS NOT NULL AND plain_length > 0 THEN 1 END) AS with_plain,
                COUNT(CASE WHEN body_html IS NOT NULL AND html_length > 0 THEN 1 END) AS with_html,
                COUNT(CASE WHEN (body_plain IS NULL OR plain_length = 0) AND (body_html IS NULL OR html_length = 0) THEN 1 END) AS without_body,
                AVG(CASE WHEN body_plain IS NOT NULL AND plain_length > 0 THEN plain_length END) AS avg_plain,
                AVG(CASE WHEN body_html IS NOT NULL AND html_length > 0 THEN html_length END) AS avg_html
            FROM email_lengths
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch email stats: {e}"))?;

        Ok(EmailStats {
            total_emails: stats.0,
            with_body_plain: stats.1,
            with_body_html: stats.2,
            without_body: stats.3,
            avg_plain_length: stats.4.unwrap_or(0.0),
            avg_html_length: stats.5.unwrap_or(0.0),
        })
    }
}

/// SQLiteを使用したOrderRepositoryの実装
pub struct SqliteOrderRepository {
    pool: SqlitePool,
}

impl SqliteOrderRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrderRepository for SqliteOrderRepository {
    async fn save_order(
        &self,
        order_info: &OrderInfo,
        email_id: Option<i64>,
        shop_domain: Option<String>,
        shop_name: Option<String>,
    ) -> Result<i64, String> {
        // トランザクション開始
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to start transaction: {e}"))?;

        // 1. 既存の注文を検索（同じorder_numberとshop_domainの組み合わせ）
        let existing_order: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT id FROM orders
            WHERE order_number = ? AND shop_domain = ?
            LIMIT 1
            "#,
        )
        .bind(&order_info.order_number)
        .bind(shop_domain.as_deref())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Failed to check existing order: {e}"))?;

        let order_id = if let Some((existing_id,)) = existing_order {
            // 既存の注文が見つかった場合、そのIDを使用
            log::debug!("Found existing order with id: {}", existing_id);
            existing_id
        } else {
            // 新規注文を作成
            let new_order_id = sqlx::query(
                r#"
                INSERT INTO orders (order_number, order_date, shop_domain, shop_name)
                VALUES (?, ?, ?, ?)
                "#,
            )
            .bind(&order_info.order_number)
            .bind(&order_info.order_date)
            .bind(shop_domain.as_deref())
            .bind(shop_name.as_deref())
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert order: {e}"))?
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
            .map_err(|e| format!("Failed to update order date: {e}"))?;

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
            .map_err(|e| format!("Failed to check existing item: {e}"))?;

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
                .map_err(|e| format!("Failed to insert item: {e}"))?;

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
            .map_err(|e| format!("Failed to check existing delivery: {e}"))?;

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
                .map_err(|e| format!("Failed to insert delivery: {e}"))?;

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
                .map_err(|e| format!("Failed to update delivery: {e}"))?;

                log::debug!("Updated delivery info for order {}", order_id);
            }
        }

        // 5. order_emailsテーブルにメールとの関連を保存（重複チェック）
        if let Some(email_id_val) = email_id {
            // 既に同じ関連が存在するかチェック
            let existing_link: Option<(i64,)> = sqlx::query_as(
                r#"
                SELECT order_id FROM order_emails
                WHERE order_id = ? AND email_id = ?
                LIMIT 1
                "#,
            )
            .bind(order_id)
            .bind(email_id_val)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("Failed to check existing order_email link: {e}"))?;

            if existing_link.is_none() {
                // 新しい関連を作成
                sqlx::query(
                    r#"
                    INSERT INTO order_emails (order_id, email_id)
                    VALUES (?, ?)
                    "#,
                )
                .bind(order_id)
                .bind(email_id_val)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Failed to link order to email: {e}"))?;

                log::debug!("Linked order {} to email {}", order_id, email_id_val);
            } else {
                log::debug!(
                    "Order {} is already linked to email {}",
                    order_id,
                    email_id_val
                );
            }
        }

        // トランザクションをコミット
        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {e}"))?;

        Ok(order_id)
    }
}

/// SQLiteを使用したParseRepositoryの実装
pub struct SqliteParseRepository {
    pool: SqlitePool,
}

impl SqliteParseRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ParseRepository for SqliteParseRepository {
    async fn get_unparsed_emails(&self, batch_size: usize) -> Result<Vec<EmailRow>, String> {
        let emails: Vec<EmailRow> = sqlx::query_as(
            r#"
            SELECT e.id, e.message_id, e.body_plain, e.from_address, e.subject, e.internal_date
            FROM emails e
            LEFT JOIN order_emails oe ON e.id = oe.email_id
            LEFT JOIN parse_skipped ps ON e.id = ps.email_id
            WHERE e.body_plain IS NOT NULL
            AND e.from_address IS NOT NULL
            AND oe.email_id IS NULL
            AND ps.email_id IS NULL
            ORDER BY e.internal_date ASC
            LIMIT ?
            "#,
        )
        .bind(batch_size as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to fetch unparsed emails: {e}"))?;

        Ok(emails)
    }

    async fn mark_parse_skipped(&self, email_id: i64, error_message: &str) -> Result<(), String> {
        let sanitized = sanitize_error_for_parse_skipped(error_message);
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO parse_skipped (email_id, error_message)
            VALUES (?, ?)
            "#,
        )
        .bind(email_id)
        .bind(&sanitized)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to mark parse skipped: {e}"))?;
        Ok(())
    }

    async fn clear_order_tables(&self) -> Result<(), String> {
        // トランザクション内で全てのDELETE操作を実行してアトミック性を確保
        // 外部キー制約により、order_emails -> deliveries -> items -> orders の順でクリア
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin transaction: {e}"))?;

        sqlx::query("DELETE FROM order_emails")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to clear order_emails table: {e}"))?;

        sqlx::query("DELETE FROM parse_skipped")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to clear parse_skipped table: {e}"))?;

        sqlx::query("DELETE FROM deliveries")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to clear deliveries table: {e}"))?;

        sqlx::query("DELETE FROM items")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to clear items table: {e}"))?;

        sqlx::query("DELETE FROM orders")
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to clear orders table: {e}"))?;

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {e}"))?;

        Ok(())
    }

    async fn get_total_email_count(&self) -> Result<i64, String> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM emails
            WHERE body_plain IS NOT NULL
            AND from_address IS NOT NULL
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to count emails: {e}"))?;

        Ok(count)
    }
}

#[async_trait]
impl EmailRepository for SqliteEmailRepository {
    async fn save_messages(&self, messages: &[GmailMessage]) -> Result<(usize, usize), String> {
        let mut saved = 0;
        let mut skipped = 0;

        // rows_affected の解釈: SQLite の changes() は「UPDATE でマッチした行数」ではなく
        // 「直近のステートメントによって実際に変更された行数」を返す。
        // このクエリでは ON CONFLICT DO UPDATE が発生した場合、既存行に対して UPDATE が実行され、
        // COALESCE により既存値がそのまま再代入されるケースでも、その行は更新されたものとしてカウントされるため
        // rows_affected は 1 となる想定である。
        // saved は「新規のみ」ではなく「INSERT/UPDATE が行われた件数」。FetchResult/SyncProgressEvent の
        // saved_count/newly_saved に伝播する。
        // 参考: https://www.sqlite.org/c3ref/changes.html

        // トランザクションを使用してバッチ処理
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin transaction: {e}"))?;

        for message in messages {
            // ON CONFLICT で既存の場合は body を補完（初回同期時に body_html 等が取れなかった場合の再取得で更新）
            let result = sqlx::query(
                r#"
                INSERT INTO emails (message_id, body_plain, body_html, internal_date, from_address, subject)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(message_id) DO UPDATE SET
                    body_plain = COALESCE(excluded.body_plain, body_plain),
                    body_html = COALESCE(excluded.body_html, body_html),
                    internal_date = COALESCE(excluded.internal_date, internal_date),
                    from_address = COALESCE(excluded.from_address, from_address),
                    subject = COALESCE(excluded.subject, subject)
                "#,
            )
            .bind(&message.message_id)
            .bind(&message.body_plain)
            .bind(&message.body_html)
            .bind(message.internal_date)
            .bind(&message.from_address)
            .bind(&message.subject)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to insert message {}: {}", message.message_id, e))?;

            if result.rows_affected() > 0 {
                saved += 1;
            } else {
                skipped += 1;
            }
        }

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit transaction: {e}"))?;

        Ok((saved, skipped))
    }

    async fn get_existing_message_ids(&self) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT message_id FROM emails")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("Failed to get existing message IDs: {e}"))?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    async fn get_message_count(&self) -> Result<i64, String> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM emails")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| format!("Failed to get message count: {e}"))?;

        Ok(count.0)
    }

    async fn get_sync_metadata(&self) -> Result<SyncMetadata, String> {
        let row: (
            String,
            Option<String>,
            i64,
            i64,
            Option<String>,
            Option<String>,
            i64,
        ) = sqlx::query_as(
            r#"
                SELECT
                    sync_status,
                    oldest_fetched_date,
                    total_synced_count,
                    batch_size,
                    last_sync_started_at,
                    last_sync_completed_at,
                    max_iterations
                FROM sync_metadata
                WHERE id = 1
                "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to get sync metadata: {e}"))?;

        Ok(SyncMetadata {
            sync_status: row.0,
            oldest_fetched_date: row.1,
            total_synced_count: row.2,
            batch_size: row.3,
            last_sync_started_at: row.4,
            last_sync_completed_at: row.5,
            max_iterations: row.6,
        })
    }

    async fn update_sync_metadata(
        &self,
        oldest_date: Option<String>,
        total_synced: i64,
        status: &str,
    ) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE sync_metadata
            SET oldest_fetched_date = COALESCE(?, oldest_fetched_date),
                total_synced_count = ?,
                sync_status = ?
            WHERE id = 1
            "#,
        )
        .bind(oldest_date)
        .bind(total_synced)
        .bind(status)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update sync metadata: {e}"))?;

        Ok(())
    }

    async fn update_sync_started_at(&self) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE sync_metadata
            SET last_sync_started_at = ?
            WHERE id = 1
            "#,
        )
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update sync started at: {e}"))?;

        Ok(())
    }

    async fn update_sync_completed_at(&self) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE sync_metadata
            SET last_sync_completed_at = ?
            WHERE id = 1
            "#,
        )
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update sync completed at: {e}"))?;

        Ok(())
    }

    async fn update_sync_status(&self, status: &str) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE sync_metadata
            SET sync_status = ?
            WHERE id = 1
            "#,
        )
        .bind(status)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update sync status: {e}"))?;

        Ok(())
    }

    async fn update_sync_error_status(&self) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE sync_metadata
            SET sync_status = 'error', last_sync_completed_at = ?
            WHERE id = 1
            "#,
        )
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update error status: {e}"))?;

        Ok(())
    }

    async fn start_sync(&self) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE sync_metadata
            SET sync_status = 'syncing', last_sync_started_at = ?
            WHERE id = 1
            "#,
        )
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to start sync: {e}"))?;

        Ok(())
    }

    async fn complete_sync(&self, status: &str) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE sync_metadata
            SET sync_status = ?, last_sync_completed_at = ?
            WHERE id = 1
            "#,
        )
        .bind(status)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to complete sync: {e}"))?;

        Ok(())
    }
}

/// SQLiteを使用したShopSettingsRepositoryの実装
pub struct SqliteShopSettingsRepository {
    pool: SqlitePool,
}

impl SqliteShopSettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ShopSettingsRepository for SqliteShopSettingsRepository {
    async fn get_all(&self) -> Result<Vec<ShopSettings>, String> {
        let settings = sqlx::query_as::<_, ShopSettings>(
            r#"
            SELECT id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at
            FROM shop_settings
            ORDER BY shop_name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get shop settings: {e}"))?;

        Ok(settings)
    }

    async fn get_enabled(&self) -> Result<Vec<ShopSettings>, String> {
        let settings = sqlx::query_as::<_, ShopSettings>(
            r#"
            SELECT id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at
            FROM shop_settings
            WHERE is_enabled = 1
            -- バッチ処理（例: batch_parse_emails）のパーサ試行順序を shop_name, id で一意に決めているため、この並び順は変更しないこと
            ORDER BY shop_name, id
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("Failed to get enabled shop settings: {e}"))?;

        Ok(settings)
    }

    async fn create(&self, settings: CreateShopSettings) -> Result<ShopSettings, String> {
        let subject_filters_json = settings
            .subject_filters
            .map(|filters| serde_json::to_string(&filters))
            .transpose()
            .map_err(|e| format!("Failed to serialize subject filters: {e}"))?;

        let result = sqlx::query(
            r#"
            INSERT INTO shop_settings (shop_name, sender_address, parser_type, subject_filters, is_enabled)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&settings.shop_name)
        .bind(&settings.sender_address)
        .bind(&settings.parser_type)
        .bind(&subject_filters_json)
        .bind(1) // 新規作成時は有効化しておく（DBデフォルトには依存しない）
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to create shop setting: {e}"))?;

        // last_insert_rowid()を使用して作成したレコードを取得
        let inserted_id = result.last_insert_rowid();
        let created = sqlx::query_as::<_, ShopSettings>(
            r#"
            SELECT id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at
            FROM shop_settings
            WHERE id = ?
            "#,
        )
        .bind(inserted_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to get created shop setting: {e}"))?;

        Ok(created)
    }

    async fn update(&self, id: i64, settings: UpdateShopSettings) -> Result<ShopSettings, String> {
        // 現在の設定を取得
        let current = sqlx::query_as::<_, ShopSettings>(
            "SELECT id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at FROM shop_settings WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to get shop setting: {e}"))?
        .ok_or_else(|| format!("Shop setting with id {id} not found"))?;

        let subject_filters_json = settings
            .subject_filters
            .map(|filters| serde_json::to_string(&filters))
            .transpose()
            .map_err(|e| format!("Failed to serialize subject filters: {e}"))?;

        // COALESCEは不要（unwrap_or/orで既にフォールバック済み）
        sqlx::query(
            r#"
            UPDATE shop_settings
            SET shop_name = ?,
                sender_address = ?,
                parser_type = ?,
                is_enabled = ?,
                subject_filters = ?,
                updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(settings.shop_name.as_ref().unwrap_or(&current.shop_name))
        .bind(
            settings
                .sender_address
                .as_ref()
                .unwrap_or(&current.sender_address),
        )
        .bind(
            settings
                .parser_type
                .as_ref()
                .unwrap_or(&current.parser_type),
        )
        .bind(settings.is_enabled.unwrap_or(current.is_enabled))
        .bind(
            subject_filters_json
                .as_ref()
                .or(current.subject_filters.as_ref()),
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update shop setting: {e}"))?;

        // 更新後のレコードを取得
        let updated = sqlx::query_as::<_, ShopSettings>(
            "SELECT id, shop_name, sender_address, parser_type, is_enabled, subject_filters, created_at, updated_at FROM shop_settings WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("Failed to get updated shop setting: {e}"))?;

        Ok(updated)
    }

    async fn delete(&self, id: i64) -> Result<(), String> {
        sqlx::query("DELETE FROM shop_settings WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("Failed to delete shop setting: {e}"))?;

        Ok(())
    }
}

// =============================================================================
// ProductMasterRepository - Gemini AI による商品名解析結果のキャッシュ
// =============================================================================

use crate::gemini::ParsedProduct;

/// ProductMaster エンティティ
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProductMaster {
    pub id: i64,
    pub raw_name: String,
    pub normalized_name: String,
    pub maker: Option<String>,
    pub series: Option<String>,
    pub product_name: Option<String>,
    pub scale: Option<String>,
    pub is_reissue: bool,
    pub platform_hint: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ProductMaster> for ParsedProduct {
    fn from(pm: ProductMaster) -> Self {
        ParsedProduct {
            maker: pm.maker,
            series: pm.series,
            name: pm.product_name.unwrap_or_default(),
            scale: pm.scale,
            is_reissue: pm.is_reissue,
        }
    }
}

/// ProductMaster リポジトリトレイト
#[cfg_attr(test, automock)]
#[async_trait]
pub trait ProductMasterRepository: Send + Sync {
    /// raw_name でキャッシュ検索
    async fn find_by_raw_name(&self, raw_name: &str) -> Result<Option<ProductMaster>, String>;

    /// normalized_name でキャッシュ検索（類似検索用）
    async fn find_by_normalized_name(
        &self,
        normalized_name: &str,
    ) -> Result<Option<ProductMaster>, String>;

    /// 複数の raw_name で一括キャッシュ検索（N+1クエリ回避用）
    async fn find_by_raw_names(
        &self,
        raw_names: &[String],
    ) -> Result<std::collections::HashMap<String, ProductMaster>, String>;

    /// 複数の normalized_name で一括キャッシュ検索（N+1クエリ回避用）
    async fn find_by_normalized_names(
        &self,
        normalized_names: &[String],
    ) -> Result<std::collections::HashMap<String, ProductMaster>, String>;

    /// 新規保存
    /// Note: platform_hintはOption<String>を使用（mockallとの互換性のため）
    async fn save(
        &self,
        raw_name: &str,
        normalized_name: &str,
        parsed: &ParsedProduct,
        platform_hint: Option<String>,
    ) -> Result<i64, String>;

    /// 更新
    async fn update(&self, id: i64, parsed: &ParsedProduct) -> Result<(), String>;
}

/// SQLiteを使用したProductMasterRepositoryの実装
pub struct SqliteProductMasterRepository {
    pool: SqlitePool,
}

impl SqliteProductMasterRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProductMasterRepository for SqliteProductMasterRepository {
    async fn find_by_raw_name(&self, raw_name: &str) -> Result<Option<ProductMaster>, String> {
        sqlx::query_as::<_, ProductMaster>(
            r#"
            SELECT
                id,
                raw_name,
                normalized_name,
                maker,
                series,
                product_name,
                scale,
                is_reissue,
                platform_hint,
                created_at,
                updated_at
            FROM product_master
            WHERE raw_name = ?
            LIMIT 1
            "#,
        )
        .bind(raw_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to find product master by raw_name: {e}"))
    }

    async fn find_by_normalized_name(
        &self,
        normalized_name: &str,
    ) -> Result<Option<ProductMaster>, String> {
        sqlx::query_as::<_, ProductMaster>(
            r#"
            SELECT
                id,
                raw_name,
                normalized_name,
                maker,
                series,
                product_name,
                scale,
                is_reissue,
                platform_hint,
                created_at,
                updated_at
            FROM product_master
            WHERE normalized_name = ?
            LIMIT 1
            "#,
        )
        .bind(normalized_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("Failed to find product master by normalized_name: {e}"))
    }

    async fn find_by_raw_names(
        &self,
        raw_names: &[String],
    ) -> Result<std::collections::HashMap<String, ProductMaster>, String> {
        if raw_names.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        // SQLite にはバインド変数の上限（デフォルト999）があるため、チャンクで分割
        const MAX_PARAMS_PER_QUERY: usize = 900;
        let mut all_rows: Vec<ProductMaster> = Vec::new();

        for chunk in raw_names.chunks(MAX_PARAMS_PER_QUERY) {
            let placeholders = chunk
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                r#"
                SELECT
                    id,
                    raw_name,
                    normalized_name,
                    maker,
                    series,
                    product_name,
                    scale,
                    is_reissue,
                    platform_hint,
                    created_at,
                    updated_at
                FROM product_master
                WHERE raw_name IN ({})
                "#,
                placeholders
            );
            let mut query = sqlx::query_as::<_, ProductMaster>(&sql);
            for name in chunk {
                query = query.bind(name);
            }
            let rows = query
                .fetch_all(&self.pool)
                .await
                .map_err(|e| format!("Failed to find product masters by raw_names: {e}"))?;
            all_rows.extend(rows);
        }
        Ok(all_rows
            .into_iter()
            .map(|r| (r.raw_name.clone(), r))
            .collect())
    }

    async fn find_by_normalized_names(
        &self,
        normalized_names: &[String],
    ) -> Result<std::collections::HashMap<String, ProductMaster>, String> {
        if normalized_names.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        // SQLite にはバインド変数の上限（デフォルト999）があるため、チャンクで分割
        const MAX_PARAMS_PER_QUERY: usize = 900;
        let mut all_rows: Vec<ProductMaster> = Vec::new();

        for chunk in normalized_names.chunks(MAX_PARAMS_PER_QUERY) {
            let placeholders = chunk
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                r#"
                SELECT
                    id,
                    raw_name,
                    normalized_name,
                    maker,
                    series,
                    product_name,
                    scale,
                    is_reissue,
                    platform_hint,
                    created_at,
                    updated_at
                FROM product_master
                WHERE normalized_name IN ({})
                "#,
                placeholders
            );
            let mut query = sqlx::query_as::<_, ProductMaster>(&sql);
            for name in chunk {
                query = query.bind(name);
            }
            let rows = query
                .fetch_all(&self.pool)
                .await
                .map_err(|e| format!("Failed to find product masters by normalized_names: {e}"))?;
            all_rows.extend(rows);
        }
        Ok(all_rows
            .into_iter()
            .map(|r| (r.normalized_name.clone(), r))
            .collect())
    }

    async fn save(
        &self,
        raw_name: &str,
        normalized_name: &str,
        parsed: &ParsedProduct,
        platform_hint: Option<String>,
    ) -> Result<i64, String> {
        // Avoid logging user/product data (raw_name, maker, series, name); keep logs metrics-only.
        log::debug!("Saving product_master entry");

        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO product_master (
                raw_name,
                normalized_name,
                maker,
                series,
                product_name,
                scale,
                is_reissue,
                platform_hint
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(raw_name) DO UPDATE SET
                maker = excluded.maker,
                series = excluded.series,
                product_name = excluded.product_name,
                scale = excluded.scale,
                is_reissue = excluded.is_reissue,
                platform_hint = COALESCE(product_master.platform_hint, excluded.platform_hint)
            RETURNING id
            "#,
        )
        .bind(raw_name)
        .bind(normalized_name)
        .bind(&parsed.maker)
        .bind(&parsed.series)
        .bind(&parsed.name)
        .bind(&parsed.scale)
        .bind(parsed.is_reissue)
        .bind(&platform_hint)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            log::error!("Failed to save product master: {}", e);
            format!("Failed to save product master: {e}")
        })?;

        log::debug!("Successfully saved to product_master");
        Ok(id)
    }

    async fn update(&self, id: i64, parsed: &ParsedProduct) -> Result<(), String> {
        sqlx::query(
            r#"
            UPDATE product_master
            SET
                maker = ?,
                series = ?,
                product_name = ?,
                scale = ?,
                is_reissue = ?
            WHERE id = ?
            "#,
        )
        .bind(&parsed.maker)
        .bind(&parsed.series)
        .bind(&parsed.name)
        .bind(&parsed.scale)
        .bind(parsed.is_reissue)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to update product master: {e}"))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn test_sanitize_error_for_parse_skipped() {
        assert_eq!(
            sanitize_error_for_parse_skipped("Order number not found"),
            "Order number not found"
        );
        assert!(
            sanitize_error_for_parse_skipped("Failed: C:\\Users\\john\\AppData\\paa_data.db")
                .contains("[PATH]")
        );
        assert!(
            sanitize_error_for_parse_skipped("sqlite:file:/path/to/db.db").contains("[DB_PATH]")
        );
        assert!(
            sanitize_error_for_parse_skipped("error: /home/user/.config/paa/file")
                .contains("[PATH]")
        );
        // /root, /etc, /usr/local 等の絶対パスもマスクされる
        assert!(sanitize_error_for_parse_skipped("error: /root/.ssh/id_rsa").contains("[PATH]"));
        assert!(sanitize_error_for_parse_skipped("error: /etc/passwd").contains("[PATH]"));
        assert!(sanitize_error_for_parse_skipped("error: /usr/local/bin/app").contains("[PATH]"));
    }

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // テーブル作成（migrationsと同等の定義）

        // emails テーブル (002, 011, 017, 019 に対応)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS emails (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id TEXT UNIQUE NOT NULL,
                body_plain TEXT,
                body_html TEXT,
                analysis_status TEXT NOT NULL DEFAULT 'pending' CHECK(analysis_status IN ('pending', 'completed')),
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                internal_date INTEGER,
                from_address TEXT,
                subject TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create emails table");

        // sync_metadata テーブル (010, 012 に対応)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sync_metadata (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                oldest_fetched_date TEXT,
                sync_status TEXT NOT NULL DEFAULT 'idle' CHECK(sync_status IN ('idle', 'syncing', 'paused', 'error')),
                total_synced_count INTEGER NOT NULL DEFAULT 0,
                batch_size INTEGER NOT NULL DEFAULT 50,
                last_sync_started_at TEXT,
                last_sync_completed_at TEXT,
                last_error_message TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now')),
                max_iterations INTEGER NOT NULL DEFAULT 1000
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create sync_metadata table");

        sqlx::query("INSERT INTO sync_metadata (id, sync_status) VALUES (1, 'idle')")
            .execute(&pool)
            .await
            .expect("Failed to insert default metadata");

        // parse_metadata テーブル (016, 018 に対応)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS parse_metadata (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                parse_status TEXT NOT NULL DEFAULT 'idle' CHECK(parse_status IN ('idle', 'running', 'completed', 'error')),
                last_parse_started_at DATETIME,
                last_parse_completed_at DATETIME,
                total_parsed_count INTEGER NOT NULL DEFAULT 0,
                last_error_message TEXT,
                batch_size INTEGER NOT NULL DEFAULT 100
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create parse_metadata table");

        sqlx::query("INSERT INTO parse_metadata (id, parse_status) VALUES (1, 'idle')")
            .execute(&pool)
            .await
            .expect("Failed to insert default parse metadata");

        // window_settings テーブル (013 に対応)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS window_settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                width INTEGER NOT NULL DEFAULT 800,
                height INTEGER NOT NULL DEFAULT 600,
                x INTEGER,
                y INTEGER,
                maximized INTEGER NOT NULL DEFAULT 0 CHECK(maximized IN (0, 1)),
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create window_settings table");

        sqlx::query(
            "INSERT OR IGNORE INTO window_settings (id, width, height) VALUES (1, 800, 600)",
        )
        .execute(&pool)
        .await
        .expect("Failed to insert default window settings");

        // orders テーブル (003 に対応、shop_name 含む)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS orders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                shop_domain TEXT,
                shop_name TEXT,
                order_number TEXT,
                order_date DATETIME,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create orders table");

        // items テーブル (004 に対応)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL,
                item_name TEXT NOT NULL,
                item_name_normalized TEXT,
                price INTEGER NOT NULL DEFAULT 0,
                quantity INTEGER NOT NULL DEFAULT 1,
                category TEXT,
                brand TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create items table");

        // deliveries テーブル (006 に対応)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS deliveries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL,
                tracking_number TEXT,
                carrier TEXT,
                delivery_status TEXT NOT NULL DEFAULT 'not_shipped' CHECK(delivery_status IN ('not_shipped', 'preparing', 'shipped', 'in_transit', 'out_for_delivery', 'delivered', 'failed', 'returned', 'cancelled')),
                estimated_delivery DATETIME,
                actual_delivery DATETIME,
                last_checked_at DATETIME,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create deliveries table");

        // order_emails テーブル (008 に対応)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS order_emails (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                order_id INTEGER NOT NULL,
                email_id INTEGER NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
                FOREIGN KEY (email_id) REFERENCES emails(id) ON DELETE CASCADE,
                UNIQUE (order_id, email_id)
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create order_emails table");

        // parse_skipped テーブル
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS parse_skipped (
                email_id INTEGER PRIMARY KEY,
                error_message TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (email_id) REFERENCES emails(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create parse_skipped table");

        // shop_settings テーブル (014, 015 に対応)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS shop_settings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                shop_name TEXT NOT NULL,
                sender_address TEXT NOT NULL,
                parser_type TEXT NOT NULL,
                is_enabled INTEGER NOT NULL DEFAULT 1 CHECK(is_enabled IN (0, 1)),
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                subject_filters TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create shop_settings table");

        pool
    }

    #[tokio::test]
    async fn test_email_repository_save_and_get() {
        let pool = setup_test_db().await;
        let repo = SqliteEmailRepository::new(pool);

        let messages = vec![
            GmailMessage {
                message_id: "test123".to_string(),
                snippet: "Test snippet".to_string(),
                subject: Some("Test subject".to_string()),
                body_plain: Some("Plain body".to_string()),
                body_html: Some("<p>HTML body</p>".to_string()),
                internal_date: 1704067200000,
                from_address: Some("test@example.com".to_string()),
            },
            GmailMessage {
                message_id: "test456".to_string(),
                snippet: "Another snippet".to_string(),
                subject: Some("Another subject".to_string()),
                body_plain: None,
                body_html: None,
                internal_date: 1704153600000,
                from_address: None,
            },
        ];

        // 保存
        let (saved, skipped) = repo.save_messages(&messages).await.unwrap();
        assert_eq!(saved, 2);
        assert_eq!(skipped, 0);

        // 重複保存: ON CONFLICT DO UPDATE により UPDATE が実行され、saved としてカウントされる
        let (saved, skipped) = repo.save_messages(&messages).await.unwrap();
        assert_eq!(saved, 2);
        assert_eq!(skipped, 0);

        // カウント確認
        let count = repo.get_message_count().await.unwrap();
        assert_eq!(count, 2);

        // 既存ID取得
        let ids = repo.get_existing_message_ids().await.unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"test123".to_string()));
        assert!(ids.contains(&"test456".to_string()));
    }

    #[tokio::test]
    async fn test_sync_metadata_operations() {
        let pool = setup_test_db().await;
        let repo = SqliteEmailRepository::new(pool);

        // 初期値確認
        let metadata = repo.get_sync_metadata().await.unwrap();
        assert_eq!(metadata.sync_status, "idle");
        assert_eq!(metadata.total_synced_count, 0);

        // 更新
        repo.update_sync_metadata(Some("2024-01-01".to_string()), 100, "syncing")
            .await
            .unwrap();

        let metadata = repo.get_sync_metadata().await.unwrap();
        assert_eq!(metadata.sync_status, "syncing");
        assert_eq!(metadata.total_synced_count, 100);
        assert_eq!(metadata.oldest_fetched_date, Some("2024-01-01".to_string()));
    }

    #[tokio::test]
    async fn test_sync_metadata_repository_get_and_update_batch_size() {
        let pool = setup_test_db().await;
        let repo = SqliteSyncMetadataRepository::new(pool.clone());

        // 初期値確認
        let metadata = repo.get_sync_metadata().await.unwrap();
        assert_eq!(metadata.sync_status, "idle");
        assert_eq!(metadata.batch_size, 50);

        // バッチサイズ更新
        repo.update_batch_size(100).await.unwrap();

        // 更新結果を検証
        let metadata = repo.get_sync_metadata().await.unwrap();
        assert_eq!(metadata.batch_size, 100);
    }

    #[tokio::test]
    async fn test_sync_metadata_repository_update_max_iterations() {
        let pool = setup_test_db().await;
        let repo = SqliteSyncMetadataRepository::new(pool.clone());

        // 初期値確認
        let metadata = repo.get_sync_metadata().await.unwrap();
        assert_eq!(metadata.max_iterations, 1000);

        // max_iterations 更新
        repo.update_max_iterations(5000).await.unwrap();

        // 更新結果を検証
        let metadata = repo.get_sync_metadata().await.unwrap();
        assert_eq!(metadata.max_iterations, 5000);
    }

    #[tokio::test]
    async fn test_sync_metadata_repository_reset_sync_status_only_when_syncing() {
        let pool = setup_test_db().await;
        let repo = SqliteSyncMetadataRepository::new(pool.clone());

        // idle のときに reset しても変化しない
        let metadata = repo.get_sync_metadata().await.unwrap();
        assert_eq!(metadata.sync_status, "idle");

        repo.reset_sync_status().await.unwrap();
        let metadata = repo.get_sync_metadata().await.unwrap();
        assert_eq!(metadata.sync_status, "idle");

        // syncing にしてから reset すると idle に戻る
        sqlx::query("UPDATE sync_metadata SET sync_status = 'syncing' WHERE id = 1")
            .execute(&pool)
            .await
            .expect("failed to set syncing status");

        let metadata = repo.get_sync_metadata().await.unwrap();
        assert_eq!(metadata.sync_status, "syncing");

        repo.reset_sync_status().await.unwrap();
        let metadata = repo.get_sync_metadata().await.unwrap();
        assert_eq!(metadata.sync_status, "idle");
    }

    #[tokio::test]
    async fn test_sync_metadata_repository_reset_sync_date() {
        let pool = setup_test_db().await;
        let repo = SqliteSyncMetadataRepository::new(pool.clone());

        // 日付をセット
        sqlx::query("UPDATE sync_metadata SET oldest_fetched_date = '2024-01-01' WHERE id = 1")
            .execute(&pool)
            .await
            .expect("failed to set oldest_fetched_date");

        let metadata = repo.get_sync_metadata().await.unwrap();
        assert_eq!(metadata.oldest_fetched_date, Some("2024-01-01".to_string()));

        // リセットで NULL になる
        repo.reset_sync_date().await.unwrap();
        let metadata = repo.get_sync_metadata().await.unwrap();
        assert_eq!(metadata.oldest_fetched_date, None);
    }

    #[tokio::test]
    async fn test_sync_metadata_repository_update_error_status() {
        let pool = setup_test_db().await;
        let repo = SqliteSyncMetadataRepository::new(pool.clone());

        let error_message = "Test error for repository";
        repo.update_error_status(error_message).await.unwrap();

        let row: (String, Option<String>, Option<String>) = sqlx::query_as(
            r#"
            SELECT sync_status, last_error_message, last_sync_completed_at
            FROM sync_metadata
            WHERE id = 1
            "#,
        )
        .fetch_one(&pool)
        .await
        .expect("failed to fetch sync_metadata row");

        assert_eq!(row.0, "error");
        assert_eq!(row.1, Some(error_message.to_string()));
        assert!(row.2.is_some(), "last_sync_completed_at should be set");
    }

    #[tokio::test]
    async fn test_window_settings_repository_get_and_save() {
        let pool = setup_test_db().await;
        let repo = SqliteWindowSettingsRepository::new(pool.clone());

        // 初期値確認
        let settings = repo.get_window_settings().await.unwrap();
        assert_eq!(settings.width, 800);
        assert_eq!(settings.height, 600);
        assert_eq!(settings.x, None);
        assert_eq!(settings.y, None);
        assert!(!settings.maximized);

        // 設定を更新
        let new_settings = WindowSettings {
            width: 1024,
            height: 768,
            x: Some(100),
            y: Some(200),
            maximized: true,
        };
        repo.save_window_settings(new_settings.clone())
            .await
            .unwrap();

        // 更新結果を検証
        let settings = repo.get_window_settings().await.unwrap();
        assert_eq!(settings.width, 1024);
        assert_eq!(settings.height, 768);
        assert_eq!(settings.x, Some(100));
        assert_eq!(settings.y, Some(200));
        assert!(settings.maximized);
    }

    #[tokio::test]
    async fn test_email_stats_repository_get_stats() {
        let pool = setup_test_db().await;
        let repo = SqliteEmailStatsRepository::new(pool.clone());

        // 空の状態での統計
        let stats = repo.get_email_stats().await.unwrap();
        assert_eq!(stats.total_emails, 0);
        assert_eq!(stats.with_body_plain, 0);
        assert_eq!(stats.with_body_html, 0);
        assert_eq!(stats.without_body, 0);
        assert_eq!(stats.avg_plain_length, 0.0);
        assert_eq!(stats.avg_html_length, 0.0);

        // テストデータを追加
        sqlx::query(
            r#"
            INSERT INTO emails (message_id, body_plain, body_html, from_address, subject)
            VALUES
                ('msg1', 'Plain text body', NULL, 'test1@example.com', 'Subject 1'),
                ('msg2', NULL, '<p>HTML body</p>', 'test2@example.com', 'Subject 2'),
                ('msg3', 'Another plain', '<p>Another HTML</p>', 'test3@example.com', 'Subject 3'),
                ('msg4', NULL, NULL, 'test4@example.com', 'Subject 4')
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to insert test emails");

        // 統計を再取得
        let stats = repo.get_email_stats().await.unwrap();
        assert_eq!(stats.total_emails, 4);
        assert_eq!(stats.with_body_plain, 2); // msg1, msg3
        assert_eq!(stats.with_body_html, 2); // msg2, msg3
        assert_eq!(stats.without_body, 1); // msg4
        assert!(stats.avg_plain_length > 0.0);
        assert!(stats.avg_html_length > 0.0);
    }

    #[tokio::test]
    async fn test_order_repository_save_new_order() {
        let pool = setup_test_db().await;
        let repo = SqliteOrderRepository::new(pool.clone());

        // テスト用のメールを追加
        sqlx::query("INSERT INTO emails (message_id, body_plain, from_address, subject) VALUES ('test-email-1', 'body', 'test@example.com', 'Subject')")
            .execute(&pool)
            .await
            .expect("Failed to insert test email");

        let email_id: (i64,) =
            sqlx::query_as("SELECT id FROM emails WHERE message_id = 'test-email-1'")
                .fetch_one(&pool)
                .await
                .expect("Failed to get email id");

        // 新しい注文情報を作成
        use crate::parsers::{DeliveryInfo, OrderInfo, OrderItem};
        let order_info = OrderInfo {
            order_number: "ORD-001".to_string(),
            order_date: Some("2024-01-01".to_string()),
            delivery_address: None,
            delivery_info: Some(DeliveryInfo {
                carrier: "ヤマト運輸".to_string(),
                tracking_number: "1234567890".to_string(),
                delivery_date: None,
                delivery_time: None,
                carrier_url: None,
            }),
            items: vec![
                OrderItem {
                    name: "商品A".to_string(),
                    manufacturer: Some("メーカーA".to_string()),
                    model_number: None,
                    unit_price: 1000,
                    quantity: 2,
                    subtotal: 2000,
                },
                OrderItem {
                    name: "商品B".to_string(),
                    manufacturer: None,
                    model_number: None,
                    unit_price: 500,
                    quantity: 1,
                    subtotal: 500,
                },
            ],
            subtotal: Some(2500),
            shipping_fee: Some(500),
            total_amount: Some(3000),
        };

        // 注文を保存
        let order_id = repo
            .save_order(
                &order_info,
                Some(email_id.0),
                Some("example.com".to_string()),
                Some("Test Shop".to_string()),
            )
            .await
            .unwrap();

        // 検証: ordersテーブル
        let order: (String, Option<String>, Option<String>, Option<String>) = sqlx::query_as(
            "SELECT order_number, order_date, shop_domain, shop_name FROM orders WHERE id = ?",
        )
        .bind(order_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch order");
        assert_eq!(order.0, "ORD-001");
        assert_eq!(order.1, Some("2024-01-01".to_string()));
        assert_eq!(order.2, Some("example.com".to_string()));
        assert_eq!(order.3, Some("Test Shop".to_string()));

        // 検証: itemsテーブル
        let items: Vec<(String, Option<String>, i64, i64)> = sqlx::query_as(
            "SELECT item_name, brand, price, quantity FROM items WHERE order_id = ? ORDER BY item_name",
        )
        .bind(order_id)
        .fetch_all(&pool)
        .await
        .expect("Failed to fetch items");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].0, "商品A");
        assert_eq!(items[0].1, Some("メーカーA".to_string()));
        assert_eq!(items[0].2, 1000);
        assert_eq!(items[0].3, 2);

        // 検証: deliveriesテーブル
        let delivery: (String, String, String) = sqlx::query_as(
            "SELECT tracking_number, carrier, delivery_status FROM deliveries WHERE order_id = ?",
        )
        .bind(order_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch delivery");
        assert_eq!(delivery.0, "1234567890");
        assert_eq!(delivery.1, "ヤマト運輸");
        assert_eq!(delivery.2, "shipped");

        // 検証: order_emailsテーブル
        let link: (i64, i64) = sqlx::query_as(
            "SELECT order_id, email_id FROM order_emails WHERE order_id = ? AND email_id = ?",
        )
        .bind(order_id)
        .bind(email_id.0)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch order_email link");
        assert_eq!(link.0, order_id);
        assert_eq!(link.1, email_id.0);
    }

    #[tokio::test]
    async fn test_parse_repository_get_unparsed_emails_and_clear() {
        let pool = setup_test_db().await;
        let repo = SqliteParseRepository::new(pool.clone());

        // テスト用のメールを追加
        sqlx::query(
            r#"
            INSERT INTO emails (message_id, body_plain, from_address, subject, internal_date)
            VALUES
                ('email1', 'body1', 'test1@example.com', 'Subject 1', 1000),
                ('email2', 'body2', 'test2@example.com', 'Subject 2', 2000),
                ('email3', 'body3', 'test3@example.com', 'Subject 3', 3000)
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to insert test emails");

        // 未パースのメールを取得
        let emails = repo.get_unparsed_emails(10).await.unwrap();
        assert_eq!(emails.len(), 3);

        // 注文を作成してemail1をパース済みにする
        sqlx::query(
            "INSERT INTO orders (order_number, shop_domain) VALUES ('ORD-001', 'example.com')",
        )
        .execute(&pool)
        .await
        .expect("Failed to insert order");

        let order_id: (i64,) =
            sqlx::query_as("SELECT id FROM orders WHERE order_number = 'ORD-001'")
                .fetch_one(&pool)
                .await
                .expect("Failed to get order id");

        let email_id: (i64,) = sqlx::query_as("SELECT id FROM emails WHERE message_id = 'email1'")
            .fetch_one(&pool)
            .await
            .expect("Failed to get email id");

        sqlx::query("INSERT INTO order_emails (order_id, email_id) VALUES (?, ?)")
            .bind(order_id.0)
            .bind(email_id.0)
            .execute(&pool)
            .await
            .expect("Failed to link order to email");

        // 未パースのメールを再取得（email1は除外される）
        let emails = repo.get_unparsed_emails(10).await.unwrap();
        assert_eq!(emails.len(), 2);

        // 全メール数を取得
        let total = repo.get_total_email_count().await.unwrap();
        assert_eq!(total, 3);

        // テーブルをクリア
        repo.clear_order_tables().await.unwrap();

        // クリア後、未パースのメールは再び3件になる
        let emails = repo.get_unparsed_emails(10).await.unwrap();
        assert_eq!(emails.len(), 3);
    }

    #[tokio::test]
    async fn test_parse_metadata_repository_get_and_update_batch_size() {
        let pool = setup_test_db().await;
        let repo = SqliteParseMetadataRepository::new(pool.clone());

        // 初期値確認
        let metadata = repo.get_parse_metadata().await.unwrap();
        assert_eq!(metadata.parse_status, "idle");
        assert_eq!(metadata.total_parsed_count, 0);
        assert_eq!(metadata.batch_size, 100);

        // バッチサイズ更新
        repo.update_batch_size(200).await.unwrap();

        // 更新結果を検証
        let metadata = repo.get_parse_metadata().await.unwrap();
        assert_eq!(metadata.batch_size, 200);
    }

    #[tokio::test]
    async fn test_parse_metadata_repository_update_and_reset_status() {
        let pool = setup_test_db().await;
        let repo = SqliteParseMetadataRepository::new(pool.clone());

        // ステータス更新
        repo.update_parse_status(
            "running",
            Some("2024-01-01T10:00:00Z".to_string()),
            None,
            Some(10),
            None,
        )
        .await
        .unwrap();

        let metadata = repo.get_parse_metadata().await.unwrap();
        assert_eq!(metadata.parse_status, "running");
        assert_eq!(metadata.total_parsed_count, 10);
        assert!(metadata.last_parse_started_at.is_some());

        // リセットでidleに戻る
        repo.reset_parse_status().await.unwrap();
        let metadata = repo.get_parse_metadata().await.unwrap();
        assert_eq!(metadata.parse_status, "idle");
    }

    #[tokio::test]
    async fn test_shop_settings_repository_crud() {
        let pool = setup_test_db().await;
        let repo = SqliteShopSettingsRepository::new(pool);

        // 作成
        let settings = CreateShopSettings {
            shop_name: "Test Shop".to_string(),
            sender_address: "shop@example.com".to_string(),
            parser_type: "hobbysearch_confirm".to_string(),
            subject_filters: Some(vec!["注文確認".to_string()]),
        };

        let created = repo.create(settings).await.unwrap();
        assert_eq!(created.shop_name, "Test Shop");
        assert!(created.is_enabled);

        // 全件取得
        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 1);

        // 更新
        let update = UpdateShopSettings {
            shop_name: Some("Updated Shop".to_string()),
            sender_address: None,
            parser_type: None,
            is_enabled: Some(false),
            subject_filters: None,
        };

        let updated = repo.update(created.id, update).await.unwrap();
        assert_eq!(updated.shop_name, "Updated Shop");
        assert!(!updated.is_enabled);

        // 有効なもののみ取得
        let enabled = repo.get_enabled().await.unwrap();
        assert_eq!(enabled.len(), 0);

        // 削除
        repo.delete(created.id).await.unwrap();
        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 0);
    }
}
