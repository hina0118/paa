//! SerpApi 画像検索モジュール
//!
//! # セキュリティガイドライン
//! このモジュールはSerpApiを使用して商品画像を検索します。
//! 以下のルールを厳守してください：
//!
//! - **APIキーのログ出力禁止**: APIキーは絶対にログに出力しないこと
//! - **個人情報の除外**: AIに送るのは「商品名」のみ。住所・氏名・注文番号は送信しない
//! - **メトリクスのみ**: ログに出力できるのは処理件数、処理時間などの統計情報のみ

pub mod client;
pub mod config;

pub use client::{ImageSearchClientTrait, ImageSearchResult, SerpApiClient};
pub use config::{delete_api_key, has_api_key, is_configured, load_api_key, save_api_key};
