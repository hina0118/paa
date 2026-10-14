//! 駿河屋マーケットプレイス マイページ HTML パースタスク
//!
//! `htmls` テーブルに保存済みの HTML を読み込み、注文情報をパースして DB に保存する。
//! WebView（ログイン）不要で何度でも再実行できる。

use async_trait::async_trait;

use super::html_parse_task::apply_cancelled_order;
use crate::batch_runner::BatchTask;
use crate::plugins::surugaya_mp::html_parser;
use crate::repository::SqliteOrderRepository;

pub const SURUGAYA_HTML_PARSE_TASK_NAME: &str = "駿河屋HTMLパース";
pub const SURUGAYA_HTML_PARSE_EVENT_NAME: &str = "batch-progress";

const SURUGAYA_DOMAIN: &str = "suruga-ya.jp";

// ─────────────────────────────────────────────────────────────────────────────
// 入出力・コンテキスト型
// ─────────────────────────────────────────────────────────────────────────────

/// 入力は Amazon と共通（`htmls` テーブルの1行）
pub type SurugayaHtmlParseInput = super::html_parse_task::HtmlParseInput;

pub struct SurugayaHtmlParseOutput {
    pub html_id: i64,
    pub trade_code: String,
}

pub type SurugayaHtmlParseContext = super::html_parse_task::HtmlParseContext;

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
            apply_cancelled_order(&mut tx, &trade_code, SURUGAYA_DOMAIN).await?;
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
            Some(SURUGAYA_DOMAIN.to_string()),
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
// URL ヘルパー
// ─────────────────────────────────────────────────────────────────────────────

/// `https://www.suruga-ya.jp/pcmypage/action_sell_search/detail?trade_code=M...`
/// から `trade_code` の値を取得する
fn extract_trade_code_from_url(url: &str) -> Option<String> {
    url.split('?').nth(1)?.split('&').find_map(|kv| {
        let (key, val) = kv.split_once('=')?;
        if key == "trade_code" {
            Some(val.to_string())
        } else {
            None
        }
    })
}
