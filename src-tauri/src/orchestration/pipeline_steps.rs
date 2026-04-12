//! パイプライン共通ステップ関数。
//!
//! スケジューラ用 [`pipeline_orchestrator`] と UI用パイプライン（#290）の
//! 両方から再利用できるよう `pub(crate)` で公開している。
//!
//! ## ステップ一覧
//!
//! | 関数 | 概要 | 利用先 |
//! |------|------|--------|
//! | `run_sync_step` | Gmail 差分同期 | スケジューラのみ |
//! | `run_parse_step` | メールパース | スケジューラ・UI |
//! | `run_surugaya_step` | 駿河屋 HTML パース | UI のみ |
//! | `run_product_parse_step` | 商品名パース | スケジューラ・UI |
//! | `run_delivery_check_step` | 配送状況確認 | スケジューラ・UI |

use sqlx::sqlite::SqlitePool;
use tauri::Manager;

use super::clamp_batch_size;

// ─────────────────────────────────────────────────────────────────────────────
// StepOutcome
// ─────────────────────────────────────────────────────────────────────────────

/// ステップの実行結果
#[derive(Debug)]
pub(crate) enum StepOutcome {
    /// ステップを実行し、新規データ数を返す
    Ran { new_count: i64 },
    /// 手動実行中・前提条件不足などでスキップされた（未実行）
    Skipped,
    /// ステップは実行済みだが、件数取得に失敗して新規データ数が不明
    Unknown,
}

// ─────────────────────────────────────────────────────────────────────────────
// ステップ関数
// ─────────────────────────────────────────────────────────────────────────────

/// 差分同期を実行し、新規メール件数を返す。
/// 手動同期中の場合は Skipped を返す。認証未設定の場合は new_count = 0 として扱う。
///
/// Note: `SyncState::try_start()` をここで直接呼び出すことで、
/// 既に実行中の場合はエラーイベントを emit せずに静かにスキップできる。
/// `run_incremental_sync_task` には `caller_did_try_start = true` を渡し、
/// 内部での二重 `try_start()` を防ぐ。
pub(crate) async fn run_sync_step(app: &tauri::AppHandle, pool: &SqlitePool) -> StepOutcome {
    use crate::gmail::SyncState;

    // OAuth認証情報が未設定なら同期自体が実行できず、新規メールも増えないことが確定
    let has_credentials = app
        .path()
        .app_data_dir()
        .map(|dir| crate::gmail::has_oauth_credentials(&dir))
        .unwrap_or(false);
    if !has_credentials {
        log::info!("[Pipeline] Gmail OAuth credentials not configured, treating as no new emails");
        return StepOutcome::Ran { new_count: 0 };
    }

    let sync_state = match app.try_state::<SyncState>() {
        Some(s) => s.inner().clone(),
        None => {
            log::warn!("[Pipeline] SyncState not available, skipping sync");
            return StepOutcome::Skipped;
        }
    };

    // 先に try_start で「すでに同期中なら即スキップ」し、無駄な COUNT クエリを避ける。
    if !sync_state.try_start() {
        log::info!(
            "[Pipeline] Failed to start sync (already running or failed to acquire state lock), skipping"
        );
        return StepOutcome::Skipped;
    }

    // try_start 成功後は is_running フラグが必ず解除されるよう、
    // before カウント失敗では早期 return せず 0 をデフォルトとする。
    let before: i64 = count_emails(pool).await.unwrap_or_default();

    log::info!("[Pipeline] Step 1/4: incremental sync");
    super::run_incremental_sync_task(app.clone(), pool.clone(), sync_state, true).await;
    log::info!("[Pipeline] Step 1/4: incremental sync completed");

    let after = match count_emails(pool).await {
        Some(n) => n,
        None => return StepOutcome::Unknown,
    };

    StepOutcome::Ran {
        new_count: after.saturating_sub(before),
    }
}

/// メールパースを実行し、新規注文件数を返す。
/// 手動パース中の場合は Skipped を返す。
pub(crate) async fn run_parse_step(app: &tauri::AppHandle, pool: &SqlitePool) -> StepOutcome {
    use crate::parsers::ParseState;

    let parse_state = match app.try_state::<ParseState>() {
        Some(s) => s.inner().clone(),
        None => {
            log::warn!("[Pipeline] ParseState not available, skipping parse");
            return StepOutcome::Skipped;
        }
    };

    if parse_state.is_running() {
        log::info!("[Pipeline] Parse already running, skipping");
        return StepOutcome::Skipped;
    }

    let batch_size = load_parse_batch_size(app);
    let before = match count_orders(pool).await {
        Some(n) => n,
        None => return StepOutcome::Skipped,
    };
    log::info!("[Pipeline] Batch parse (batch_size={})", batch_size);
    super::run_batch_parse_task(app.clone(), pool.clone(), parse_state, batch_size).await;
    log::info!("[Pipeline] Batch parse completed");

    let after = match count_orders(pool).await {
        Some(n) => n,
        None => return StepOutcome::Unknown,
    };

    StepOutcome::Ran {
        new_count: after.saturating_sub(before),
    }
}

/// 駿河屋マイページ HTML フェッチステップ（差分取得）。
/// `surugaya-session` ウィンドウが開いていない場合は Skipped を返す。
/// パース処理は `run_parse_step` 内の `run_surugaya_html_parse_step` で行う。
#[allow(dead_code)]
pub(crate) async fn run_surugaya_step(app: &tauri::AppHandle, pool: &SqlitePool) -> StepOutcome {
    use crate::commands::{surugaya_session, SurugayaSessionState};
    use tauri::Emitter;

    // surugaya-session ウィンドウが開いていない（ログイン未済み）場合はスキップ
    let win = match app.get_webview_window("surugaya-session") {
        Some(w) => w,
        None => {
            log::info!("[Pipeline] Surugaya session window not open, skipping");
            return StepOutcome::Skipped;
        }
    };

    let session_state = match app.try_state::<SurugayaSessionState>() {
        Some(s) => s.inner().clone(),
        None => {
            log::warn!("[Pipeline] SurugayaSessionState not available, skipping");
            return StepOutcome::Skipped;
        }
    };

    if let Err(e) = session_state.try_start() {
        log::info!("[Pipeline] Surugaya fetch already running, skipping: {e}");
        return StepOutcome::Skipped;
    }

    log::info!("[Pipeline] Surugaya mypage fetch step (diff only)");
    // force_refetch = false: 差分取得のみ（パイプラインは効率優先）
    let result = surugaya_session::run_mypage_batch(app, pool, &win, &session_state, false).await;
    session_state.finish();

    let (cancelled, error) = match result {
        Ok(cancelled) => (cancelled, None),
        Err(e) => {
            log::warn!("[Pipeline] Surugaya fetch failed: {e}");
            (false, Some(e))
        }
    };
    let _ = app.emit(
        "surugaya:fetch_complete",
        serde_json::json!({ "cancelled": cancelled, "error": error }),
    );

    log::info!("[Pipeline] Surugaya mypage fetch step completed");
    StepOutcome::Unknown
}

/// 商品名パースを実行する。
/// 手動実行中の場合はスキップする。
pub(crate) async fn run_product_parse_step(app: &tauri::AppHandle, pool: &SqlitePool) {
    use crate::commands::ProductNameParseState;

    let parse_state = match app.try_state::<ProductNameParseState>() {
        Some(s) => s.inner().clone(),
        None => {
            log::warn!("[Pipeline] ProductNameParseState not available, skipping product parse");
            return;
        }
    };

    if let Err(e) = parse_state.try_start() {
        log::info!("[Pipeline] Product name parse already running, skipping: {e}");
        return;
    }

    log::info!("[Pipeline] Product name parse step");
    super::run_product_name_parse_task(app.clone(), pool.clone(), parse_state, true).await;
    log::info!("[Pipeline] Product name parse step completed");
}

/// 配送状況確認を実行する。
/// 手動実行中の場合はスキップする。
pub(crate) async fn run_delivery_check_step(app: &tauri::AppHandle, pool: &SqlitePool) {
    use crate::commands::DeliveryCheckState;

    let check_state = match app.try_state::<DeliveryCheckState>() {
        Some(s) => s.inner().clone(),
        None => {
            log::warn!("[Pipeline] DeliveryCheckState not available, skipping delivery check");
            return;
        }
    };

    if let Err(e) = check_state.try_start() {
        log::info!("[Pipeline] Delivery check already running, skipping: {e}");
        return;
    }

    log::info!("[Pipeline] Delivery check step");
    super::run_delivery_check_task(app.clone(), pool.clone(), check_state).await;
    log::info!("[Pipeline] Delivery check step completed");
}

// ─────────────────────────────────────────────────────────────────────────────
// ヘルパー関数
// ─────────────────────────────────────────────────────────────────────────────

async fn count_table(pool: &SqlitePool, table: &str) -> Option<i64> {
    let query = format!("SELECT COUNT(*) FROM {table}");
    match sqlx::query_scalar::<_, i64>(&query).fetch_one(pool).await {
        Ok(count) => Some(count),
        Err(e) => {
            log::error!("[Pipeline] Failed to count {table}: {e}");
            None
        }
    }
}

pub(crate) async fn count_emails(pool: &SqlitePool) -> Option<i64> {
    count_table(pool, "emails").await
}

pub(crate) async fn count_orders(pool: &SqlitePool) -> Option<i64> {
    count_table(pool, "orders").await
}

pub(crate) fn load_parse_batch_size(app: &tauri::AppHandle) -> usize {
    let config_dir = match app.path().app_config_dir() {
        Ok(dir) => dir,
        Err(e) => {
            log::warn!("[Pipeline] Failed to get app_config_dir: {e}, using default batch_size");
            return 100;
        }
    };
    match crate::config::load(&config_dir) {
        Ok(c) => clamp_batch_size(c.parse.batch_size, 100),
        Err(e) => {
            log::warn!("[Pipeline] Failed to load config: {e}, using default batch_size");
            100
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// テスト
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::test_helpers::create_pool;

    #[tokio::test]
    async fn count_emails_returns_none_when_table_missing() {
        let pool = create_pool().await;
        assert!(count_emails(&pool).await.is_none());
    }

    #[tokio::test]
    async fn count_emails_returns_zero_for_empty_table() {
        let pool = create_pool().await;
        sqlx::query("CREATE TABLE emails (id INTEGER PRIMARY KEY, subject TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(count_emails(&pool).await, Some(0));
    }

    #[tokio::test]
    async fn count_emails_returns_correct_count() {
        let pool = create_pool().await;
        sqlx::query("CREATE TABLE emails (id INTEGER PRIMARY KEY, subject TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO emails (subject) VALUES ('a'), ('b'), ('c')")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(count_emails(&pool).await, Some(3));
    }

    #[tokio::test]
    async fn count_orders_returns_none_when_table_missing() {
        let pool = create_pool().await;
        assert!(count_orders(&pool).await.is_none());
    }

    #[tokio::test]
    async fn count_orders_returns_zero_for_empty_table() {
        let pool = create_pool().await;
        sqlx::query("CREATE TABLE orders (id INTEGER PRIMARY KEY, item TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(count_orders(&pool).await, Some(0));
    }

    #[tokio::test]
    async fn count_orders_returns_correct_count() {
        let pool = create_pool().await;
        sqlx::query("CREATE TABLE orders (id INTEGER PRIMARY KEY, item TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO orders (item) VALUES ('x'), ('y')")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(count_orders(&pool).await, Some(2));
    }
}
