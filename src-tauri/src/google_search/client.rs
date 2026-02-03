//! SerpApi 画像検索クライアント
//!
//! # セキュリティガイドライン
//! - APIキーはログに出力しない
//! - 商品名のみをAPIに送信（個人情報を含めない）
//!
//! # レート制限
//! - 無料枠は月100リクエストまで

use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request};
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 画像検索結果の1件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSearchResult {
    /// 画像のURL
    pub url: String,
    /// サムネイルURL
    pub thumbnail_url: Option<String>,
    /// 画像の幅
    pub width: Option<u32>,
    /// 画像の高さ
    pub height: Option<u32>,
    /// 画像のタイトル
    pub title: Option<String>,
    /// 画像のMIMEタイプ
    pub mime_type: Option<String>,
}

/// SerpApi レスポンスの構造
#[derive(Debug, Deserialize)]
struct SerpApiResponse {
    images_results: Option<Vec<SerpApiImageResult>>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SerpApiImageResult {
    original: Option<String>,
    thumbnail: Option<String>,
    original_width: Option<u32>,
    original_height: Option<u32>,
    title: Option<String>,
}

/// リクエストタイムアウト（秒）
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// 画像検索クライアントトレイト（テスト用モック対応）
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ImageSearchClientTrait: Send + Sync {
    /// 画像を検索
    async fn search_images(
        &self,
        query: &str,
        num_results: u32,
    ) -> Result<Vec<ImageSearchResult>, String>;
}

/// SerpApi クライアント実装
pub struct SerpApiClient {
    api_key: String,
    http_client: Client<HttpsConnector<HttpConnector>, Full<Bytes>>,
}

impl SerpApiClient {
    /// 新しい SerpApi クライアントを作成
    ///
    /// # セキュリティ
    /// APIキーはログに出力されません
    pub fn new(api_key: String) -> Result<Self, String> {
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|e| format!("Failed to create HTTPS connector: {e}"))?
            .https_or_http()
            .enable_http1()
            .build();

        let http_client = Client::builder(TokioExecutor::new()).build(https);

        // セキュリティ: APIキーをログに出力しない
        log::info!("SerpApiClient created");

        Ok(Self {
            api_key,
            http_client,
        })
    }

    /// API エンドポイント URL を構築
    fn build_url(&self, query: &str, num_results: u32) -> String {
        let encoded_query = urlencoding::encode(query);
        format!(
            "https://serpapi.com/search.json?engine=google_images&q={}&google_domain=google.co.jp&num={}&api_key={}",
            encoded_query,
            num_results.min(100), // SerpApi は最大100件
            self.api_key
        )
    }
}

#[async_trait]
impl ImageSearchClientTrait for SerpApiClient {
    async fn search_images(
        &self,
        query: &str,
        num_results: u32,
    ) -> Result<Vec<ImageSearchResult>, String> {
        if query.is_empty() {
            return Err("Search query is empty".to_string());
        }

        log::info!(
            "Searching images for query (length: {} chars), requesting {} results",
            query.len(),
            num_results
        );

        let url = self.build_url(query, num_results);

        // URLからAPIキーを除去してログ出力
        let safe_url = url.split("api_key=").next().unwrap_or(&url);
        log::debug!("SerpApi URL: {}...", safe_url);

        let req = Request::builder()
            .method(Method::GET)
            .uri(&url)
            .header("Accept", "application/json")
            .body(Full::new(Bytes::new()))
            .map_err(|e| format!("Failed to build request: {e}"))?;

        let request_result =
            tokio::time::timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS), async {
                let response = self
                    .http_client
                    .request(req)
                    .await
                    .map_err(|e| format!("Failed to send request to SerpApi: {e}"))?;
                let status = response.status();
                let body_bytes = response
                    .into_body()
                    .collect()
                    .await
                    .map_err(|e| format!("Failed to read response body: {e}"))?
                    .to_bytes();
                Ok::<_, String>((status, body_bytes))
            })
            .await;

        let (status, body_bytes) = match request_result {
            Ok(Ok((s, b))) => (s, b),
            Ok(Err(e)) => {
                log::error!("Failed to complete SerpApi request: {e}");
                return Err(e);
            }
            Err(_) => {
                log::error!(
                    "SerpApi request timed out after {} seconds",
                    REQUEST_TIMEOUT_SECS
                );
                return Err(format!(
                    "Request timed out after {} seconds",
                    REQUEST_TIMEOUT_SECS
                ));
            }
        };

        if !status.is_success() {
            log::error!(
                "SerpApi error (status {}), response body length: {} bytes",
                status,
                body_bytes.len()
            );

            // エラーレスポンスをパースしてより詳細なエラーメッセージを返す
            if let Ok(error_response) = serde_json::from_slice::<SerpApiResponse>(&body_bytes) {
                if let Some(error) = error_response.error {
                    if status.as_u16() == 401 {
                        return Err(format!(
                            "APIキーが無効です。設定画面でSerpApi APIキーを確認してください。\n詳細: {}",
                            error
                        ));
                    }
                    if status.as_u16() == 429 {
                        return Err("API利用制限に達しました。しばらく待ってから再度お試しください。".to_string());
                    }
                    return Err(format!("SerpApi error: {}", error));
                }
            }

            return Err(format!("SerpApi returned status {}", status));
        }

        let response: SerpApiResponse = serde_json::from_slice(&body_bytes)
            .map_err(|e| format!("Failed to parse SerpApi response: {e}"))?;

        if let Some(error) = response.error {
            log::error!("SerpApi returned error: {}", error);
            return Err(format!("SerpApi error: {}", error));
        }

        let results: Vec<ImageSearchResult> = response
            .images_results
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| {
                // original または thumbnail が必要
                let url = item.original.or(item.thumbnail.clone())?;

                // GIFは除外
                if url.to_lowercase().contains(".gif") {
                    return None;
                }

                Some(ImageSearchResult {
                    url,
                    thumbnail_url: item.thumbnail,
                    width: item.original_width,
                    height: item.original_height,
                    title: item.title,
                    mime_type: None,
                })
            })
            .collect();

        log::info!("SerpApi returned {} image(s)", results.len());
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url() {
        let client = SerpApiClient {
            api_key: "test_key".to_string(),
            http_client: Client::builder(TokioExecutor::new()).build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .unwrap()
                    .https_or_http()
                    .enable_http1()
                    .build(),
            ),
        };

        let url = client.build_url("test query", 5);
        assert!(url.contains("engine=google_images"));
        assert!(url.contains("q=test%20query"));
        assert!(url.contains("google_domain=google.co.jp"));
        assert!(url.contains("num=5"));
        assert!(url.contains("api_key=test_key"));
    }

    #[test]
    fn test_build_url_max_results() {
        let client = SerpApiClient {
            api_key: "test_key".to_string(),
            http_client: Client::builder(TokioExecutor::new()).build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .unwrap()
                    .https_or_http()
                    .enable_http1()
                    .build(),
            ),
        };

        // 100件を超えるリクエストは100件に制限される
        let url = client.build_url("test", 200);
        assert!(url.contains("num=100"));
    }

    #[test]
    fn test_image_search_result_serialization() {
        let result = ImageSearchResult {
            url: "https://example.com/image.jpg".to_string(),
            thumbnail_url: Some("https://example.com/thumb.jpg".to_string()),
            width: Some(800),
            height: Some(600),
            title: Some("Test Image".to_string()),
            mime_type: Some("image/jpeg".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("https://example.com/image.jpg"));
        assert!(json.contains("800"));
    }

    #[tokio::test]
    async fn test_mock_image_search_client() {
        let mut mock = MockImageSearchClientTrait::new();

        mock.expect_search_images().returning(|_, _| {
            Ok(vec![ImageSearchResult {
                url: "https://example.com/image.jpg".to_string(),
                thumbnail_url: Some("https://example.com/thumb.jpg".to_string()),
                width: Some(800),
                height: Some(600),
                title: Some("Test Image".to_string()),
                mime_type: None,
            }])
        });

        let result = mock.search_images("test query", 5).await;
        assert!(result.is_ok());

        let images = result.unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].url, "https://example.com/image.jpg");
    }
}
