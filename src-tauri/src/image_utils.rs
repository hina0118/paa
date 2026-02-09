//! 画像ダウンロード・保存の共通ロジック
//!
//! 注文確認メールのパース時や UI からの手動保存時に利用する。

use sqlx::SqlitePool;
use std::path::Path;

const MAX_IMAGE_SIZE_BYTES: usize = 10 * 1024 * 1024; // 10MB

/// 画像ダウンロード用URLの検証（SSRF対策）
pub(crate) fn validate_image_url(url_str: &str) -> Result<(), String> {
    use std::net::IpAddr;
    use url::Url;

    let parsed = Url::parse(url_str).map_err(|e| format!("Invalid URL: {e}"))?;

    // https:// のみ許可
    if parsed.scheme() != "https" {
        return Err("Only HTTPS URLs are allowed".to_string());
    }

    // ホスト名の検証
    let host_str = parsed.host_str().ok_or("URL has no host")?.to_lowercase();

    // localhost 系をブロック
    if host_str == "localhost"
        || host_str == "127.0.0.1"
        || host_str == "::1"
        || host_str == "0.0.0.0"
    {
        return Err("Localhost URLs are not allowed".to_string());
    }

    // メタデータエンドポイント
    if host_str == "169.254.169.254" || host_str == "metadata" {
        return Err("Metadata endpoint URLs are not allowed".to_string());
    }

    // IPアドレスの場合はプライベート範囲をブロック
    if let Ok(ip) = host_str.parse::<IpAddr>() {
        if is_private_ip(ip) {
            return Err("Private IP addresses are not allowed".to_string());
        }
    }

    Ok(())
}

pub(crate) fn is_private_ip(ip: std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            octets[0] == 10
                || (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31)
                || (octets[0] == 192 && octets[1] == 168)
                || octets[0] == 127
                || (octets[0] == 169 && octets[1] == 254)
        }
        IpAddr::V6(ipv6) => {
            let segments = ipv6.segments();
            (segments[0] == 0 && segments[1] == 0 && segments[2] == 0
                && segments[3] == 0
                && segments[4] == 0
                && segments[5] == 0
                && segments[6] == 0
                && segments[7] == 1)
                || (segments[0] & 0xffc0 == 0xfe80)
                || (segments[0] & 0xfe00 == 0xfc00)
        }
    }
}

/// 画像URLから画像をダウンロードして images テーブルに保存
///
/// * `skip_if_exists`: true のとき、既存レコードがあればダウンロードせずスキップ（パース用）
pub async fn save_image_from_url_for_item(
    pool: &SqlitePool,
    images_dir: &Path,
    item_name_normalized: &str,
    image_url: &str,
    skip_if_exists: bool,
) -> Result<String, String> {
    use bytes::Bytes;
    use http_body_util::{BodyExt, Full};
    use hyper::{Method, Request};
    use hyper_rustls::HttpsConnector;
    use hyper_util::client::legacy::connect::HttpConnector;
    use hyper_util::client::legacy::Client;
    use hyper_util::rt::TokioExecutor;
    use std::time::Duration;

    if skip_if_exists {
        let existing: Option<String> =
            sqlx::query_scalar("SELECT file_name FROM images WHERE item_name_normalized = ?")
                .bind(item_name_normalized)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Failed to check existing image: {e}"))?
                .flatten();
        if existing.is_some() {
            log::debug!(
                "Image already exists for item_name_normalized={}, skipping download",
                item_name_normalized
            );
            return Ok(existing.unwrap_or_default());
        }
    }

    validate_image_url(image_url)?;

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .map_err(|e| format!("Failed to create HTTPS connector: {e}"))?
        .https_only()
        .enable_http1()
        .build();

    let http_client: Client<HttpsConnector<HttpConnector>, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build(https);

    let req = Request::builder()
        .method(Method::GET)
        .uri(image_url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .body(Full::new(Bytes::new()))
        .map_err(|e| format!("Failed to build request: {e}"))?;

    let request_result = tokio::time::timeout(Duration::from_secs(30), async {
        let response = http_client
            .request(req)
            .await
            .map_err(|e| format!("Failed to download image: {e}"))?;
        let status = response.status();
        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok());
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| format!("Failed to read image body: {e}"))?
            .to_bytes();
        Ok::<_, String>((status, content_length, body_bytes))
    })
    .await;

    let (status, content_length, image_data) = match request_result {
        Ok(Ok((s, cl, b))) => (s, cl, b),
        Ok(Err(e)) => return Err(e),
        Err(_) => return Err("Image download timed out".to_string()),
    };

    if !status.is_success() {
        return Err(format!("Failed to download image: HTTP {}", status));
    }

    if let Some(len) = content_length {
        if len > MAX_IMAGE_SIZE_BYTES {
            return Err(format!(
                "Image too large ({} bytes). Maximum size is {} MB",
                len,
                MAX_IMAGE_SIZE_BYTES / (1024 * 1024)
            ));
        }
    }

    if image_data.len() > MAX_IMAGE_SIZE_BYTES {
        return Err(format!(
            "Image too large ({} bytes). Maximum size is {} MB",
            image_data.len(),
            MAX_IMAGE_SIZE_BYTES / (1024 * 1024)
        ));
    }

    let format =
        image::guess_format(&image_data).map_err(|e| format!("Invalid image format: {e}"))?;
    let extension = match format {
        image::ImageFormat::Jpeg => "jpg",
        image::ImageFormat::Png => "png",
        image::ImageFormat::WebP => "webp",
        _ => {
            return Err(
                "Unsupported image format. Only JPEG, PNG, and WebP are allowed".to_string(),
            );
        }
    };

    let file_name = format!("{}.{}", uuid::Uuid::new_v4(), extension);

    std::fs::create_dir_all(images_dir)
        .map_err(|e| format!("Failed to create images directory: {e}"))?;

    let old_file_name: Option<String> =
        sqlx::query_scalar("SELECT file_name FROM images WHERE item_name_normalized = ?")
            .bind(item_name_normalized)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("Failed to get existing image: {e}"))?
            .flatten();

    let file_path = images_dir.join(&file_name);
    std::fs::write(&file_path, &image_data)
        .map_err(|e| format!("Failed to write image file: {e}"))?;

    let existing: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM images WHERE item_name_normalized = ?")
            .bind(item_name_normalized)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("Failed to check existing image: {e}"))?;

    if existing.is_some() {
        sqlx::query(
            r#"
            UPDATE images
            SET file_name = ?, created_at = CURRENT_TIMESTAMP
            WHERE item_name_normalized = ?
            "#,
        )
        .bind(&file_name)
        .bind(item_name_normalized)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to update image in database: {e}"))?;
    } else {
        sqlx::query(
            r#"
            INSERT INTO images (item_name_normalized, file_name, created_at)
            VALUES (?, ?, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(item_name_normalized)
        .bind(&file_name)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to save image to database: {e}"))?;
    }

    if let Some(ref old_name) = old_file_name {
        if old_name != &file_name {
            let old_path = images_dir.join(old_name);
            if let Err(e) = std::fs::remove_file(&old_path) {
                log::warn!("Failed to delete old image {}: {}", old_name, e);
            }
        }
    }

    log::info!(
        "Saved image for item_name_normalized={} from {}",
        item_name_normalized,
        image_url
    );

    Ok(file_name)
}
