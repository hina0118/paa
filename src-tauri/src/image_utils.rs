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

    // ホスト名の検証（SSRF 対策）
    let host = parsed.host().ok_or("URL has no host")?;
    match host {
        url::Host::Domain(domain) => {
            let host_str = domain.to_lowercase();

            // localhost 系をブロック
            if host_str == "localhost" {
                return Err("Localhost URLs are not allowed".to_string());
            }

            // メタデータエンドポイント
            if host_str == "metadata" {
                return Err("Metadata endpoint URLs are not allowed".to_string());
            }
        }
        url::Host::Ipv4(ipv4) => {
            // ループバック / unspecified は localhost 扱い
            if ipv4.is_loopback() || ipv4.is_unspecified() {
                return Err("Localhost URLs are not allowed".to_string());
            }

            // 169.254.169.254 はクラウドメタデータで有名
            if ipv4.octets() == [169, 254, 169, 254] {
                return Err("Metadata endpoint URLs are not allowed".to_string());
            }

            if is_private_ip(IpAddr::V4(ipv4)) {
                return Err("Private IP addresses are not allowed".to_string());
            }
        }
        url::Host::Ipv6(ipv6) => {
            if ipv6.is_loopback() {
                return Err("Localhost URLs are not allowed".to_string());
            }
            if is_private_ip(IpAddr::V6(ipv6)) {
                return Err("Private IP addresses are not allowed".to_string());
            }
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
            (segments[0] == 0
                && segments[1] == 0
                && segments[2] == 0
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

        // Content-Length が取得できる場合は、ボディ読み込み前に上限チェックして中断
        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok());
        if let Some(len) = content_length {
            if len > MAX_IMAGE_SIZE_BYTES {
                return Err(format!(
                    "Image too large ({} bytes). Maximum size is {} MB",
                    len,
                    MAX_IMAGE_SIZE_BYTES / (1024 * 1024)
                ));
            }
        }

        let body_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| format!("Failed to read image body: {e}"))?
            .to_bytes();
        Ok::<_, String>((status, body_bytes))
    })
    .await;

    let (status, image_data) = match request_result {
        Ok(Ok((s, b))) => (s, b),
        Ok(Err(e)) => return Err(e),
        Err(_) => return Err("Image download timed out".to_string()),
    };

    if !status.is_success() {
        return Err(format!("Failed to download image: HTTP {}", status));
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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::str::FromStr;

    async fn create_test_pool_with_images_table() -> SqlitePool {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS images (
              id INTEGER PRIMARY KEY,
              item_name_normalized TEXT NOT NULL UNIQUE,
              file_name TEXT NOT NULL,
              created_at TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[test]
    fn validate_image_url_rejects_non_https() {
        let err = validate_image_url("http://example.com/a.jpg").unwrap_err();
        assert_eq!(err, "Only HTTPS URLs are allowed");
    }

    #[test]
    fn validate_image_url_rejects_missing_host() {
        let err = validate_image_url("https://").unwrap_err();
        assert!(err.contains("Invalid URL:"));
    }

    #[test]
    fn validate_image_url_rejects_localhost_hosts() {
        for host in ["localhost", "127.0.0.1", "0.0.0.0", "[::1]"] {
            let err = validate_image_url(&format!("https://{host}/a.png")).unwrap_err();
            assert_eq!(err, "Localhost URLs are not allowed");
        }
    }

    #[test]
    fn validate_image_url_rejects_metadata_endpoints() {
        for host in ["169.254.169.254", "metadata"] {
            let err = validate_image_url(&format!("https://{host}/a.webp")).unwrap_err();
            assert_eq!(err, "Metadata endpoint URLs are not allowed");
        }
    }

    #[test]
    fn is_private_ip_v4_ranges() {
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 1, 2))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    }

    #[test]
    fn is_private_ip_v6_ranges() {
        assert!(is_private_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        assert!(is_private_ip(IpAddr::V6(Ipv6Addr::from_str("fe80::1").unwrap())));
        assert!(is_private_ip(IpAddr::V6(Ipv6Addr::from_str("fc00::1").unwrap())));
        assert!(!is_private_ip(IpAddr::V6(
            Ipv6Addr::from_str("2001:4860:4860::8888").unwrap()
        )));
    }

    #[test]
    fn validate_image_url_rejects_private_ip_host() {
        let err = validate_image_url("https://10.0.0.1/a.jpg").unwrap_err();
        assert_eq!(err, "Private IP addresses are not allowed");
    }

    #[test]
    fn validate_image_url_accepts_public_https_url() {
        validate_image_url("https://example.com/a.jpg").unwrap();
    }

    #[tokio::test]
    async fn save_image_skip_if_exists_returns_existing_file_name_without_validating_url() {
        let pool = create_test_pool_with_images_table().await;

        sqlx::query("INSERT INTO images (item_name_normalized, file_name) VALUES (?, ?)")
            .bind("item-1")
            .bind("existing.png")
            .execute(&pool)
            .await
            .unwrap();

        let tmp = tempfile::tempdir().unwrap();

        // URL は不正でも、skip_if_exists=true かつ既存があれば早期 return で成功する
        let file_name = save_image_from_url_for_item(
            &pool,
            tmp.path(),
            "item-1",
            "http://not-https.example/a.png",
            true,
        )
        .await
        .unwrap();

        assert_eq!(file_name, "existing.png");
    }

    #[tokio::test]
    async fn save_image_skip_if_exists_validates_url_when_no_existing_record() {
        let pool = create_test_pool_with_images_table().await;
        let tmp = tempfile::tempdir().unwrap();

        let err = save_image_from_url_for_item(
            &pool,
            tmp.path(),
            "item-2",
            "http://example.com/a.png",
            true,
        )
        .await
        .unwrap_err();

        assert_eq!(err, "Only HTTPS URLs are allowed");
    }
}
