use google_gmail1::{hyper_rustls, Gmail};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use yup_oauth2 as oauth2;

#[derive(Debug, Serialize, Deserialize)]
pub struct GmailMessage {
    pub message_id: String,
    pub snippet: String,
    pub body_plain: Option<String>,
    pub body_html: Option<String>,
    pub internal_date: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FetchResult {
    pub fetched_count: usize,
    pub saved_count: usize,
    pub skipped_count: usize,
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

        let auth = oauth2::InstalledFlowAuthenticator::builder(
            secret,
            oauth2::InstalledFlowReturnMethod::HTTPRedirect,
        )
        .persist_tokens_to_disk(token_path)
        .build()
        .await
        .map_err(|e| format!("Failed to create authenticator: {}", e))?;

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
                for msg in messages {
                    if let Some(id) = &msg.id {
                        let full_message = self.get_message(id).await?;
                        all_messages.push(full_message);
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
        let (_, message) = self
            .hub
            .users()
            .messages_get("me", message_id)
            .format("full")
            .doit()
            .await
            .map_err(|e| format!("Failed to get message {}: {}", message_id, e))?;

        let snippet = message.snippet.unwrap_or_default();
        let internal_date = message.internal_date.unwrap_or(0);

        let mut body_plain: Option<String> = None;
        let mut body_html: Option<String> = None;

        if let Some(payload) = message.payload {
            if let Some(parts) = payload.parts {
                for part in parts {
                    if let Some(mime_type) = &part.mime_type {
                        if let Some(body) = &part.body {
                            if let Some(data) = &body.data {
                                let decoded = Self::decode_base64(data);
                                match mime_type.as_ref() {
                                    "text/plain" => body_plain = Some(decoded),
                                    "text/html" => body_html = Some(decoded),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            } else if let Some(body) = payload.body {
                if let Some(data) = &body.data {
                    let decoded = Self::decode_base64(data);
                    if let Some(mime_type) = &payload.mime_type {
                        match mime_type.as_ref() {
                            "text/plain" => body_plain = Some(decoded),
                            "text/html" => body_html = Some(decoded),
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(GmailMessage {
            message_id: message_id.to_string(),
            snippet,
            body_plain,
            body_html,
            internal_date,
        })
    }

    fn decode_base64(data: &[u8]) -> String {
        String::from_utf8_lossy(data).to_string()
    }
}

pub async fn save_messages_to_db(
    app_handle: &AppHandle,
    messages: Vec<GmailMessage>,
) -> Result<FetchResult, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let db_path = app_data_dir.join("paa_data.db");
    let db_url = format!("sqlite:{}", db_path.to_string_lossy());

    let pool = sqlx::SqlitePool::connect(&db_url)
        .await
        .map_err(|e| format!("Failed to connect to database: {}", e))?;

    let mut saved_count = 0;
    let mut skipped_count = 0;

    for msg in &messages {
        let result = sqlx::query(
            r#"
            INSERT INTO emails (message_id, body_plain, body_html)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(message_id) DO NOTHING
            "#,
        )
        .bind(&msg.message_id)
        .bind(&msg.body_plain)
        .bind(&msg.body_html)
        .execute(&pool)
        .await
        .map_err(|e| format!("Failed to insert message {}: {}", msg.message_id, e))?;

        if result.rows_affected() > 0 {
            saved_count += 1;
        } else {
            skipped_count += 1;
        }
    }

    pool.close().await;

    Ok(FetchResult {
        fetched_count: messages.len(),
        saved_count,
        skipped_count,
    })
}
