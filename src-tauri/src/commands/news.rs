use reqwest;
use serde::Serialize;

const MEDIA_NS: &str = "http://search.yahoo.com/mrss/";

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
    // テキスト子ノードを取得するユーティリティ
    let child_text = |tag: &str| -> Option<String> {
        item.children()
            .find(|n| n.has_tag_name(tag))
            .and_then(|n| n.text())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let title = child_text("title").unwrap_or_default();

    // URL: <link> → <guid isPermaLink="true"> → <guid> の順で探す
    let url = child_text("link")
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
        .unwrap_or_default();

    let id = child_text("guid").unwrap_or_else(|| url.clone());
    let description = child_text("description");
    let published_at = child_text("pubDate");

    // サムネイル: <media:thumbnail> → <media:content> → <enclosure type="image/*"> の順
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

    let items = doc
        .descendants()
        .filter(|n| n.has_tag_name("item"))
        .map(parse_item)
        .collect();

    Ok(items)
}
