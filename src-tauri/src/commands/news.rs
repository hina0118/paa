use reqwest;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::time::Duration;
use tauri::Manager;

const MEDIA_NS: &str = "http://search.yahoo.com/mrss/";
/// Dublin Core 名前空間（RDF/RSS 1.0 の dc:date など）
const DC_NS: &str = "http://purl.org/dc/elements/1.1/";
/// 記事本文の Gemini 送信上限文字数
const ARTICLE_CONTENT_LIMIT: usize = 3000;
/// 記事フェッチのタイムアウト秒数
const ARTICLE_FETCH_TIMEOUT_SECS: u64 = 10;
/// Gemini API 呼び出しのタイムアウト秒数
const GEMINI_TIMEOUT_SECS: u64 = 30;

// =============================================================================
// RSS フィード取得
// =============================================================================

#[derive(Debug, Serialize)]
pub struct NewsFeedItem {
    pub id: String,
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub published_at: Option<String>,
    pub thumbnail_url: Option<String>,
}

fn parse_item(item: roxmltree::Node) -> NewsFeedItem {
    let is_atom = item.has_tag_name("entry");

    let child_text = |tag: &str| -> Option<String> {
        item.children()
            .find(|n| n.has_tag_name(tag))
            .and_then(|n| n.text())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let title = child_text("title").unwrap_or_default();

    // Atom: <link href="..."/>、RSS: <link>テキスト</link> or <guid>
    let url = if is_atom {
        item.children()
            .find(|n| {
                n.has_tag_name("link")
                    && n.attribute("rel").map_or(true, |r| r == "alternate")
            })
            .and_then(|n| n.attribute("href"))
            .map(|s| s.to_string())
            .unwrap_or_default()
    } else {
        child_text("link")
            .or_else(|| {
                item.children()
                    .find(|n| n.has_tag_name("guid"))
                    .filter(|n| {
                        n.attribute("isPermaLink")
                            .map(|v| v != "false")
                            .unwrap_or(true)
                    })
                    .and_then(|n| n.text())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_default()
    };

    let id = if is_atom {
        child_text("id").unwrap_or_else(|| url.clone())
    } else {
        child_text("guid").unwrap_or_else(|| url.clone())
    };

    let description = if is_atom {
        child_text("summary").or_else(|| child_text("content"))
    } else {
        child_text("description")
    };

    // 日付: pubDate (RSS2) → dc:date (RDF/RSS1) → published/updated (Atom)
    let published_at = child_text("pubDate")
        .or_else(|| {
            // Dublin Core dc:date（ファミ通・Game Spark・インサイド・Gamer 等）
            item.children()
                .find(|n| {
                    let tag = n.tag_name();
                    tag.namespace() == Some(DC_NS) && tag.name() == "date"
                })
                .and_then(|n| n.text())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .or_else(|| child_text("published"))
        .or_else(|| child_text("updated"));

    let thumbnail_url = item
        .children()
        .find(|n| {
            let tag = n.tag_name();
            tag.namespace() == Some(MEDIA_NS)
                && (tag.name() == "thumbnail" || tag.name() == "content")
        })
        .and_then(|n| n.attribute("url"))
        .map(|s| s.to_string())
        .or_else(|| {
            item.children()
                .find(|n| {
                    n.has_tag_name("enclosure")
                        && n.attribute("type")
                            .map(|t| t.starts_with("image/"))
                            .unwrap_or(false)
                })
                .and_then(|n| n.attribute("url"))
                .map(|s| s.to_string())
        });

    NewsFeedItem {
        id,
        title,
        url,
        description,
        published_at,
        thumbnail_url,
    }
}

/// RSS/Atom フィードを取得してパースする Tauri コマンド
#[tauri::command]
pub async fn fetch_news_feed(url: String) -> Result<Vec<NewsFeedItem>, String> {

    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("フィードの取得に失敗しました: {e}"))?;

    let text = response
        .text()
        .await
        .map_err(|e| format!("レスポンスの読み取りに失敗しました: {e}"))?;

    let doc = roxmltree::Document::parse(&text)
        .map_err(|e| format!("XMLの解析に失敗しました: {e}"))?;

    // RSS 2.0/1.0 は <item>、Atom は <entry>
    let items = doc
        .descendants()
        .filter(|n| n.has_tag_name("item") || n.has_tag_name("entry"))
        .map(parse_item)
        .collect();

    Ok(items)
}

// =============================================================================
// HTML スクレイピングによるニュース取得
// =============================================================================

/// HTML スクレイピング用セレクタ設定（フロントエンドから受け取る）
#[derive(Debug, Deserialize)]
pub struct HtmlScrapeSelectors {
    pub item: String,
    pub title: Option<String>,
    pub thumbnail: Option<String>,
    pub date: Option<String>,
}

/// HTML ページをスクレイピングしてニュース記事を取得する Tauri コマンド
/// RSS 非対応サイト（GameWith 等）への対応に使用
#[tauri::command]
pub async fn fetch_news_html(
    url: String,
    selectors: HtmlScrapeSelectors,
) -> Result<Vec<NewsFeedItem>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(ARTICLE_FETCH_TIMEOUT_SECS))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .map_err(|e| format!("HTTPクライアントの初期化に失敗: {e}"))?;

    let html = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("ページの取得に失敗しました: {e}"))?
        .text()
        .await
        .map_err(|e| format!("レスポンスの読み取りに失敗しました: {e}"))?;

    let document = Html::parse_document(&html);

    let item_sel = Selector::parse(&selectors.item)
        .map_err(|_| format!("無効なCSSセレクタ: {}", selectors.item))?;
    let title_sel = selectors.title.as_deref().and_then(|s| Selector::parse(s).ok());
    let thumb_sel = selectors
        .thumbnail
        .as_deref()
        .and_then(|s| Selector::parse(s).ok());
    let date_sel = selectors.date.as_deref().and_then(|s| Selector::parse(s).ok());

    // 相対 URL 解決用のベース URL
    let base = url::Url::parse(&url).ok();

    let items: Vec<NewsFeedItem> = document
        .select(&item_sel)
        .filter_map(|el| {
            // リンク URL: item 要素の href 属性
            let href = el.value().attr("href").map(|s| s.to_string())?;
            let item_url = base
                .as_ref()
                .and_then(|b| b.join(&href).ok())
                .map(|u| u.to_string())
                .unwrap_or(href);

            // タイトル: title_selector があればその要素のテキスト、なければ item のテキスト
            let title = if let Some(ref sel) = title_sel {
                el.select(sel)
                    .next()
                    .map(|t| t.text().collect::<String>().trim().to_string())
            } else {
                // 日本語日付パターン（"2026年4月5日"）を除去してタイトルを抽出
                let raw = el
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                let cleaned = regex::Regex::new(r"\d{4}年\d{1,2}月\d{1,2}日[\d:]*")
                    .ok()
                    .map(|re| re.replace_all(&raw, "").trim().to_string())
                    .unwrap_or(raw);
                Some(cleaned)
            }
            .filter(|s| !s.is_empty())?;

            // サムネイル: thumb_selector があればその要素の src / data-src 属性
            let thumbnail_url = thumb_sel.as_ref().and_then(|sel| {
                el.select(sel).next().and_then(|img| {
                    img.value()
                        .attr("src")
                        .or_else(|| img.value().attr("data-src"))
                        .map(|s| {
                            base.as_ref()
                                .and_then(|b| b.join(s).ok())
                                .map(|u| u.to_string())
                                .unwrap_or_else(|| s.to_string())
                        })
                })
            });

            // 日付: date_selector があればその要素のテキスト
            let published_at = date_sel.as_ref().and_then(|sel| {
                el.select(sel)
                    .next()
                    .map(|d| d.text().collect::<String>().trim().to_string())
                    .filter(|s| !s.is_empty())
            });

            Some(NewsFeedItem {
                id: item_url.clone(),
                title,
                url: item_url,
                description: None,
                published_at,
                thumbnail_url,
            })
        })
        .collect();

    Ok(items)
}

// =============================================================================
// クリップ機能
// =============================================================================

#[derive(Debug, Serialize)]
pub struct NewsClip {
    pub id: i64,
    pub title: String,
    pub url: String,
    pub source_name: String,
    pub published_at: Option<String>,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub clipped_at: String,
}

/// DB の生行をドメイン型に変換するヘルパー
fn row_to_clip(
    id: i64,
    title: String,
    url: String,
    source_name: String,
    published_at: Option<String>,
    summary: Option<String>,
    tags_json: String,
    clipped_at: String,
) -> NewsClip {
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    NewsClip {
        id,
        title,
        url,
        source_name,
        published_at,
        summary,
        tags,
        clipped_at,
    }
}

// -------------------------------------------------------
// 記事本文の取得（HTML → プレーンテキスト抽出）
// -------------------------------------------------------

/// 記事URLから本文テキストを取得する。失敗した場合は None を返す
async fn fetch_article_content(url: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(ARTICLE_FETCH_TIMEOUT_SECS))
        .user_agent("Mozilla/5.0")
        .build()
        .ok()?;

    let html = client.get(url).send().await.ok()?.text().await.ok()?;
    let document = Html::parse_document(&html);

    // コンテンツが入りやすいセレクタを順に試す
    let candidate_selectors = [
        "article",
        "main",
        ".entry-content",
        ".post-content",
        ".article-body",
        "#content",
    ];

    for sel_str in &candidate_selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            let text: String = document
                .select(&sel)
                .flat_map(|el| el.text())
                .collect::<Vec<_>>()
                .join(" ");
            let trimmed = text.split_whitespace().collect::<Vec<_>>().join(" ");
            if trimmed.len() > 100 {
                return Some(trimmed.chars().take(ARTICLE_CONTENT_LIMIT).collect());
            }
        }
    }

    // フォールバック: <p> タグ全文
    if let Ok(p_sel) = Selector::parse("p") {
        let text: String = document
            .select(&p_sel)
            .flat_map(|el| el.text())
            .collect::<Vec<_>>()
            .join(" ");
        let trimmed = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if !trimmed.is_empty() {
            return Some(trimmed.chars().take(ARTICLE_CONTENT_LIMIT).collect());
        }
    }

    None
}

// -------------------------------------------------------
// Gemini による要約・タグ抽出
// -------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GeminiSummaryResult {
    summary: String,
    tags: Vec<String>,
}

/// Gemini API を呼び出して記事の要約とタグを生成する
async fn summarize_with_gemini(
    api_key: &str,
    title: &str,
    content: &str,
) -> Result<(String, Vec<String>), String> {
    let prompt = format!(
        "以下のゲーム・ホビー系ニュース記事を分析してください。\n\nタイトル: {title}\n\n本文:\n{content}\n\n次のJSON形式のみで回答してください（余分なテキスト不要）:\n{{\"summary\":\"記事の要約（100〜150文字）\",\"tags\":[\"タグ1\",\"タグ2\",\"タグ3\",\"タグ4\",\"タグ5\"]}}\n\nsummaryは日本語で簡潔に。tagsはゲームタイトル・シリーズ・ジャンル・ハード名・会社名などを5個程度。"
    );

    let body = serde_json::json!({
        "contents": [{"parts": [{"text": prompt}]}],
        "generationConfig": {
            "responseMimeType": "application/json",
            "temperature": 0.1,
            "maxOutputTokens": 512
        }
    })
    .to_string();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(GEMINI_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("HTTPクライアントの初期化に失敗: {e}"))?;

    let response = client
        .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-lite:generateContent")
        .header("Content-Type", "application/json")
        .header("X-goog-api-key", api_key)
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Gemini APIへの接続に失敗しました: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        return Err(format!(
            "Gemini API エラー (status {status})。APIキーと利用制限をご確認ください。"
        ));
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Gemini レスポンスの読み取りに失敗: {e}"))?;

    // Gemini レスポンス構造: candidates[0].content.parts[0].text
    let gemini_resp: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Gemini レスポンスのパースに失敗: {e}"))?;

    let text = gemini_resp["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| "Gemini レスポンスからテキストを取得できませんでした".to_string())?;

    let result: GeminiSummaryResult = serde_json::from_str(text)
        .map_err(|e| format!("AIレスポンスのJSONパースに失敗: {e}"))?;

    Ok((result.summary, result.tags))
}

// -------------------------------------------------------
// Tauri コマンド
// -------------------------------------------------------

/// 記事をクリップ保存する。AI要約とタグを生成してDBに格納する
#[tauri::command]
pub async fn clip_news_article(
    pool: tauri::State<'_, SqlitePool>,
    app_handle: tauri::AppHandle,
    url: String,
    title: String,
    source_name: String,
    published_at: Option<String>,
    description: Option<String>,
) -> Result<NewsClip, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("アプリデータディレクトリの取得に失敗: {e}"))?;

    let api_key = crate::gemini::config::load_api_key(&app_data_dir)?;

    // 記事本文を取得。失敗時は RSS description にフォールバック
    let content = fetch_article_content(&url)
        .await
        .or(description)
        .unwrap_or_else(|| title.clone());

    let (summary, tags) = summarize_with_gemini(&api_key, &title, &content).await?;

    let tags_json =
        serde_json::to_string(&tags).map_err(|e| format!("タグのシリアライズに失敗: {e}"))?;

    let row: (i64, String, String, String, Option<String>, Option<String>, String, String) =
        sqlx::query_as(
            "INSERT INTO news_clips (title, url, source_name, published_at, summary, tags)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(url) DO UPDATE SET
               summary = excluded.summary,
               tags    = excluded.tags
             RETURNING id, title, url, source_name, published_at, summary, tags, clipped_at",
        )
        .bind(&title)
        .bind(&url)
        .bind(&source_name)
        .bind(&published_at)
        .bind(&summary)
        .bind(&tags_json)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| format!("クリップの保存に失敗しました: {e}"))?;

    Ok(row_to_clip(row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7))
}

/// クリップ一覧を clipped_at 降順で返す
#[tauri::command]
pub async fn get_news_clips(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<NewsClip>, String> {
    let rows: Vec<(i64, String, String, String, Option<String>, Option<String>, String, String)> =
        sqlx::query_as(
            "SELECT id, title, url, source_name, published_at, summary, tags, clipped_at
             FROM news_clips
             ORDER BY clipped_at DESC",
        )
        .fetch_all(pool.inner())
        .await
        .map_err(|e| format!("クリップの取得に失敗しました: {e}"))?;

    Ok(rows
        .into_iter()
        .map(|r| row_to_clip(r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7))
        .collect())
}

/// クリップを削除する
#[tauri::command]
pub async fn delete_news_clip(
    pool: tauri::State<'_, SqlitePool>,
    id: i64,
) -> Result<(), String> {
    sqlx::query("DELETE FROM news_clips WHERE id = ?")
        .bind(id)
        .execute(pool.inner())
        .await
        .map_err(|e| format!("クリップの削除に失敗しました: {e}"))?;
    Ok(())
}

/// クリップ済み URL の一覧を返す（ニュース一覧でのクリップ済み判定用）
#[tauri::command]
pub async fn get_clipped_urls(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<String>, String> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT url FROM news_clips")
        .fetch_all(pool.inner())
        .await
        .map_err(|e| format!("クリップURLの取得に失敗しました: {e}"))?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}
