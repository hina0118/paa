//! Amazon 注文詳細 HTML パースタスク
//!
//! `htmls` テーブルに保存済みの HTML を読み込み、注文情報をパースして DB に保存する。
//! WebView（ログイン）不要で何度でも再実行できる。

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::batch_runner::BatchTask;
use crate::plugins::amazon::html_parser;
use crate::repository::SqliteOrderRepository;

pub const HTML_PARSE_TASK_NAME: &str = "HTMLパース";
pub const HTML_PARSE_EVENT_NAME: &str = "batch-progress";

// ─────────────────────────────────────────────────────────────────────────────
// 入出力・コンテキスト型
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct HtmlParseInput {
    pub html_id: i64,
    pub url: String,
    pub html_content: String,
}

pub struct HtmlParseOutput {
    pub html_id: i64,
    pub order_number: String,
}

pub struct HtmlParseContext {
    pub pool: Arc<SqlitePool>,
}

// ─────────────────────────────────────────────────────────────────────────────
// BatchTask 実装
// ─────────────────────────────────────────────────────────────────────────────

pub struct HtmlParseTask;

#[async_trait]
impl BatchTask for HtmlParseTask {
    type Input = HtmlParseInput;
    type Output = HtmlParseOutput;
    type Context = HtmlParseContext;

    fn name(&self) -> &str {
        HTML_PARSE_TASK_NAME
    }

    fn event_name(&self) -> &str {
        HTML_PARSE_EVENT_NAME
    }

    async fn process(
        &self,
        input: Self::Input,
        ctx: &Self::Context,
    ) -> Result<Self::Output, String> {
        let order_number = extract_order_id_from_url(&input.url)
            .ok_or_else(|| format!("Cannot extract orderID from URL: {}", input.url))?;

        let mut tx = ctx
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin tx: {e}"))?;

        if html_parser::is_cancelled_order(&input.html_content) {
            apply_cancelled_order(&mut tx, &order_number).await?;
            tx.commit()
                .await
                .map_err(|e| format!("Failed to commit: {e}"))?;
            return Ok(HtmlParseOutput {
                html_id: input.html_id,
                order_number,
            });
        }

        let order_info = html_parser::parse_order_detail_html(&input.html_content, &order_number)?;

        SqliteOrderRepository::save_order_in_tx(
            &mut tx,
            &order_info,
            None,
            Some("amazon.co.jp".to_string()),
            None,
        )
        .await?;

        tx.commit()
            .await
            .map_err(|e| format!("Failed to commit: {e}"))?;

        Ok(HtmlParseOutput {
            html_id: input.html_id,
            order_number,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// キャンセル処理
// ─────────────────────────────────────────────────────────────────────────────

/// キャンセル済み HTML の処理: 既存注文のアイテムを削除し配送ステータスを更新する
async fn apply_cancelled_order(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    order_number: &str,
) -> Result<(), String> {
    let order: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM orders WHERE order_number = ? AND shop_domain = 'amazon.co.jp' LIMIT 1",
    )
    .bind(order_number)
    .fetch_optional(tx.as_mut())
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    let Some((order_id,)) = order else {
        log::warn!(
            "[html_parse] キャンセル済み注文が未登録: order_number={}",
            order_number
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
        "[html_parse] キャンセル適用: order_number={} order_id={}",
        order_number,
        order_id
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// URL ヘルパー
// ─────────────────────────────────────────────────────────────────────────────

/// URL の `orderID` クエリパラメータを取り出す
fn extract_order_id_from_url(url: &str) -> Option<String> {
    url.split('?').nth(1)?.split('&').find_map(|param| {
        let (key, value) = param.split_once('=')?;
        if key == "orderID" {
            Some(value.to_string())
        } else {
            None
        }
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// テスト
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_order_id_standard() {
        let url = "https://www.amazon.co.jp/your-orders/order-details?orderID=123-4567890-1234567";
        assert_eq!(
            extract_order_id_from_url(url),
            Some("123-4567890-1234567".to_string())
        );
    }

    #[test]
    fn test_extract_order_id_with_extra_params() {
        let url = "https://www.amazon.co.jp/your-orders/order-details?ref=ppx&orderID=234-5678901-2345678";
        assert_eq!(
            extract_order_id_from_url(url),
            Some("234-5678901-2345678".to_string())
        );
    }

    #[test]
    fn test_extract_order_id_missing() {
        assert_eq!(
            extract_order_id_from_url("https://www.amazon.co.jp/your-orders/orders"),
            None
        );
    }
}
