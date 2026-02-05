//! E2E テスト用の外部APIモック
//!
//! 環境変数 PAA_E2E_MOCK=1 が設定されている場合、Gmail・Gemini・SerpApi の
//! 実際のAPI呼び出しをモックに置き換え、CIやローカルE2Eで外部依存なしにテスト可能にする。

use async_trait::async_trait;

use crate::gemini::client::{GeminiClientTrait, ParsedProduct};
use crate::gmail::client::GmailMessage;
use crate::gmail_client::GmailClientTrait;
use crate::google_search::client::{ImageSearchClientTrait, ImageSearchResult};

/// E2E用 Gmail API モック（空のメッセージリストを返す）
pub struct E2EMockGmailClient;

#[async_trait]
impl GmailClientTrait for E2EMockGmailClient {
    async fn list_message_ids(
        &self,
        _query: &str,
        _max_results: u32,
        _page_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), String> {
        log::info!("[E2E Mock] Gmail list_message_ids: returning empty list");
        Ok((vec![], None))
    }

    async fn get_message(&self, message_id: &str) -> Result<GmailMessage, String> {
        log::info!("[E2E Mock] Gmail get_message: {} (unused)", message_id);
        Err("E2E mock: get_message should not be called with empty list".to_string())
    }
}

/// E2E用 Gemini API モック（入力商品名をそのままパース結果として返す）
pub struct E2EMockGeminiClient;

#[async_trait]
impl GeminiClientTrait for E2EMockGeminiClient {
    async fn parse_product_name(&self, product_name: &str) -> Result<ParsedProduct, String> {
        log::info!("[E2E Mock] Gemini parse_product_name: {}", product_name);
        Ok(ParsedProduct {
            maker: None,
            series: None,
            name: product_name.to_string(),
            scale: None,
            is_reissue: false,
        })
    }

    async fn parse_single_chunk(&self, product_names: &[String]) -> Option<Vec<ParsedProduct>> {
        log::info!(
            "[E2E Mock] Gemini parse_single_chunk: {} items",
            product_names.len()
        );
        Some(
            product_names
                .iter()
                .map(|n| ParsedProduct {
                    maker: None,
                    series: None,
                    name: n.clone(),
                    scale: None,
                    is_reissue: false,
                })
                .collect(),
        )
    }

    async fn parse_product_names_batch(
        &self,
        product_names: &[String],
    ) -> Result<Vec<ParsedProduct>, String> {
        log::info!(
            "[E2E Mock] Gemini parse_product_names_batch: {} items",
            product_names.len()
        );
        Ok(product_names
            .iter()
            .map(|n| ParsedProduct {
                maker: None,
                series: None,
                name: n.clone(),
                scale: None,
                is_reissue: false,
            })
            .collect())
    }
}

/// E2E用 SerpApi 画像検索モック（ダミーURLを返す）
pub struct E2EMockImageSearchClient;

#[async_trait]
impl ImageSearchClientTrait for E2EMockImageSearchClient {
    async fn search_images(
        &self,
        query: &str,
        num_results: u32,
    ) -> Result<Vec<ImageSearchResult>, String> {
        log::info!(
            "[E2E Mock] SerpApi search_images: query={}, num_results={}",
            query,
            num_results
        );
        let count = num_results.min(3) as usize;
        Ok((0..count)
            .map(|i| ImageSearchResult {
                url: format!("https://example.com/e2e-mock-image-{}.png", i + 1),
                thumbnail_url: Some(format!(
                    "https://example.com/e2e-mock-thumb-{}.png",
                    i + 1
                )),
                width: Some(100),
                height: Some(100),
                title: Some(format!("E2E Mock Image {}", i + 1)),
                mime_type: Some("image/png".to_string()),
            })
            .collect())
    }
}

/// 環境変数 PAA_E2E_MOCK が設定されているか
pub fn is_e2e_mock_mode() -> bool {
    std::env::var("PAA_E2E_MOCK").as_deref() == Ok("1")
}

/// Gmail クライアントの E2E 対応ラッパー（実機 or モックを切り替え）
pub enum GmailClientForE2E {
    Real(crate::gmail::GmailClient),
    Mock(E2EMockGmailClient),
}

#[async_trait]
impl GmailClientTrait for GmailClientForE2E {
    async fn list_message_ids(
        &self,
        query: &str,
        max_results: u32,
        page_token: Option<String>,
    ) -> Result<(Vec<String>, Option<String>), String> {
        match self {
            Self::Real(c) => c.list_message_ids(query, max_results, page_token).await,
            Self::Mock(m) => m.list_message_ids(query, max_results, page_token).await,
        }
    }

    async fn get_message(&self, message_id: &str) -> Result<GmailMessage, String> {
        match self {
            Self::Real(c) => c.get_message(message_id).await,
            Self::Mock(m) => m.get_message(message_id).await,
        }
    }
}

/// Gemini クライアントの E2E 対応ラッパー（実機 or モックを切り替え）
pub enum GeminiClientForE2E {
    Real(crate::gemini::GeminiClient),
    Mock(E2EMockGeminiClient),
}

#[async_trait]
impl GeminiClientTrait for GeminiClientForE2E {
    async fn parse_product_name(&self, product_name: &str) -> Result<ParsedProduct, String> {
        match self {
            Self::Real(c) => c.parse_product_name(product_name).await,
            Self::Mock(m) => m.parse_product_name(product_name).await,
        }
    }

    async fn parse_single_chunk(&self, product_names: &[String]) -> Option<Vec<ParsedProduct>> {
        match self {
            Self::Real(c) => c.parse_single_chunk(product_names).await,
            Self::Mock(m) => m.parse_single_chunk(product_names).await,
        }
    }

    async fn parse_product_names_batch(
        &self,
        product_names: &[String],
    ) -> Result<Vec<ParsedProduct>, String> {
        match self {
            Self::Real(c) => c.parse_product_names_batch(product_names).await,
            Self::Mock(m) => m.parse_product_names_batch(product_names).await,
        }
    }
}
