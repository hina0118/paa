//! 駿河屋マーケットプレイス マイページ HTML パースタスク
//!
//! `htmls` テーブルに保存済みの HTML を読み込み、注文情報をパースして DB に保存する。
//! WebView（ログイン）不要で何度でも再実行できる。

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::batch_runner::BatchTask;
use crate::plugins::surugaya_mp::html_parser;
use crate::repository::SqliteOrderRepository;

pub const SURUGAYA_HTML_PARSE_TASK_NAME: &str = "駿河屋HTMLパース";
pub const SURUGAYA_HTML_PARSE_EVENT_NAME: &str = "batch-progress";

// ─────────────────────────────────────────────────────────────────────────────
// 入出力・コンテキスト型
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SurugayaHtmlParseInput {
    pub html_id: i64,
    pub url: String,
    pub html_content: String,
}

pub struct SurugayaHtmlParseOutput {
    pub html_id: i64,
    pub trade_code: String,
}

pub struct SurugayaHtmlParseContext {
    pub pool: Arc<SqlitePool>,
}

// ─────────────────────────────────────────────────────────────────────────────
// BatchTask 実装
// ─────────────────────────────────────────────────────────────────────────────

pub struct SurugayaHtmlParseTask;

#[async_trait]
impl BatchTask for SurugayaHtmlParseTask {
    type Input = SurugayaHtmlParseInput;
    type Output = SurugayaHtmlParseOutput;
    type Context = SurugayaHtmlParseContext;

    fn name(&self) -> &str {
        SURUGAYA_HTML_PARSE_TASK_NAME
    }

    fn event_name(&self) -> &str {
        SURUGAYA_HTML_PARSE_EVENT_NAME
    }

    async fn process(
        &self,
        input: Self::Input,
        ctx: &Self::Context,
    ) -> Result<Self::Output, String> {
        let mut tx = ctx
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin tx: {e}"))?;

        if html_parser::is_cancelled_order(&input.html_content) {
            let trade_code = extract_trade_code_from_url(&input.url)
                .ok_or_else(|| format!("Cannot extract trade_code from URL: {}", input.url))?;
            apply_cancelled_order(&mut tx, &trade_code).await?;
            tx.commit()
                .await
                .map_err(|e| format!("Failed to commit: {e}"))?;
            return Ok(SurugayaHtmlParseOutput {
                html_id: input.html_id,
                trade_code,
            });
        }

        let mypage_info = html_parser::parse_mypage_html(&input.html_content)?;

        SqliteOrderRepository::save_order_in_tx(
            &mut tx,
            &mypage_info.order_info,
            None,
            Some("suruga-ya.jp".to_string()),
            None,
        )
        .await?;

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit: {e}"))?;

        Ok(SurugayaHtmlParseOutput {
            html_id: input.html_id,
            trade_code: mypage_info.trade_code,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// キャンセル処理
// ─────────────────────────────────────────────────────────────────────────────

async fn apply_cancelled_order(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    trade_code: &str,
) -> Result<(), String> {
    let order: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM orders WHERE order_number = ? AND shop_domain = 'suruga-ya.jp' LIMIT 1",
    )
    .bind(trade_code)
    .fetch_optional(tx.as_mut())
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    let Some((order_id,)) = order else {
        log::warn!(
            "[surugaya_html_parse] キャンセル済み注文が未登録: trade_code={}",
            trade_code
        );
        return Ok(());
    };

    sqlx::query("DELETE FROM items WHERE order_id = ?")
        .bind(order_id)
        .execute(tx.as_mut())
        .await
        .map_err(|e| format!("Failed to delete items: {e}"))?;

    sqlx::query(
        r#"
        UPDATE deliveries
        SET delivery_status = 'cancelled', updated_at = CURRENT_TIMESTAMP
        WHERE order_id = ?
        "#,
    )
    .bind(order_id)
    .execute(tx.as_mut())
    .await
    .map_err(|e| format!("Failed to update deliveries: {e}"))?;

    log::info!(
        "[surugaya_html_parse] キャンセル適用: trade_code={} order_id={}",
        trade_code,
        order_id
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// URL ヘルパー
// ─────────────────────────────────────────────────────────────────────────────

/// `https://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=M...`
/// から `trade_code` の値を取得する
fn extract_trade_code_from_url(url: &str) -> Option<String> {
    url.split('?')
        .nth(1)?
        .split('&')
        .find_map(|kv| {
            let (key, val) = kv.split_once('=')?;
            if key == "trade_code" {
                Some(val.to_string())
            } else {
                None
            }
        })
}
