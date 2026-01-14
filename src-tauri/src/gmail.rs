use google_gmail1::{hyper_rustls, Gmail};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use yup_oauth2 as oauth2;

// カスタムInstalledFlowDelegateでブラウザを自動的に開く
struct CustomFlowDelegate;

impl oauth2::authenticator_delegate::InstalledFlowDelegate for CustomFlowDelegate {
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        need_code: bool,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            log::info!("Opening browser with URL: {}", url);

            // ブラウザで認証URLを開く
            if let Err(e) = webbrowser::open(url) {
                log::warn!("Failed to open browser automatically: {}", e);
                log::warn!("Please open this URL manually in your browser:");
                log::warn!("{}", url);
            } else {
                log::info!("Browser opened successfully. Please complete the authentication in your browser.");
            }

            if need_code {
                log::info!("Waiting for authentication code...");
            }

            // HTTPRedirectモードでは空文字列を返す（リダイレクトでコードを受け取る）
            Ok(String::new())
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GmailMessage {
    pub message_id: String,
    pub snippet: String,
    pub body_plain: Option<String>,
    pub body_html: Option<String>,
    pub internal_date: i64,
}

pub struct GmailClient {
    hub: Gmail<hyper_rustls::HttpsConnector<HttpConnector>>,
}

impl GmailClient {
    pub async fn new(app_handle: &AppHandle) -> Result<Self, String> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;

        std::fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Failed to create app data dir: {}", e))?;

        // DBファイルと同じディレクトリに配置
        let token_path = app_data_dir.join("gmail_token.json");
        let client_secret_path = app_data_dir.join("client_secret.json");

        if !client_secret_path.exists() {
            return Err(format!(
                "Client secret file not found. Please place client_secret.json at: {}\n\nThis is the same directory where paa_data.db is stored.",
                client_secret_path.display()
            ));
        }

        let auth = Self::authenticate(&client_secret_path, &token_path).await?;

        // トークンを取得して認証を確実にする
        // gmail.readonlyスコープのみを使用（デスクトップアプリケーションに必要な最小限の権限）
        log::info!("Requesting OAuth token...");
        let _token = auth
            .token(&["https://www.googleapis.com/auth/gmail.readonly"])
            .await
            .map_err(|e| format!("Failed to get OAuth token: {}", e))?;
        log::info!("OAuth token obtained successfully");

        // Gmail Hub用のHTTPコネクタとクライアントを作成
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|e| format!("Failed to create HTTPS connector: {}", e))?
            .https_or_http()
            .enable_http1()
            .build();

        let client = Client::builder(TokioExecutor::new()).build(https);

        let hub = Gmail::new(client, auth);

        Ok(Self { hub })
    }

    async fn authenticate(
        client_secret_path: &PathBuf,
        token_path: &PathBuf,
    ) -> Result<oauth2::authenticator::Authenticator<hyper_rustls::HttpsConnector<HttpConnector>>, String>
    {
        let secret = oauth2::read_application_secret(client_secret_path)
            .await
            .map_err(|e| format!("Failed to read client secret: {}", e))?;

        log::info!("Starting OAuth authentication flow...");
        log::info!("Opening browser for authentication...");

        // カスタムブラウザオープナーを使用してHTTPRedirectモードで認証
        let auth = oauth2::InstalledFlowAuthenticator::builder(
            secret,
            oauth2::InstalledFlowReturnMethod::HTTPRedirect,
        )
        .persist_tokens_to_disk(token_path)
        .flow_delegate(Box::new(CustomFlowDelegate))
        .build()
        .await
        .map_err(|e| {
            format!(
                "Failed to create authenticator: {}\n\n\
                If a browser window didn't open, please check the console for the authentication URL and open it manually.\n\
                URL format: https://accounts.google.com/o/oauth2/auth?...",
                e
            )
        })?;

        Ok(auth)
    }

    pub async fn fetch_messages(&self, query: &str) -> Result<Vec<GmailMessage>, String> {
        let mut all_messages = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut req = self.hub.users().messages_list("me").q(query);

            if let Some(token) = page_token {
                req = req.page_token(&token);
            }

            let (_, result) = req
                .doit()
                .await
                .map_err(|e| format!("Failed to list messages: {}", e))?;

            if let Some(messages) = result.messages {
                // メッセージIDを収集
                let message_ids: Vec<String> = messages
                    .iter()
                    .filter_map(|msg| msg.id.clone())
                    .collect();

                log::info!("Fetching {} messages in parallel batches", message_ids.len());

                // 順次処理でメッセージを取得
                // 注: 並列処理はライフタイムの問題とGmail API制限により複雑
                // 将来的な改善: tokio::spawn + Arc<Mutex<Hub>>の使用を検討
                for message_id in message_ids {
                    match self.get_message(&message_id).await {
                        Ok(msg) => all_messages.push(msg),
                        Err(e) => log::warn!("Failed to fetch message {}: {}", message_id, e),
                    }
                }
            }

            page_token = result.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        Ok(all_messages)
    }

    async fn get_message(&self, message_id: &str) -> Result<GmailMessage, String> {
        log::debug!("Fetching message: {}", message_id);

        let (response, message) = self
            .hub
            .users()
            .messages_get("me", message_id)
            .format("full")
            .doit()
            .await
            .map_err(|e| format!("Failed to get message {}: {}", message_id, e))?;

        log::debug!("Response status: {:?}", response.status());

        let snippet = message.snippet.unwrap_or_default();
        let internal_date = message.internal_date.unwrap_or(0);

        let mut body_plain: Option<String> = None;
        let mut body_html: Option<String> = None;

        // 再帰的にMIMEパートを解析
        if let Some(payload) = &message.payload {
            Self::extract_body_from_part(payload, &mut body_plain, &mut body_html);
        }

        Ok(GmailMessage {
            message_id: message_id.to_string(),
            snippet,
            body_plain,
            body_html,
            internal_date,
        })
    }

    fn decode_base64(data: &str) -> String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

        // Gmail APIはbase64url形式（パディングなし）でエンコードされた文字列を返す
        match URL_SAFE_NO_PAD.decode(data) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
            Err(_) => {
                log::warn!("Failed to decode base64 data, returning empty string");
                String::new()
            }
        }
    }

    // 再帰的にMIMEパートを解析する
    fn extract_body_from_part(
        part: &google_gmail1::api::MessagePart,
        body_plain: &mut Option<String>,
        body_html: &mut Option<String>,
    ) {
        // 現在のパートのbodyをチェック
        if let Some(mime_type) = &part.mime_type {
            if let Some(body) = &part.body {
                if let Some(data) = &body.data {
                    if let Ok(data_str) = std::str::from_utf8(data) {
                        let decoded = Self::decode_base64(data_str);
                        match mime_type.as_ref() {
                            "text/plain" if body_plain.is_none() => {
                                *body_plain = Some(decoded);
                            }
                            "text/html" if body_html.is_none() => {
                                *body_html = Some(decoded);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // 子パートを再帰的に処理
        if let Some(parts) = &part.parts {
            for child_part in parts {
                Self::extract_body_from_part(child_part, body_plain, body_html);
            }
        }
    }
}
