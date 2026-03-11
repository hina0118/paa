//! Gemini Vision API を使ったOCR処理
//!
//! 画像バイト列をGemini Vision APIに送信し、含まれるテキストを抽出する。
//! 既存の GeminiClient の HTTP クライアントと同じ構成を使用する。

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use std::time::Duration;

const OCR_MODEL: &str = "gemini-2.0-flash-lite";
const OCR_TIMEOUT_SECS: u64 = 30;

/// 画像バイト列（PNG）をGemini Vision APIでOCR処理し、テキストを返す
///
/// # セキュリティ
/// APIキーはログに出力されない
pub async fn ocr_image_bytes(api_key: &str, image_bytes: &[u8]) -> Result<String, String> {
    let image_base64 = BASE64.encode(image_bytes);

    let request_body = serde_json::json!({
        "contents": [{
            "parts": [
                {
                    "inline_data": {
                        "mime_type": "image/png",
                        "data": image_base64
                    }
                },
                {
                    "text": "この画像に含まれているテキストをすべて抽出してください。商品名・型番・メーカー名などが含まれる場合はそのまま出力してください。テキストのみを出力し、説明や解説は不要です。"
                }
            ]
        }],
        "generationConfig": {
            "temperature": 0.0,
            "maxOutputTokens": 1024
        }
    })
    .to_string();

    let https = HttpsConnectorBuilder::new()
        .with_native_roots()
        .map_err(|e| format!("HTTPS connector error: {e}"))?
        .https_or_http()
        .enable_http1()
        .build();

    let http_client = Client::builder(TokioExecutor::new()).build::<_, Full<Bytes>>(https);

    let endpoint = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        OCR_MODEL
    );

    let body = Full::new(Bytes::from(request_body));
    let req = Request::builder()
        .method(Method::POST)
        .uri(&endpoint)
        .header("Content-Type", "application/json")
        .header("X-goog-api-key", api_key)
        .body(body)
        .map_err(|e| format!("Failed to build request: {e}"))?;

    let result =
        tokio::time::timeout(Duration::from_secs(OCR_TIMEOUT_SECS), async {
            let response = http_client
                .request(req)
                .await
                .map_err(|e| format!("Request failed: {e}"))?;
            let status = response.status();
            let body_bytes = response
                .into_body()
                .collect()
                .await
                .map_err(|e| format!("Failed to read body: {e}"))?
                .to_bytes();
            Ok::<_, String>((status, body_bytes))
        })
        .await
        .map_err(|_| format!("OCR request timed out after {OCR_TIMEOUT_SECS}s"))??;

    let (status, body_bytes) = result;
    if !status.is_success() {
        return Err(format!(
            "Gemini OCR API error: HTTP {} (body: {} bytes)",
            status,
            body_bytes.len()
        ));
    }

    let response: serde_json::Value = serde_json::from_slice(&body_bytes)
        .map_err(|e| format!("Failed to parse OCR response: {e}"))?;

    let text = response["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| "No text in OCR response".to_string())?
        .trim()
        .to_string();

    log::info!("OCR extracted {} chars", text.len());
    Ok(text)
}
