//! Gemini API クライアント
//!
//! # セキュリティガイドライン
//! - APIキーはログに出力しない
//! - 商品名のみをAIに送信（個人情報を含めない）
//!
//! # レート制限対策
//! - 1リクエストで最大10件処理
//! - リクエスト間に10秒のディレイ
//! - RESOURCE_EXHAUSTED エラー時は処理をスキップ

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
use tokio::time::sleep;

/// Gemini API がパースした商品情報
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedProduct {
    pub maker: Option<String>,
    pub series: Option<String>,
    pub name: String,
    pub scale: Option<String>,
    pub is_reissue: bool,
}

/// Gemini API レスポンスの構造
#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<GeminiError>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: Option<Content>,
}

#[derive(Debug, Deserialize)]
struct Content {
    parts: Option<Vec<Part>>,
}

#[derive(Debug, Deserialize)]
struct Part {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    message: String,
}

/// Gemini API のレート制限関連定数
pub const GEMINI_BATCH_SIZE: usize = 10;
pub const GEMINI_DELAY_SECONDS: u64 = 10;

/// リクエスト送信〜レスポンスボディ取得のタイムアウト（秒）
/// ネットワークハング時に ProductNameParseState が永久に実行中のままになるのを防ぐ
const GEMINI_REQUEST_TIMEOUT_SECS: u64 = 120;

/// Gemini クライアントトレイト（テスト用モック対応）
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait GeminiClientTrait: Send + Sync {
    /// 単一の商品名をパース
    async fn parse_product_name(&self, product_name: &str) -> Result<ParsedProduct, String>;

    /// 単一チャンク（最大 GEMINI_BATCH_SIZE 件）をパース
    /// チャンク分割やディレイは呼び出し側で管理する
    /// エラー時は None を返し、呼び出し側でフォールバック処理を行う
    async fn parse_single_chunk(&self, product_names: &[String]) -> Option<Vec<ParsedProduct>>;

    /// 複数の商品名を一括パース（バッチ処理用）
    /// 内部で GEMINI_BATCH_SIZE 件ずつに分割し、間に GEMINI_DELAY_SECONDS 秒のディレイを入れる
    async fn parse_product_names_batch(
        &self,
        product_names: &[String],
    ) -> Result<Vec<ParsedProduct>, String>;
}

/// Gemini API クライアント実装
/// リクエストボディに Full<Bytes> を使用（hyper-util Client の型パラメータと一致）
pub struct GeminiClient {
    api_key: String,
    http_client: Client<HttpsConnector<HttpConnector>, Full<Bytes>>,
    model: String,
}

impl GeminiClient {
    /// 新しいGeminiクライアントを作成
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
        log::info!("GeminiClient created with model: gemini-2.0-flash-lite");

        Ok(Self {
            api_key,
            http_client,
            model: "gemini-2.0-flash-lite".to_string(),
        })
    }

    /// プロンプト構築
    fn build_prompt(&self, product_names: &[String]) -> String {
        let products_list = product_names
            .iter()
            .enumerate()
            .map(|(i, name)| format!("{}. {}", i + 1, name))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"あなたはECサイトの商品名を解析する専門家です。
以下の商品名テキストを解析し、各商品について情報を抽出してJSON配列で出力してください。

商品名リスト:
{products_list}

各商品について以下の形式で出力してください:
- maker: メーカー名（不明な場合は null）
- series: 作品名・シリーズ名（不明な場合は null）
- name: 商品名本体（型番や予約・再販などのノイズを除去したもの）
- scale: スケール情報（例: "1/7", "1/144", "NON"。不明な場合は null）
- is_reissue: 再販品かどうか（true/false）

注意事項:
- メーカー名は正式名称に統一してください（例: バンダイナムコ → BANDAI SPIRITS）
- 【再販】【予約】などのタグは is_reissue フラグで表現し、name からは除去してください
- 品番・型番は name に含めないでください

出力は必ず有効なJSON配列形式で、商品名リストと同じ順序で出力してください。"#
        )
    }

    /// Gemini API エンドポイントURL
    fn get_endpoint(&self) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.model
        )
    }

    /// APIリクエストボディを構築
    fn build_request_body(&self, prompt: &str) -> String {
        serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }]
            }],
            "generationConfig": {
                "responseMimeType": "application/json",
                "temperature": 0.1,
                "maxOutputTokens": 4096
            }
        })
        .to_string()
    }

    /// レスポンステキストをパース
    fn parse_response_text(&self, text: &str) -> Result<Vec<ParsedProduct>, String> {
        // JSONとしてパース
        let products: Vec<ParsedProduct> = serde_json::from_str(text).map_err(|e| {
            log::warn!("Failed to parse Gemini response as JSON array: {e}");
            format!("Failed to parse response: {e}")
        })?;

        Ok(products)
    }

    /// 単一のAPIリクエストを実行（内部用）
    /// RESOURCE_EXHAUSTED などのエラー時は None を返す（呼び出し元でフォールバック処理）
    async fn execute_single_request(&self, product_names: &[String]) -> Option<Vec<ParsedProduct>> {
        if product_names.is_empty() {
            return Some(Vec::new());
        }

        log::info!("Calling Gemini API for {} product(s)", product_names.len());

        let prompt = self.build_prompt(product_names);
        let request_body = self.build_request_body(&prompt);
        let endpoint = self.get_endpoint();

        // リクエストのメトリクスのみログに出力（内容や商品名は含めない）
        log::info!("Gemini API endpoint: {}", endpoint);
        log::debug!(
            "Gemini API request body length: {} bytes",
            request_body.len()
        );

        let body = Full::new(Bytes::from(request_body));
        let req = match Request::builder()
            .method(Method::POST)
            .uri(&endpoint)
            .header("Content-Type", "application/json")
            .header("X-goog-api-key", &self.api_key)
            .body(body)
        {
            Ok(r) => r,
            Err(e) => {
                log::error!("Failed to build request: {e}");
                return None;
            }
        };

        // ネットワークハング時に ProductNameParseState が永久に実行中のままになるのを防ぐためタイムアウトを設定
        let request_result =
            tokio::time::timeout(Duration::from_secs(GEMINI_REQUEST_TIMEOUT_SECS), async {
                let response = self
                    .http_client
                    .request(req)
                    .await
                    .map_err(|e| format!("Failed to send request to Gemini API: {e}"))?;
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
                log::error!("Failed to complete Gemini API request: {e}");
                return None;
            }
            Err(_) => {
                log::error!(
                    "Gemini API request timed out after {} seconds",
                    GEMINI_REQUEST_TIMEOUT_SECS
                );
                return None;
            }
        };

        if !status.is_success() {
            // レスポンスボディ全文はログに出さず、ステータスコードやボディ長などのメタ情報のみを出力
            // （API側のエラーメッセージがプロンプト=商品名を含むケースがあり、商品データがログに漏れる可能性があるため）
            log::error!(
                "Gemini API error (status {}), response body length: {} bytes",
                status,
                body_bytes.len()
            );

            let error_text = String::from_utf8_lossy(&body_bytes);
            // RESOURCE_EXHAUSTED (429) やその他のエラーは None を返してスキップ
            if status.as_u16() == 429 || error_text.contains("RESOURCE_EXHAUSTED") {
                log::warn!("Gemini API quota exceeded, skipping this batch");
            }
            return None;
        }

        let response_text = String::from_utf8_lossy(&body_bytes);
        let gemini_response: GeminiResponse = match serde_json::from_str(&response_text) {
            Ok(r) => r,
            Err(e) => {
                log::error!("Failed to parse Gemini response: {e}");
                return None;
            }
        };

        if let Some(error) = gemini_response.error {
            // エラーメッセージ本文は商品名等を含む可能性があるためログに出さず、メタ情報のみ
            log::error!(
                "Gemini API returned error object (message length: {} chars)",
                error.message.len()
            );
            return None;
        }

        let text = match gemini_response
            .candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content)
            .and_then(|c| c.parts)
            .and_then(|p| p.into_iter().next())
            .and_then(|p| p.text)
        {
            Some(t) => t,
            None => {
                log::error!("No content in Gemini response");
                return None;
            }
        };

        match self.parse_response_text(&text) {
            Ok(products) => {
                log::info!("Gemini API returned {} parsed product(s)", products.len());

                // 結果数が入力数と一致しない場合は警告
                if products.len() != product_names.len() {
                    log::warn!(
                        "Gemini returned {} products but expected {}",
                        products.len(),
                        product_names.len()
                    );
                }

                Some(products)
            }
            Err(e) => {
                log::error!("Failed to parse Gemini response text: {e}");
                None
            }
        }
    }
}

#[async_trait]
impl GeminiClientTrait for GeminiClient {
    async fn parse_product_name(&self, product_name: &str) -> Result<ParsedProduct, String> {
        self.parse_product_names_batch(&[product_name.to_string()])
            .await
            .and_then(|v| {
                v.into_iter()
                    .next()
                    .ok_or_else(|| "No result from Gemini API".to_string())
            })
    }

    /// 単一チャンク（最大 GEMINI_BATCH_SIZE 件）をパース
    /// execute_single_request のラッパー（トレイト経由でアクセス可能にする）
    async fn parse_single_chunk(&self, product_names: &[String]) -> Option<Vec<ParsedProduct>> {
        self.execute_single_request(product_names).await
    }

    /// 複数の商品名を一括パース
    /// - GEMINI_BATCH_SIZE 件ずつに分割して処理
    /// - 各リクエスト間に GEMINI_DELAY_SECONDS 秒のディレイ
    /// - エラー時はフォールバックとしてデフォルト値（元の商品名）を返す
    async fn parse_product_names_batch(
        &self,
        product_names: &[String],
    ) -> Result<Vec<ParsedProduct>, String> {
        if product_names.is_empty() {
            return Ok(Vec::new());
        }

        let total_count = product_names.len();
        let chunk_count = (total_count + GEMINI_BATCH_SIZE - 1) / GEMINI_BATCH_SIZE;

        log::info!(
            "Gemini batch parse: {} items in {} chunk(s) (batch size: {}, delay: {}s)",
            total_count,
            chunk_count,
            GEMINI_BATCH_SIZE,
            GEMINI_DELAY_SECONDS
        );

        let mut all_results: Vec<ParsedProduct> = Vec::with_capacity(total_count);

        for (chunk_idx, chunk) in product_names.chunks(GEMINI_BATCH_SIZE).enumerate() {
            // 2回目以降のリクエスト前にディレイを入れる
            if chunk_idx > 0 {
                log::info!(
                    "Waiting {} seconds before next Gemini API request...",
                    GEMINI_DELAY_SECONDS
                );
                sleep(Duration::from_secs(GEMINI_DELAY_SECONDS)).await;
            }

            log::info!(
                "Processing Gemini chunk {}/{}: {} items",
                chunk_idx + 1,
                chunk_count,
                chunk.len()
            );

            // API リクエストを実行
            match self.execute_single_request(chunk).await {
                Some(mut parsed) => {
                    // 結果数が一致しない場合はフォールバック
                    if parsed.len() != chunk.len() {
                        log::warn!(
                            "Gemini returned {} items but expected {}, using fallback",
                            parsed.len(),
                            chunk.len()
                        );
                        // 不足分をフォールバックで埋める
                        while parsed.len() < chunk.len() {
                            let idx = parsed.len();
                            parsed.push(ParsedProduct {
                                maker: None,
                                series: None,
                                name: chunk[idx].clone(),
                                scale: None,
                                is_reissue: false,
                            });
                        }
                    }
                    all_results.extend(parsed);
                }
                None => {
                    // エラー時はフォールバック（元の商品名をそのまま使用）
                    log::warn!(
                        "Gemini API failed for chunk {}, using fallback for {} items",
                        chunk_idx + 1,
                        chunk.len()
                    );
                    for name in chunk {
                        all_results.push(ParsedProduct {
                            maker: None,
                            series: None,
                            name: name.clone(),
                            scale: None,
                            is_reissue: false,
                        });
                    }
                }
            }
        }

        log::info!(
            "Gemini batch parse completed: {} items processed",
            all_results.len()
        );

        Ok(all_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_prompt_single_item() {
        let client = GeminiClient {
            api_key: "test".to_string(),
            http_client: Client::builder(TokioExecutor::new()).build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .unwrap()
                    .https_or_http()
                    .enable_http1()
                    .build(),
            ),
            model: "gemini-2.0-flash-lite".to_string(),
        };

        let prompt = client.build_prompt(&["KADOKAWA 1/7 レム".to_string()]);

        assert!(prompt.contains("1. KADOKAWA 1/7 レム"));
        assert!(prompt.contains("maker"));
        assert!(prompt.contains("series"));
        assert!(prompt.contains("is_reissue"));
    }

    #[test]
    fn test_build_prompt_multiple_items() {
        let client = GeminiClient {
            api_key: "test".to_string(),
            http_client: Client::builder(TokioExecutor::new()).build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .unwrap()
                    .https_or_http()
                    .enable_http1()
                    .build(),
            ),
            model: "gemini-2.0-flash-lite".to_string(),
        };

        let prompt = client.build_prompt(&[
            "商品A".to_string(),
            "商品B".to_string(),
            "商品C".to_string(),
        ]);

        assert!(prompt.contains("1. 商品A"));
        assert!(prompt.contains("2. 商品B"));
        assert!(prompt.contains("3. 商品C"));
    }

    #[test]
    fn test_parse_response_text_success() {
        let client = GeminiClient {
            api_key: "test".to_string(),
            http_client: Client::builder(TokioExecutor::new()).build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .unwrap()
                    .https_or_http()
                    .enable_http1()
                    .build(),
            ),
            model: "gemini-2.0-flash-lite".to_string(),
        };

        let response_text = r#"[
            {
                "maker": "KADOKAWA",
                "series": "Re:ゼロから始める異世界生活",
                "name": "レム 優雅美人ver.",
                "scale": "1/7",
                "is_reissue": true
            }
        ]"#;

        let result = client.parse_response_text(response_text);
        assert!(result.is_ok());

        let products = result.unwrap();
        assert_eq!(products.len(), 1);
        assert_eq!(products[0].maker, Some("KADOKAWA".to_string()));
        assert_eq!(
            products[0].series,
            Some("Re:ゼロから始める異世界生活".to_string())
        );
        assert_eq!(products[0].name, "レム 優雅美人ver.");
        assert_eq!(products[0].scale, Some("1/7".to_string()));
        assert!(products[0].is_reissue);
    }

    #[test]
    fn test_parse_response_text_invalid_json() {
        let client = GeminiClient {
            api_key: "test".to_string(),
            http_client: Client::builder(TokioExecutor::new()).build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .unwrap()
                    .https_or_http()
                    .enable_http1()
                    .build(),
            ),
            model: "gemini-2.0-flash-lite".to_string(),
        };

        let invalid_json = "not valid json";
        let result = client.parse_response_text(invalid_json);
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Failed to parse response"));
    }

    #[test]
    fn test_parse_response_text_invalid_array_element() {
        let client = GeminiClient {
            api_key: "test".to_string(),
            http_client: Client::builder(TokioExecutor::new()).build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .unwrap()
                    .https_or_http()
                    .enable_http1()
                    .build(),
            ),
            model: "gemini-2.0-flash-lite".to_string(),
        };

        // 配列形式だが要素がParsedProductの型と合わない
        let invalid_structure = r#"[{"invalid": "structure"}]"#;
        let result = client.parse_response_text(invalid_structure);
        assert!(result.is_err());
    }

    #[test]
    fn test_parsed_product_default() {
        let product = ParsedProduct::default();

        assert!(product.maker.is_none());
        assert!(product.series.is_none());
        assert_eq!(product.name, "");
        assert!(product.scale.is_none());
        assert!(!product.is_reissue);
    }

    #[tokio::test]
    async fn test_mock_gemini_client() {
        let mut mock = MockGeminiClientTrait::new();

        mock.expect_parse_product_name().returning(|_| {
            Ok(ParsedProduct {
                maker: Some("バンダイ".to_string()),
                series: Some("機動戦士ガンダム".to_string()),
                name: "RX-78-2 ガンダム".to_string(),
                scale: Some("1/144".to_string()),
                is_reissue: false,
            })
        });

        let result = mock.parse_product_name("バンダイ ガンダム 1/144").await;
        assert!(result.is_ok());

        let product = result.unwrap();
        assert_eq!(product.maker, Some("バンダイ".to_string()));
        assert_eq!(product.name, "RX-78-2 ガンダム");
    }
}
