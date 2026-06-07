/// PAA MCP SSE サーバー
///
/// Tauri アプリと同一プロセス内で起動する HTTP サーバー。
/// MCP の SSE トランスポート（2024-11-05）を実装し、以下の2ツールを公開する。
///   - search_products   : product_master テーブルをフリーテキスト検索
///   - search_news_clips : news_clips テーブルをフリーテキスト検索
///
/// エンドポイント:
///   GET  http://localhost:4119/sse      ← LLM が最初に接続する SSE ストリーム
///   POST http://localhost:4119/messages ← JSON-RPC リクエストの送信先
///
/// Claude Desktop の設定例:
/// {
///   "mcpServers": {
///     "paa": {
///       "type": "sse",
///       "url": "http://localhost:4119/sse"
///     }
///   }
/// }
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use futures::StreamExt;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;

pub const MCP_PORT: u16 = 4119;

// =============================================================================
// サーバー状態
// =============================================================================

#[derive(Clone)]
pub struct McpState {
    pool: sqlx::SqlitePool,
    app_handle: tauri::AppHandle,
    /// セッション ID → SSE レスポンスチャンネル
    sessions: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
}

// =============================================================================
// 起動エントリーポイント（lib.rs::setup() から呼ばれる）
// =============================================================================

pub async fn start_sse_server(pool: sqlx::SqlitePool, app_handle: tauri::AppHandle) {
    let state = McpState {
        pool,
        app_handle,
        sessions: Arc::new(Mutex::new(HashMap::new())),
    };

    let router = Router::new()
        .route("/sse", get(handle_sse))
        .route("/messages", post(handle_message))
        .with_state(state);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], MCP_PORT));

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => {
            log::info!("[mcp-sse] 起動しました → http://{addr}/sse");
            l
        }
        Err(e) => {
            log::error!("[mcp-sse] ポート {MCP_PORT} のバインドに失敗しました: {e}");
            return;
        }
    };

    if let Err(e) = axum::serve(listener, router).await {
        log::error!("[mcp-sse] サーバーエラー: {e}");
    }
}

// =============================================================================
// SSE 接続ハンドラ（GET /sse）
// =============================================================================

async fn handle_sse(
    State(state): State<McpState>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let session_id = Uuid::new_v4().to_string();
    let (tx, rx) = mpsc::unbounded_channel::<String>();

    state.sessions.lock().await.insert(session_id.clone(), tx);

    log::info!("[mcp-sse] 新しいセッション: {session_id}");

    // 1. まず endpoint イベントを送って POST 先を教える
    let endpoint_url = format!("/messages?sessionId={session_id}");
    let endpoint_event = futures::stream::once(futures::future::ready(Ok::<Event, Infallible>(
        Event::default().event("endpoint").data(endpoint_url),
    )));

    // 2. 以降は JSON-RPC レスポンスを message イベントとして流す
    let message_stream = UnboundedReceiverStream::new(rx)
        .map(|data| Ok::<Event, Infallible>(Event::default().event("message").data(data)));

    Sse::new(endpoint_event.chain(message_stream)).keep_alive(KeepAlive::default())
}

// =============================================================================
// メッセージ受信ハンドラ（POST /messages）
// =============================================================================

#[derive(serde::Deserialize)]
struct MessageQuery {
    #[serde(rename = "sessionId")]
    session_id: String,
}

async fn handle_message(
    Query(q): Query<MessageQuery>,
    State(state): State<McpState>,
    Json(req): Json<Value>,
) -> impl IntoResponse {
    // notification（id なし）はレスポンス不要なので即 202 を返す
    let has_id = req.get("id").is_some();

    if has_id {
        if let Some(resp) = dispatch(&state, req).await {
            let sessions = state.sessions.lock().await;
            match sessions.get(&q.session_id) {
                Some(tx) => {
                    if tx.send(resp.to_string()).is_err() {
                        log::warn!(
                            "[mcp-sse] セッション {} の送信に失敗（切断済み？）",
                            q.session_id
                        );
                    }
                }
                None => {
                    log::warn!("[mcp-sse] セッション {} が見つかりません", q.session_id);
                    return StatusCode::NOT_FOUND;
                }
            }
        }
    }

    // MCP SSE 仕様: HTTP レスポンスは常に 202 Accepted（本文なし）
    StatusCode::ACCEPTED
}

// =============================================================================
// JSON-RPC ディスパッチ
// =============================================================================

async fn dispatch(state: &McpState, req: Value) -> Option<Value> {
    let id = req.get("id").cloned()?;
    let method = req["method"].as_str().unwrap_or("");

    match method {
        "initialize" => Some(ok(
            &id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "paa-mcp-server", "version": "0.1.0" }
            }),
        )),

        "ping" => Some(ok(&id, json!({}))),

        "tools/list" => Some(ok(&id, json!({ "tools": tool_definitions() }))),

        "tools/call" => {
            let name = req["params"]["name"].as_str().unwrap_or("");
            let args = &req["params"]["arguments"];

            // UI に「AI が検索中」を通知
            state
                .app_handle
                .emit(
                    "mcp-tool-call",
                    json!({ "tool": name, "query": args["query"] }),
                )
                .ok();

            let result = run_tool(&state.pool, name, args).await;

            // UI に完了を通知
            state
                .app_handle
                .emit("mcp-tool-done", json!({ "tool": name }))
                .ok();

            Some(result.map_or_else(
                |e| err(&id, -32603, &e),
                |text| {
                    ok(
                        &id,
                        json!({ "content": [{ "type": "text", "text": text }] }),
                    )
                },
            ))
        }

        _ => Some(err(&id, -32601, "Method not found")),
    }
}

// =============================================================================
// ツール実行
// =============================================================================

async fn run_tool(pool: &sqlx::SqlitePool, name: &str, args: &Value) -> Result<String, String> {
    match name {
        "search_products" => {
            let query = args["query"].as_str().ok_or("query is required")?;
            let limit = args["limit"].as_i64().unwrap_or(20).min(100);
            let items = search_products(pool, query, limit).await?;
            serde_json::to_string_pretty(&items).map_err(|e| e.to_string())
        }
        "search_news_clips" => {
            let query = args["query"].as_str().ok_or("query is required")?;
            let limit = args["limit"].as_i64().unwrap_or(20).min(100);
            let items = search_news_clips(pool, query, limit).await?;
            serde_json::to_string_pretty(&items).map_err(|e| e.to_string())
        }
        "get_all_products" => {
            let items = get_all_products(pool).await?;
            serde_json::to_string_pretty(&items).map_err(|e| e.to_string())
        }
        "add_exclusion_pattern" => {
            let keyword = args["keyword"].as_str().ok_or("keyword is required")?;
            let match_type = args["match_type"].as_str().unwrap_or("contains");
            let shop_domain = args["shop_domain"].as_str().map(|s| s.to_string());
            let note = args["note"].as_str().map(|s| s.to_string());
            let id = add_exclusion_pattern(
                pool,
                shop_domain,
                keyword.to_string(),
                match_type.to_string(),
                note,
            )
            .await?;
            Ok(format!("{{\"id\": {id}}}"))
        }
        _ => Err(format!("Unknown tool: {name}")),
    }
}

// =============================================================================
// ツール定義
// =============================================================================

fn tool_definitions() -> Value {
    json!([
        {
            "name": "search_products",
            "description": "購入済み商品（注文明細）をフリーテキストで検索します。商品名が対象です。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "検索キーワード（商品名・メーカー・シリーズなど）" },
                    "limit": { "type": "integer", "description": "最大取得件数（デフォルト: 20）" }
                },
                "required": ["query"]
            }
        },
        {
            "name": "get_all_products",
            "description": "注文明細（購入済み商品）の全件を取得します。商品名・価格・数量・ショップ・注文日を返します。",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "add_exclusion_pattern",
            "description": "除外キーワードを追加します。指定したキーワードにマッチする商品名は注文取り込み時に除外されます。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": { "type": "string", "description": "除外キーワード" },
                    "match_type": { "type": "string", "enum": ["contains", "starts_with", "exact"], "description": "マッチ方式（デフォルト: contains）" },
                    "shop_domain": { "type": "string", "description": "適用するショップドメイン（省略時は全ショップ）" },
                    "note": { "type": "string", "description": "メモ" }
                },
                "required": ["keyword"]
            }
        },
        {
            "name": "search_news_clips",
            "description": "クリップ済みニュース記事をフリーテキストで検索します。タイトル・要約・タグが対象です。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "検索キーワード（タイトル・要約・タグなど）" },
                    "limit": { "type": "integer", "description": "最大取得件数（デフォルト: 20）" }
                },
                "required": ["query"]
            }
        }
    ])
}

// =============================================================================
// JSON-RPC ヘルパー
// =============================================================================

fn ok(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn err(id: &Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

// =============================================================================
// DB クエリ
// =============================================================================

#[derive(Debug, Serialize, sqlx::FromRow)]
struct OrderItem {
    id: i64,
    item_name: String,
    price: i64,
    quantity: i64,
    shop_domain: String,
    shop_name: String,
    order_number: String,
    order_date: String,
    brand: Option<String>,
    category: Option<String>,
    maker: Option<String>,
    series: Option<String>,
    product_name: Option<String>,
    scale: Option<String>,
    is_reissue: Option<bool>,
    delivery_status: Option<String>,
}

/// UIの商品一覧と同等のベースクエリ（WITH句 + JOIN群）
///
/// - excluded_items / excluded_orders を除外
/// - item_overrides / order_overrides のオーバーライドを反映
/// - product_master の解析結果（maker / series 等）を付加
const ORDER_ITEMS_BASE_SQL: &str = "
    WITH latest_delivery AS (
      SELECT order_id, delivery_status
      FROM (
        SELECT order_id, delivery_status,
               ROW_NUMBER() OVER (PARTITION BY order_id ORDER BY updated_at DESC) AS rn
        FROM deliveries
      ) t
      WHERE rn = 1
    )
    SELECT
      i.id,
      COALESCE(io.item_name, i.item_name)           AS item_name,
      COALESCE(io.price, i.price)                    AS price,
      COALESCE(io.quantity, i.quantity)              AS quantity,
      o.shop_domain,
      COALESCE(oo.shop_name, o.shop_name)            AS shop_name,
      COALESCE(oo.new_order_number, o.order_number)  AS order_number,
      COALESCE(oo.order_date, o.order_date)          AS order_date,
      NULLIF(COALESCE(io.brand, i.brand, ''), '')    AS brand,
      COALESCE(io.category, i.category)              AS category,
      pm.maker,
      pm.series,
      pm.product_name,
      pm.scale,
      pm.is_reissue,
      ld.delivery_status
    FROM items i
    JOIN orders o ON i.order_id = o.id
    LEFT JOIN latest_delivery ld ON ld.order_id = o.id
    LEFT JOIN product_master pm ON i.item_name_normalized = pm.normalized_name
    LEFT JOIN item_overrides io
          ON io.shop_domain = o.shop_domain
         AND io.order_number COLLATE NOCASE = o.order_number
         AND io.original_item_name = i.item_name
         AND io.original_brand = COALESCE(i.brand, '')
    LEFT JOIN order_overrides oo
          ON oo.shop_domain = o.shop_domain
         AND oo.order_number COLLATE NOCASE = o.order_number
    LEFT JOIN excluded_items ei
          ON ei.shop_domain = o.shop_domain
         AND ei.order_number COLLATE NOCASE = o.order_number
         AND ei.item_name = i.item_name
         AND ei.brand = COALESCE(i.brand, '')
    LEFT JOIN excluded_orders eo
          ON eo.shop_domain = o.shop_domain
         AND eo.order_number COLLATE NOCASE = o.order_number
    WHERE ei.id IS NULL AND eo.id IS NULL
";

async fn search_products(
    pool: &sqlx::SqlitePool,
    query: &str,
    limit: i64,
) -> Result<Vec<OrderItem>, String> {
    let like = format!("%{query}%");
    let sql = format!(
        "{ORDER_ITEMS_BASE_SQL}
         AND COALESCE(io.item_name, i.item_name) LIKE ?
         ORDER BY COALESCE(oo.order_date, o.order_date) DESC LIMIT ?"
    );
    sqlx::query_as::<_, OrderItem>(&sql)
        .bind(&like)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("DB error: {e}"))
}

#[derive(Debug, Serialize)]
struct NewsClipItem {
    id: i64,
    title: String,
    url: String,
    source_name: String,
    published_at: Option<String>,
    summary: Option<String>,
    tags: Vec<String>,
    clipped_at: String,
}

type NewsClipRow = (
    i64,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
);

async fn add_exclusion_pattern(
    pool: &sqlx::SqlitePool,
    shop_domain: Option<String>,
    keyword: String,
    match_type: String,
    note: Option<String>,
) -> Result<i64, String> {
    let repo = crate::repository::SqliteExclusionPatternRepository::new(pool.clone());
    repo.add(shop_domain, keyword, match_type, note).await
}

async fn get_all_products(pool: &sqlx::SqlitePool) -> Result<Vec<OrderItem>, String> {
    let sql = format!(
        "{ORDER_ITEMS_BASE_SQL}
         ORDER BY COALESCE(oo.order_date, o.order_date) DESC"
    );
    sqlx::query_as::<_, OrderItem>(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("DB error: {e}"))
}

async fn search_news_clips(
    pool: &sqlx::SqlitePool,
    query: &str,
    limit: i64,
) -> Result<Vec<NewsClipItem>, String> {
    let like = format!("%{query}%");
    let rows: Vec<NewsClipRow> = sqlx::query_as(
        "SELECT id, title, url, source_name, published_at, summary, tags, clipped_at
         FROM news_clips
         WHERE title LIKE ? OR summary LIKE ? OR tags LIKE ?
         ORDER BY clipped_at DESC LIMIT ?",
    )
    .bind(&like)
    .bind(&like)
    .bind(&like)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    Ok(rows
        .into_iter()
        .map(|r| NewsClipItem {
            id: r.0,
            title: r.1,
            url: r.2,
            source_name: r.3,
            published_at: r.4,
            summary: r.5,
            tags: serde_json::from_str(&r.6).unwrap_or_default(),
            clipped_at: r.7,
        })
        .collect())
}
