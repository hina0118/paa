//! Gemini API 連携モジュール
//!
//! # セキュリティガイドライン
//! このモジュールはGemini AIを使用して商品名を解析します。以下のルールを厳守してください：
//!
//! - **APIキーのログ出力禁止**: APIキーは絶対にログに出力しないこと
//! - **個人情報の除外**: AIに送るのは「商品名」のみ。住所・氏名・注文番号は送信しない
//! - **メトリクスのみ**: ログに出力できるのは処理件数、処理時間などの統計情報のみ

pub mod client;
pub mod config;
pub mod product_parse_task;
pub mod product_parser;

pub use client::{GeminiClient, GeminiClientTrait, ParsedProduct};
pub use config::{has_api_key, load_api_key};
pub use product_parse_task::{
    create_input as create_product_parse_input, ProductNameParseCache, ProductNameParseContext,
    ProductNameParseInput, ProductNameParseOutput, ProductNameParseTask,
    PRODUCT_NAME_PARSE_EVENT_NAME, PRODUCT_NAME_PARSE_TASK_NAME,
};
pub use product_parser::{normalize_product_name, ParseBatchResult, ProductParseService};
