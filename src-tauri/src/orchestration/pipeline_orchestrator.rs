//! スケジューラ用パイプラインオーケストレーション。
//!
//! 「差分同期 → メールパース → 商品名解析 → 配達状況確認」を順番に実行する。
//! 各ステップの成否に関わらず次のステップへ進む（ベストエフォート方式）。
//! 手動実行中のステップはスキップして次へ進む。
//!
//! メールパース以降は前ステップで新規データが生まれた場合のみ実行する。
//! ただし手動実行中でスキップされたステップは「結果不明」として後続ステップに進む。

use sqlx::sqlite::SqlitePool;
use tauri::Manager;

use super::clamp_batch_size;

/// ステップの実行結果
enum StepOutcome {
    /// ステップを実行し、新規データ数を返す
    Ran { new_count: i64 },
    /// 手動実行中・状態未取得などでスキップされた
    Skipped,
}

/// パイプラインを実行する。スケジューラから呼ばれる。
pub async fn run_pipeline(app: &tauri::AppHandle) {
    let pool = match app.try_state::<SqlitePool>() {
        Some(p) => p.inner().clone(),
        None => {
            log::error!("[Pipeline] SqlitePool not available, aborting");
            return;
        }
    };

    // Step 1: 差分同期
    let sync_outcome = run_sync_step(app, &pool).await;
    match &sync_outcome {
        StepOutcome::Ran { new_count: 0 } => {
            log::info!("[Pipeline] No new emails synced, skipping subsequent steps");
            return;
        }
        StepOutcome::Ran { new_count } => {
            log::info!("[Pipeline] {new_count} new email(s) synced, proceeding to parse");
        }
        StepOutcome::Skipped => {
            log::info!("[Pipeline] Sync was skipped, proceeding to parse anyway");
        }
    }

    // Step 2: メールパース
    let parse_outcome = run_parse_step(app, &pool).await;
    match &parse_outcome {
        StepOutcome::Ran { new_count: 0 } => {
            log::info!("[Pipeline] No new orders after parse, skipping subsequent steps");
            return;
        }
        StepOutcome::Ran { new_count } => {
            log::info!("[Pipeline] {new_count} new order(s) after parse, proceeding");
        }
        StepOutcome::Skipped => {
            log::info!("[Pipeline] Parse was skipped, proceeding anyway");
        }
    }

    // Step 3: 商品名解析
    run_product_parse_step(app, &pool).await;

    // Step 4: 配達状況確認
    run_delivery_check_step(app, &pool).await;
}

/// 差分同期を実行し、新規メール件数を返す。
/// 手動同期中の場合は Skipped を返す。認証未設定の場合は new_count = 0 として扱う。
///
/// Note: `SyncState::try_start()` をここで直接呼び出すことで、
/// 既に実行中の場合はエラーイベントを emit せずに静かにスキップできる。
/// `run_incremental_sync_task` には `caller_did_try_start = true` を渡し、
/// 内部での二重 `try_start()` を防ぐ。
async fn run_sync_step(app: &tauri::AppHandle, pool: &SqlitePool) -> StepOutcome {
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

    if !sync_state.try_start() {
        log::info!("[Pipeline] Sync already running, skipping");
        return StepOutcome::Skipped;
    }

    let before = count_emails(pool).await;
    log::info!("[Pipeline] Step 1/4: incremental sync");
    super::run_incremental_sync_task(app.clone(), pool.clone(), sync_state, true).await;
    log::info!("[Pipeline] Step 1/4: incremental sync completed");
    let after = count_emails(pool).await;

    StepOutcome::Ran {
        new_count: after.saturating_sub(before),
    }
}

/// メールパースを実行し、新規注文件数を返す。
/// 手動パース中の場合は Skipped を返す。
async fn run_parse_step(app: &tauri::AppHandle, pool: &SqlitePool) -> StepOutcome {
    use crate::parsers::ParseState;

    let parse_state = match app.try_state::<ParseState>() {
        Some(s) => s.inner().clone(),
        None => {
            log::warn!("[Pipeline] ParseState not available, skipping parse");
            return StepOutcome::Skipped;
        }
    };

    if *parse_state
        .is_running
        .lock()
        .unwrap_or_else(|e| e.into_inner())
    {
        log::info!("[Pipeline] Parse already running, skipping");
        return StepOutcome::Skipped;
    }

    let batch_size = load_parse_batch_size(app);
    let before = count_orders(pool).await;
    log::info!(
        "[Pipeline] Step 2/4: batch parse (batch_size={})",
        batch_size
    );
    super::run_batch_parse_task(app.clone(), pool.clone(), parse_state, batch_size).await;
    log::info!("[Pipeline] Step 2/4: batch parse completed");
    let after = count_orders(pool).await;

    StepOutcome::Ran {
        new_count: after.saturating_sub(before),
    }
}

async fn run_product_parse_step(app: &tauri::AppHandle, pool: &SqlitePool) {
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

    log::info!("[Pipeline] Step 3/4: product name parse");
    super::run_product_name_parse_task(app.clone(), pool.clone(), parse_state, true).await;
    log::info!("[Pipeline] Step 3/4: product name parse completed");
}

async fn run_delivery_check_step(app: &tauri::AppHandle, pool: &SqlitePool) {
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

    log::info!("[Pipeline] Step 4/4: delivery check");
    super::run_delivery_check_task(app.clone(), pool.clone(), check_state).await;
    log::info!("[Pipeline] Step 4/4: delivery check completed");
}

async fn count_emails(pool: &SqlitePool) -> i64 {
    match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM emails")
        .fetch_one(pool)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            log::error!("[Pipeline] Failed to count emails: {e}");
            0
        }
    }
}

async fn count_orders(pool: &SqlitePool) -> i64 {
    match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM orders")
        .fetch_one(pool)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            log::error!("[Pipeline] Failed to count orders: {e}");
            0
        }
    }
}

fn load_parse_batch_size(app: &tauri::AppHandle) -> usize {
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
