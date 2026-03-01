//! 配送状況確認バッチ
//!
//! 各配送業者の追跡ページをHTTPで取得し、HTMLのキーワードマッチで
//! 配送ステータスを判定して `tracking_check_logs` に記録する。

use async_trait::async_trait;
use bytes::Bytes;
use reqwest::Client;
use sqlx::sqlite::SqlitePool;

use crate::batch_runner::BatchTask;

pub const DELIVERY_CHECK_TASK_NAME: &str = "配送状況確認";
pub const DELIVERY_CHECK_EVENT_NAME: &str = "batch-progress";

/// リクエストタイムアウト（秒）
const REQUEST_TIMEOUT_SECS: u64 = 20;
/// ブラウザとして振る舞うための User-Agent
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

// ---------------------------------------------------------------------------
// 入出力型
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DeliveryCheckInput {
    pub delivery_id: i64,
    pub tracking_number: String,
    pub carrier: String,
}

#[derive(Debug)]
pub struct DeliveryCheckOutput {
    pub delivery_id: i64,
    pub check_status: String, // "success" | "failed" | "not_found"
}

// ---------------------------------------------------------------------------
// コンテキスト
// ---------------------------------------------------------------------------

pub struct DeliveryCheckContext {
    pub pool: SqlitePool,
    pub http_client: Client,
}

impl DeliveryCheckContext {
    /// reqwest + native-tls（Windows: SChannel）の HTTPS クライアントを作成。
    /// OS のTLSスタックを使うため、日本の配送業者サイトとの互換性が高い。
    pub fn new(pool: SqlitePool) -> Result<Self, String> {
        let http_client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS as usize))
            .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| format!("HTTP client error: {e}"))?;
        Ok(Self { pool, http_client })
    }
}

// ---------------------------------------------------------------------------
// 追跡URL構築
// ---------------------------------------------------------------------------

fn build_tracking_url(carrier: &str, tracking_number: &str) -> Option<String> {
    let num = urlencoding::encode(tracking_number.trim()).into_owned();
    if carrier.contains("佐川") {
        Some(format!(
            "https://k2k.sagawa-exp.co.jp/p/web/okurijosearch.do?okurijoNo={num}"
        ))
    } else if carrier.contains("日本郵便")
        || carrier.contains("ゆうパケット")
        || carrier.contains("ゆうパック")
    {
        Some(format!(
            "https://trackings.post.japanpost.jp/services/srv/search/?requestNo={num}"
        ))
    } else if carrier.contains("ヤマト") || carrier.contains("クロネコ") {
        // POST で送信するため URL にはパラメータを付けない
        Some("https://toi.kuronekoyamato.co.jp/cgi-bin/tneko".to_string())
    } else {
        None
    }
}

/// ヤマト運輸向けフォーム POST ボディを構築する。
/// `number00=1` が必須（これがないと正しい検索結果が返らない）。
fn build_yamato_post_body(tracking_number: &str) -> String {
    let num = urlencoding::encode(tracking_number.trim()).into_owned();
    format!(
        "number00=1&number01={num}\
         &number02=&number03=&number04=&number05=\
         &number06=&number07=&number08=&number09=&number10="
    )
}

// ---------------------------------------------------------------------------
// HTMLエンティティデコード
// ---------------------------------------------------------------------------

/// `&#NNNNN;` 形式の数値文字参照を Unicode 文字に展開する。
/// 佐川急便など Windows-31J ページはタグ外テキストをすべてエンティティで書く。
fn decode_numeric_entities(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp_pos) = rest.find("&#") {
        result.push_str(&rest[..amp_pos]);
        rest = &rest[amp_pos + 2..]; // "&#" の次から
                                     // 16進 &#xNNNN; か 10進 &#NNNNN; か判別
        let (is_hex, rest2) = if rest.starts_with('x') || rest.starts_with('X') {
            (true, &rest[1..])
        } else {
            (false, rest)
        };
        if let Some(semi) = rest2.find(';') {
            let num_str = &rest2[..semi];
            let codepoint = if is_hex {
                u32::from_str_radix(num_str, 16).ok()
            } else {
                num_str.parse::<u32>().ok()
            };
            if let Some(ch) = codepoint.and_then(char::from_u32) {
                result.push(ch);
                rest = &rest2[semi + 1..];
                continue;
            }
        }
        // デコード失敗 → そのまま残す
        result.push_str("&#");
    }
    result.push_str(rest);
    result
}

// ---------------------------------------------------------------------------
// HTMLからステータスを判定
// ---------------------------------------------------------------------------

/// チェック結果
struct ParsedStatus {
    check_status: &'static str,    // "success" | "not_found"
    delivery_status: &'static str, // deliveries.delivery_status 値
    description: Option<String>,
}

fn parse_tracking_html(carrier: &str, html: &str) -> ParsedStatus {
    // HTML エンティティ（&#NNNNN;）をデコードしてからキーワード検索する。
    // 佐川急便など Windows-31J ページはテキストをすべてエンティティで書くため必須。
    let decoded = decode_numeric_entities(html);
    let html = decoded.as_str();

    // --- not_found パターン（情報なし）---
    let not_found_keywords: &[&str] = if carrier.contains("佐川") {
        &[
            "お荷物データが登録されておりません", // 実際のSagawa not_found メッセージ
            "該当なし",
        ]
    } else if carrier.contains("日本郵便")
        || carrier.contains("ゆうパケット")
        || carrier.contains("ゆうパック")
    {
        &[
            "追跡情報がありません",
            "お探しの郵便物",
            "見当たりません",
            "取扱なし",
        ]
    } else {
        // ヤマト
        &[
            "お荷物情報が見つかりません",
            "ご指定のお荷物情報が存在しません",
            "お荷物情報照会できませんでした",
            "伝票番号未登録", // 古い番号や存在しない番号の場合
        ]
    };

    for kw in not_found_keywords {
        if html.contains(kw) {
            return ParsedStatus {
                check_status: "not_found",
                delivery_status: "delivered", // 不明 → 配達完了扱い
                description: Some("追跡情報なし（不明）".to_string()),
            };
        }
    }

    // --- 配達完了 ---
    let delivered_keywords: &[&str] = &[
        "お届け済み",
        "配達完了",
        "ご不在連絡票をお届け済み",
        "配達しました",
        "お届けしました",
        "お届けが済んでおります", // ヤマト: "このお品物はお届けが済んでおります。"
        "お荷物のお届けが完了いたしました", // 佐川急便
    ];
    for kw in delivered_keywords {
        if html.contains(kw) {
            return ParsedStatus {
                check_status: "success",
                delivery_status: "delivered",
                description: Some((*kw).to_string()),
            };
        }
    }

    // --- 配達中（持ち出し）---
    // 注: 「お届け予定日時：」はヤマトの追跡ページに常時表示されるラベルのため除外
    let out_keywords: &[&str] = &["持ち出し中", "配達中"];
    for kw in out_keywords {
        if html.contains(kw) {
            return ParsedStatus {
                check_status: "success",
                delivery_status: "out_for_delivery",
                description: Some((*kw).to_string()),
            };
        }
    }

    // --- 輸送中 ---
    let transit_keywords: &[&str] = &[
        "輸送中",
        "中継",
        "到着",
        "発送しました",
        "集荷",
        "引受",
        "仕分",
    ];
    for kw in transit_keywords {
        if html.contains(kw) {
            return ParsedStatus {
                check_status: "success",
                delivery_status: "in_transit",
                description: Some((*kw).to_string()),
            };
        }
    }

    // --- その他 → shipped のまま更新（last_checked_at のみ更新）---
    ParsedStatus {
        check_status: "success",
        delivery_status: "shipped",
        description: None,
    }
}

// ---------------------------------------------------------------------------
// HTTPリクエスト
// ---------------------------------------------------------------------------

/// reqwest のリダイレクト上限
const MAX_REDIRECTS: u8 = 5;

/// HTML を取得してデコードして返す。
/// `form_body` が Some の場合は POST、None の場合は GET。
/// リダイレクト・タイムアウトは reqwest::Client が処理する。
async fn fetch_html(client: &Client, url: &str, form_body: Option<&str>) -> Result<String, String> {
    let builder = if let Some(body) = form_body {
        client
            .post(url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body.to_string())
    } else {
        client.get(url)
    };

    let resp = builder
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .header("Accept-Language", "ja,en-US;q=0.7,en;q=0.3")
        // 圧縮なしを要求（gzip のまま読むと文字化けするため）
        .header("Accept-Encoding", "identity")
        .send()
        .await
        .map_err(|e| format!("HTTP request error: {e}"))?;

    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        // Drain the body so the underlying connection can be reused.
        let _ = resp.bytes().await;
        return Err(format!("HTTP error: status={status} url={url}"));
    }

    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("none")
        .to_string();

    let body: Bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Body read error: {e}"))?;

    let html = decode_body(&body)?;

    log::debug!(
        "[DeliveryCheck] Response url={url} status={status} \
         content-type={content_type} body_bytes={}",
        body.len(),
    );

    Ok(html)
}

/// HTML の先頭バイトから `<meta charset="...">` を探してラベルを返す。
/// バイト列は ASCII 部分のみ lossy UTF-8 で走査する（日本語部分は不要）。
fn detect_charset(body: &[u8]) -> Option<String> {
    // 先頭 4 KB で十分（<head> は必ずここに収まる）
    let preview = String::from_utf8_lossy(&body[..body.len().min(4096)]);
    let lower = preview.to_ascii_lowercase();
    // charset= の直後を取り出す
    let pos = lower.find("charset=")?;
    let rest = preview[pos + 8..].trim_start_matches(['"', '\'']);
    let end = rest.find(['"', '\'', ';', ' ', '>']).unwrap_or(rest.len());
    let label = rest[..end].trim().to_string();
    if label.is_empty() {
        None
    } else {
        Some(label)
    }
}

/// バイト列を <meta charset> / Content-Type charset に従ってデコードする。
/// 不明な場合は UTF-8 を試し、FFFD が出たら Shift-JIS にフォールバック。
fn decode_body(body: &Bytes) -> Result<String, String> {
    // HTML から charset ラベルを検出して encoding_rs に渡す
    if let Some(label) = detect_charset(body) {
        if let Some(enc) = encoding_rs::Encoding::for_label(label.as_bytes()) {
            let (decoded, _, _) = enc.decode(body);
            return Ok(decoded.into_owned());
        }
    }
    // フォールバック: UTF-8 → Shift-JIS
    let (decoded, _, _) = encoding_rs::UTF_8.decode(body);
    let html = decoded.into_owned();
    if html.contains('\u{FFFD}') {
        let (decoded_sjis, _, _) = encoding_rs::SHIFT_JIS.decode(body);
        return Ok(decoded_sjis.into_owned());
    }
    Ok(html)
}

// ---------------------------------------------------------------------------
// DB 更新ヘルパー
// ---------------------------------------------------------------------------

async fn insert_check_log(
    pool: &SqlitePool,
    tracking_number: &str,
    check_status: &str,
    delivery_status: Option<&str>,
    description: Option<&str>,
    error_message: Option<&str>,
) -> Result<(), String> {
    sqlx::query(
        r#"
        INSERT INTO tracking_check_logs
            (tracking_number, checked_at, check_status, delivery_status, description, error_message)
        VALUES
            (?, CURRENT_TIMESTAMP, ?, ?, ?, ?)
        ON CONFLICT(tracking_number) DO UPDATE SET
            checked_at      = excluded.checked_at,
            check_status    = excluded.check_status,
            delivery_status = excluded.delivery_status,
            description     = excluded.description,
            error_message   = excluded.error_message
        "#,
    )
    .bind(tracking_number)
    .bind(check_status)
    .bind(delivery_status)
    .bind(description)
    .bind(error_message)
    .execute(pool)
    .await
    .map_err(|e| format!("DB insert error: {e}"))?;
    Ok(())
}

async fn update_delivery_status(
    pool: &SqlitePool,
    delivery_id: i64,
    new_status: &str,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE deliveries SET delivery_status = ?, last_checked_at = CURRENT_TIMESTAMP \
         WHERE id = ?",
    )
    .bind(new_status)
    .bind(delivery_id)
    .execute(pool)
    .await
    .map_err(|e| format!("DB update error: {e}"))?;
    Ok(())
}

async fn touch_delivery_last_checked(pool: &SqlitePool, delivery_id: i64) -> Result<(), String> {
    sqlx::query("UPDATE deliveries SET last_checked_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(delivery_id)
        .execute(pool)
        .await
        .map_err(|e| format!("DB touch error: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// BatchTask 実装
// ---------------------------------------------------------------------------

pub struct DeliveryCheckTask;

#[async_trait]
impl BatchTask for DeliveryCheckTask {
    type Input = DeliveryCheckInput;
    type Output = DeliveryCheckOutput;
    type Context = DeliveryCheckContext;

    fn name(&self) -> &str {
        DELIVERY_CHECK_TASK_NAME
    }

    fn event_name(&self) -> &str {
        DELIVERY_CHECK_EVENT_NAME
    }

    async fn process(
        &self,
        input: Self::Input,
        ctx: &Self::Context,
    ) -> Result<Self::Output, String> {
        let delivery_id = input.delivery_id;
        log::info!(
            "[DeliveryCheck] Checking delivery_id={} carrier={} tracking={}",
            delivery_id,
            input.carrier,
            input.tracking_number
        );

        // 追跡URL が構築できない業者はスキップ（check_status = not_found 扱い）
        let Some(url) = build_tracking_url(&input.carrier, &input.tracking_number) else {
            log::warn!(
                "[DeliveryCheck] Unknown carrier: {} (delivery_id={})",
                input.carrier,
                delivery_id
            );
            insert_check_log(
                &ctx.pool,
                &input.tracking_number,
                "not_found",
                Some("delivered"),
                Some("未対応の配送業者"),
                None,
            )
            .await?;
            update_delivery_status(&ctx.pool, delivery_id, "delivered").await?;
            return Ok(DeliveryCheckOutput {
                delivery_id,
                check_status: "not_found".to_string(),
            });
        };

        // ヤマト運輸はフォーム POST（number00=1 が必須）、それ以外は GET
        let form_body = if input.carrier.contains("ヤマト") || input.carrier.contains("クロネコ")
        {
            Some(build_yamato_post_body(&input.tracking_number))
        } else {
            None
        };

        let html = match fetch_html(&ctx.http_client, &url, form_body.as_deref()).await {
            Ok(h) => h,
            Err(e) => {
                log::warn!(
                    "[DeliveryCheck] HTTP error for delivery_id={}: {}",
                    delivery_id,
                    e
                );
                insert_check_log(
                    &ctx.pool,
                    &input.tracking_number,
                    "failed",
                    None,
                    None,
                    Some(&e),
                )
                .await?;
                touch_delivery_last_checked(&ctx.pool, delivery_id).await?;
                return Ok(DeliveryCheckOutput {
                    delivery_id,
                    check_status: "failed".to_string(),
                });
            }
        };

        // HTML 解析
        let parsed = parse_tracking_html(&input.carrier, &html);

        // ログ挿入
        insert_check_log(
            &ctx.pool,
            &input.tracking_number,
            parsed.check_status,
            Some(parsed.delivery_status),
            parsed.description.as_deref(),
            None,
        )
        .await?;

        // deliveries テーブルを更新
        if parsed.delivery_status != "shipped" {
            // shipped のままなら last_checked_at だけ更新（status は変えない）
            update_delivery_status(&ctx.pool, delivery_id, parsed.delivery_status).await?;
        } else {
            touch_delivery_last_checked(&ctx.pool, delivery_id).await?;
        }

        log::info!(
            "[DeliveryCheck] delivery_id={} => check_status={} delivery_status={}",
            delivery_id,
            parsed.check_status,
            parsed.delivery_status,
        );

        Ok(DeliveryCheckOutput {
            delivery_id,
            check_status: parsed.check_status.to_string(),
        })
    }
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_tracking_url_sagawa() {
        let url = build_tracking_url("佐川急便", "123456789").unwrap();
        assert!(url.contains("sagawa-exp.co.jp"));
        assert!(url.contains("123456789"));
    }

    #[test]
    fn test_build_tracking_url_japanpost() {
        let url = build_tracking_url("日本郵便", "987654321").unwrap();
        assert!(url.contains("japanpost.jp"));
    }

    #[test]
    fn test_build_tracking_url_yamato() {
        let url = build_tracking_url("ヤマト運輸", "111222333444").unwrap();
        assert!(url.contains("toi.kuronekoyamato.co.jp"));
        // POST 送信のため URL にはパラメータを付けない
        assert!(!url.contains("number"));
    }

    #[test]
    fn test_build_yamato_post_body() {
        let body = build_yamato_post_body("504160758231");
        assert!(body.contains("number00=1"));
        assert!(body.contains("number01=504160758231"));
    }

    #[test]
    fn test_build_tracking_url_yupacket() {
        let url = build_tracking_url("ゆうパケット", "000111222").unwrap();
        assert!(url.contains("japanpost.jp"));
    }

    #[test]
    fn test_build_tracking_url_unknown() {
        assert!(build_tracking_url("不明業者", "123").is_none());
    }

    #[test]
    fn test_parse_delivered() {
        let html = "<html>お届け済みです</html>";
        let result = parse_tracking_html("佐川急便", html);
        assert_eq!(result.check_status, "success");
        assert_eq!(result.delivery_status, "delivered");
    }

    #[test]
    fn test_parse_not_found() {
        let html = "<html>お荷物データが登録されておりません。</html>";
        let result = parse_tracking_html("佐川急便", html);
        assert_eq!(result.check_status, "not_found");
        assert_eq!(result.delivery_status, "delivered");
    }

    #[test]
    fn test_parse_sagawa_delivered() {
        let html = "<html>お荷物のお届けが完了いたしました</html>";
        let result = parse_tracking_html("佐川急便", html);
        assert_eq!(result.check_status, "success");
        assert_eq!(result.delivery_status, "delivered");
    }

    #[test]
    fn test_parse_in_transit() {
        let html = "<html>輸送中です</html>";
        let result = parse_tracking_html("日本郵便", html);
        assert_eq!(result.check_status, "success");
        assert_eq!(result.delivery_status, "in_transit");
    }

    #[test]
    fn test_parse_out_for_delivery() {
        let html = "<html>配達中です</html>";
        let result = parse_tracking_html("ヤマト運輸", html);
        assert_eq!(result.check_status, "success");
        assert_eq!(result.delivery_status, "out_for_delivery");
    }

    #[test]
    fn test_parse_yamato_not_registered() {
        // 伝票番号未登録 → not_found → delivered 扱い
        let html = "<html><div>伝票番号未登録</div></html>";
        let result = parse_tracking_html("ヤマト運輸", html);
        assert_eq!(result.check_status, "not_found");
        assert_eq!(result.delivery_status, "delivered");
    }

    #[test]
    fn test_parse_default_shipped() {
        let html = "<html>発送いたしました</html>";
        let result = parse_tracking_html("佐川急便", html);
        assert_eq!(result.check_status, "success");
        assert_eq!(result.delivery_status, "shipped");
    }
}
