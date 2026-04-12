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
        let mypage_info = html_parser::parse_mypage_html(&input.html_content)?;

        let mut tx = ctx
            .pool
            .begin()
            .await
            .map_err(|e| format!("Failed to begin tx: {e}"))?;

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
