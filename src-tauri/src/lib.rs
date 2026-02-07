use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem, Submenu};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{Listener, Manager};
use tauri_plugin_sql::{Migration, MigrationKind};

pub mod batch_commands;
pub mod batch_runner;
pub mod config;
pub mod e2e_mocks;
pub mod e2e_seed;
pub mod gemini;
pub mod gmail;
pub mod gmail_client;
pub mod google_search;
pub mod logic;
pub mod metadata_export;
pub mod parsers;
pub mod repository;

use crate::e2e_mocks::{is_e2e_mock_mode, E2EMockImageSearchClient};
use crate::logic::email_parser::get_candidate_parsers;
use crate::repository::{
    DeliveryStats, DeliveryStatsRepository, EmailStats, EmailStatsRepository, MiscStats,
    MiscStatsRepository, OrderRepository, OrderStats, OrderStatsRepository, ProductMasterStats,
    ProductMasterStatsRepository, ShopSettingsRepository, SqliteDeliveryStatsRepository,
    SqliteEmailStatsRepository, SqliteMiscStatsRepository, SqliteOrderRepository,
    SqliteOrderStatsRepository, SqliteProductMasterStatsRepository, SqliteShopSettingsRepository,
};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

/// E2E モード時に DB シードを実行。フロントエンドのマウント後に呼ぶ（マイグレーション完了後）
#[tauri::command]
async fn seed_e2e_db(pool: tauri::State<'_, SqlitePool>) -> Result<(), String> {
    e2e_seed::seed_if_e2e_and_empty(pool.inner()).await;
    Ok(())
}

/// DB ファイル名を返す。E2E モード時は paa_e2e.db（開発用と分離）、通常時は paa_data.db
#[tauri::command]
fn get_db_filename() -> &'static str {
    if crate::e2e_mocks::is_e2e_mock_mode() {
        "paa_e2e.db"
    } else {
        "paa_data.db"
    }
}

/// Gmail同期処理を開始
/// BatchRunner<GmailSyncTask> を使用
#[tauri::command]
async fn start_sync(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    sync_state: tauri::State<'_, gmail::SyncState>,
) -> Result<(), String> {
    let pool_clone = pool.inner().clone();
    let sync_state_clone = sync_state.inner().clone();
    tauri::async_runtime::spawn(batch_commands::run_sync_task(
        app_handle,
        pool_clone,
        sync_state_clone,
    ));
    Ok(())
}

#[tauri::command]
async fn cancel_sync(sync_state: tauri::State<'_, gmail::SyncState>) -> Result<(), String> {
    log::info!("Cancelling sync...");
    sync_state.request_cancel();
    Ok(())
}

#[tauri::command]
async fn get_sync_status(
    app_handle: tauri::AppHandle,
    sync_state: tauri::State<'_, gmail::SyncState>,
) -> Result<gmail::SyncMetadata, String> {
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let config = config::load(&app_config_dir)?;

    let sync_status = if sync_state.inner().is_running() {
        "syncing"
    } else if sync_state
        .inner()
        .last_error
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false)
    {
        "error"
    } else {
        "idle"
    };

    let last_error_message = sync_state
        .inner()
        .last_error
        .lock()
        .ok()
        .and_then(|g| g.clone());

    Ok(gmail::SyncMetadata {
        sync_status: sync_status.to_string(),
        oldest_fetched_date: None,
        total_synced_count: 0,
        batch_size: config.sync.batch_size,
        last_sync_started_at: None,
        last_sync_completed_at: None,
        max_iterations: config.sync.max_iterations,
        max_results_per_page: config.sync.max_results_per_page,
        timeout_minutes: config.sync.timeout_minutes,
        last_error_message,
    })
}

#[tauri::command]
async fn reset_sync_status(sync_state: tauri::State<'_, gmail::SyncState>) -> Result<(), String> {
    log::info!("Resetting sync status to 'idle'");
    sync_state.inner().force_idle();
    Ok(())
}

#[tauri::command]
async fn reset_sync_date() -> Result<(), String> {
    log::info!("reset_sync_date: no-op (oldest_fetched_date は未使用)");
    Ok(())
}

#[tauri::command]
async fn update_batch_size(app_handle: tauri::AppHandle, batch_size: i64) -> Result<(), String> {
    log::info!("Updating sync batch size to: {batch_size}");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.sync.batch_size = batch_size;
    config::save(&app_config_dir, &config)
}

/// 最大繰り返し回数のバリデーション（1以上である必要がある）
pub fn validate_max_iterations(max_iterations: i64) -> Result<(), String> {
    if max_iterations <= 0 {
        return Err("最大繰り返し回数は1以上である必要があります".to_string());
    }
    Ok(())
}

#[tauri::command]
async fn update_max_iterations(
    app_handle: tauri::AppHandle,
    max_iterations: i64,
) -> Result<(), String> {
    validate_max_iterations(max_iterations)?;

    log::info!("Updating max iterations to: {max_iterations}");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.sync.max_iterations = max_iterations;
    config::save(&app_config_dir, &config)
}

#[tauri::command]
async fn update_max_results_per_page(
    app_handle: tauri::AppHandle,
    max_results_per_page: i64,
) -> Result<(), String> {
    if !(1..=500).contains(&max_results_per_page) {
        return Err("1ページあたり取得件数は1〜500の範囲である必要があります".to_string());
    }
    log::info!("Updating max results per page to: {max_results_per_page}");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.sync.max_results_per_page = max_results_per_page;
    config::save(&app_config_dir, &config)
}

#[tauri::command]
async fn update_timeout_minutes(
    app_handle: tauri::AppHandle,
    timeout_minutes: i64,
) -> Result<(), String> {
    if !(1..=120).contains(&timeout_minutes) {
        return Err("同期タイムアウトは1〜120分の範囲である必要があります".to_string());
    }
    log::info!("Updating sync timeout to: {timeout_minutes} minutes");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.sync.timeout_minutes = timeout_minutes;
    config::save(&app_config_dir, &config)
}

#[tauri::command]
async fn get_gemini_config(app_handle: tauri::AppHandle) -> Result<config::GeminiConfig, String> {
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let config = config::load(&app_config_dir)?;
    Ok(config.gemini)
}

#[tauri::command]
async fn update_gemini_batch_size(
    app_handle: tauri::AppHandle,
    batch_size: i64,
) -> Result<(), String> {
    if !(1..=50).contains(&batch_size) {
        return Err("商品名パースのバッチサイズは1〜50の範囲である必要があります".to_string());
    }
    log::info!("Updating Gemini batch size to: {batch_size}");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.gemini.batch_size = batch_size;
    config::save(&app_config_dir, &config)
}

#[tauri::command]
async fn update_gemini_delay_seconds(
    app_handle: tauri::AppHandle,
    delay_seconds: i64,
) -> Result<(), String> {
    if !(0..=60).contains(&delay_seconds) {
        return Err("リクエスト間の待機秒数は0〜60の範囲である必要があります".to_string());
    }
    log::info!("Updating Gemini delay to: {delay_seconds} seconds");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.gemini.delay_seconds = delay_seconds;
    config::save(&app_config_dir, &config)
}

#[tauri::command]
async fn get_window_settings(app_handle: tauri::AppHandle) -> Result<config::WindowConfig, String> {
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let config = config::load(&app_config_dir)?;
    Ok(config.window)
}

/// ウィンドウサイズのバリデーション（最小200、最大10000）
pub fn validate_window_size(width: i64, height: i64) -> Result<(), String> {
    const MIN_SIZE: i64 = 200;
    const MAX_SIZE: i64 = 10000;

    if !(MIN_SIZE..=MAX_SIZE).contains(&width) {
        return Err(format!(
            "ウィンドウの幅は{MIN_SIZE}〜{MAX_SIZE}の範囲である必要があります"
        ));
    }
    if !(MIN_SIZE..=MAX_SIZE).contains(&height) {
        return Err(format!(
            "ウィンドウの高さは{MIN_SIZE}〜{MAX_SIZE}の範囲である必要があります"
        ));
    }
    Ok(())
}

#[tauri::command]
async fn save_window_settings(
    app_handle: tauri::AppHandle,
    width: i64,
    height: i64,
    x: Option<i64>,
    y: Option<i64>,
    maximized: bool,
) -> Result<(), String> {
    validate_window_size(width, height)?;

    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.window = config::WindowConfig {
        width,
        height,
        x,
        y,
        maximized,
    };
    config::save(&app_config_dir, &config)
}

/// Gmail メール取得（BatchRunner 経由で start_sync と同等の処理を実行）
#[tauri::command]
async fn fetch_gmail_emails(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    sync_state: tauri::State<'_, gmail::SyncState>,
) -> Result<gmail::FetchResult, String> {
    log::info!("Starting Gmail email fetch (via start_sync / BatchRunner)...");
    log::info!("If a browser window doesn't open automatically, please check the console for the authentication URL.");

    // BatchRunner を使用する start_sync に委譲
    start_sync(app_handle, pool, sync_state).await?;

    // 進捗は batch-progress イベントで通知される
    Ok(gmail::FetchResult {
        fetched_count: 0,
        saved_count: 0,
        skipped_count: 0,
    })
}

/// メール統計情報を取得
#[tauri::command]
async fn get_email_stats(pool: tauri::State<'_, SqlitePool>) -> Result<EmailStats, String> {
    let repo = SqliteEmailStatsRepository::new(pool.inner().clone());
    repo.get_email_stats().await
}

/// 注文・商品サマリを取得
#[tauri::command]
async fn get_order_stats(pool: tauri::State<'_, SqlitePool>) -> Result<OrderStats, String> {
    let repo = SqliteOrderStatsRepository::new(pool.inner().clone());
    repo.get_order_stats().await
}

/// 配送状況サマリを取得
#[tauri::command]
async fn get_delivery_stats(pool: tauri::State<'_, SqlitePool>) -> Result<DeliveryStats, String> {
    let repo = SqliteDeliveryStatsRepository::new(pool.inner().clone());
    repo.get_delivery_stats().await
}

/// 商品名解析進捗を取得
#[tauri::command]
async fn get_product_master_stats(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<ProductMasterStats, String> {
    let repo = SqliteProductMasterStatsRepository::new(pool.inner().clone());
    repo.get_product_master_stats().await
}

/// 店舗設定・画像サマリを取得
#[tauri::command]
async fn get_misc_stats(pool: tauri::State<'_, SqlitePool>) -> Result<MiscStats, String> {
    let repo = SqliteMiscStatsRepository::new(pool.inner().clone());
    repo.get_misc_stats().await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

// ログバッファ用グローバルMutex
//
// 注意: グローバルMutexの使用はロック競合のリスクがあります。
// 現在の実装では適切なエラーハンドリングにより安全性を確保していますが、
// 将来的にはTauriのステート管理機能への移行を検討してください。
//
// パフォーマンスに関する考慮事項:
// - ログ記録の度にMutexロックを取得しますが、ロック保持時間は短く抑えられています
// - MAX_LOG_ENTRIESを超えた古いログは自動的に削除され、メモリ使用量を制限しています
// - 通常のアプリケーション使用では十分なパフォーマンスを提供します
static LOG_BUFFER: Mutex<Option<VecDeque<LogEntry>>> = Mutex::new(None);
const MAX_LOG_ENTRIES: usize = 1000;

/// ログバッファを初期化
///
/// アプリケーション起動時に一度だけ呼び出してください。
/// 複数回呼び出しても安全ですが、既存のログは破棄されます。
pub fn init_log_buffer() {
    match LOG_BUFFER.lock() {
        Ok(mut buffer) => {
            *buffer = Some(VecDeque::with_capacity(MAX_LOG_ENTRIES));
        }
        Err(e) => {
            eprintln!("Failed to initialize log buffer: {e}");
            // ログバッファの初期化に失敗してもアプリケーションは継続
            // ログ機能は利用できないが、クラッシュは回避
        }
    }
}

/// ログエントリを追加
///
/// # パラメータ
/// - `level`: ログレベル（例: "INFO", "ERROR", "DEBUG"）
/// - `message`: ログメッセージ
///
/// # パフォーマンス
/// この関数はログ記録の度にMutexロックを取得しますが、
/// ロック保持時間は最小限（数マイクロ秒）に抑えられています。
/// 通常のログ記録頻度では問題になりません。
pub fn add_log_entry(level: &str, message: &str) {
    match LOG_BUFFER.lock() {
        Ok(mut buffer) => {
            if let Some(ref mut logs) = *buffer {
                let entry = LogEntry {
                    timestamp: chrono::Utc::now()
                        .with_timezone(&chrono_tz::Asia::Tokyo)
                        .format("%Y-%m-%d %H:%M:%S%.3f")
                        .to_string(),
                    level: level.to_string(),
                    message: message.to_string(),
                };

                logs.push_back(entry);

                if logs.len() > MAX_LOG_ENTRIES {
                    logs.pop_front();
                }
            }
            // ログバッファが未初期化の場合は静かに無視
            // アプリケーション起動時の初期化前に呼ばれる可能性がある
        }
        Err(e) => {
            // ロック取得失敗時は標準エラー出力に出力
            // ログシステム自体が問題を抱えているため、通常のログ機能は使えない
            eprintln!("Failed to lock log buffer for adding entry: {e}");
        }
    }
}

/// ログエントリを取得
///
/// # パラメータ
/// - `level_filter`: ログレベルでフィルタリング（例: "ERROR", "INFO"）。Noneの場合は全てのレベルを返す
/// - `limit`: 返却する最大件数。フィルタリング後のログに対して適用される
///
/// # 戻り値
/// 新しい順（最新が先頭）でログエントリのリストを返す
///
/// # 注意
/// limitパラメータはフィルタリング後のログに適用されます。
/// 例：limit=100, `level_filter="ERROR"の場合、ERRORログから最大100件を返します`。
#[tauri::command]
fn get_logs(level_filter: Option<String>, limit: Option<usize>) -> Result<Vec<LogEntry>, String> {
    let buffer = LOG_BUFFER
        .lock()
        .map_err(|e| format!("Failed to lock log buffer: {e}"))?;

    if let Some(ref logs) = *buffer {
        let mut filtered_logs: Vec<LogEntry> = logs
            .iter()
            .filter(|entry| {
                if let Some(ref filter) = level_filter {
                    &entry.level == filter
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        filtered_logs.reverse();

        if let Some(limit) = limit {
            filtered_logs.truncate(limit);
        }

        Ok(filtered_logs)
    } else {
        Ok(Vec::new())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = || {
        vec![Migration {
            version: 1,
            description: "init",
            sql: include_str!("../migrations/001_init.sql"),
            kind: MigrationKind::Up,
        }]
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 二重起動が検知された場合、既存のウィンドウを最前面に表示
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
                log::info!("Second instance detected - bringing existing window to front");
            }
        }))
        .setup(move |app| {
            // ログバッファの初期化
            init_log_buffer();

            // マルチロガーの初期化（コンソールとメモリの両方に出力）
            // リリースビルドではWarnレベル以上、デバッグビルドではInfoレベル以上のログを出力
            // これにより、本番環境で機密情報を含む可能性のあるデバッグログを防ぐ
            #[cfg(debug_assertions)]
            let default_level = log::LevelFilter::Info;
            #[cfg(not(debug_assertions))]
            let default_level = log::LevelFilter::Warn;

            env_logger::Builder::from_default_env()
                .filter_level(default_level)
                .format(|buf, record| {
                    // メモリにログを保存
                    add_log_entry(&record.level().to_string(), &format!("{}", record.args()));

                    // コンソールにも出力（JST）。タイムゾーン規約: README §4 参照
                    writeln!(
                        buf,
                        "[{} {:5} {}] {}",
                        chrono::Utc::now()
                            .with_timezone(&chrono_tz::Asia::Tokyo)
                            .format("%Y-%m-%d %H:%M:%S"),
                        record.level(),
                        record.target(),
                        record.args()
                    )
                })
                .init();

            // DBはapp_config_dirに配置（tauri-plugin-sqlのpreloadとパスを統一）
            // E2E モード時は paa_e2e.db を使用し、開発用 paa_data.db と分離する
            let app_config_dir = app
                .path()
                .app_config_dir()
                .expect("failed to get app config dir");
            std::fs::create_dir_all(&app_config_dir).expect("failed to create app config dir");

            let db_filename = if crate::e2e_mocks::is_e2e_mock_mode() {
                "paa_e2e.db"
            } else {
                "paa_data.db"
            };
            let db_path = app_config_dir.join(db_filename);
            let db_url = format!("sqlite:{}", db_path.to_string_lossy());

            log::info!(
                "Database path: {} (E2E={})",
                db_path.display(),
                crate::e2e_mocks::is_e2e_mock_mode()
            );

            // tauri-plugin-sqlを登録。両DBにマイグレーションを登録（E2E/通常でどちらか一方のみ使用）
            app.handle().plugin(
                tauri_plugin_sql::Builder::default()
                    .add_migrations("sqlite:paa_data.db", migrations())
                    .add_migrations("sqlite:paa_e2e.db", migrations())
                    .build(),
            )?;

            log::info!("tauri-plugin-sql registered with migrations");

            // sqlxプールを作成してバックエンド用に管理
            // DB自体はtauri-plugin-sqlのマイグレーションで初期化される想定
            let pool = tauri::async_runtime::block_on(async {
                use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
                use std::str::FromStr;

                // DB接続オプション（create_if_missingを有効化）
                let options = SqliteConnectOptions::from_str(&db_url)
                    .expect("Failed to parse database URL")
                    .create_if_missing(true);

                // DB接続プール作成
                SqlitePoolOptions::new()
                    .connect_with(options)
                    .await
                    .expect("Failed to create sqlx pool")
            });

            app.manage(pool.clone());
            log::info!("sqlx pool created for backend use");

            // E2E シードはフロントエンドの initDb 完了後に seed_e2e_db コマンドで実行

            // Initialize sync state
            app.manage(gmail::SyncState::new());
            log::info!("Sync state initialized");

            // Initialize parse state
            app.manage(parsers::ParseState::new());
            log::info!("Parse state initialized");

            // Initialize product name parse state (多重実行ガード用)
            app.manage(ProductNameParseState::new());
            log::info!("Product name parse state initialized");

            // Restore window settings and setup close handler
            let window = app
                .get_webview_window("main")
                .expect("Failed to get main window");

            // Handle window close request - hide instead of closing
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_clone.hide();
                }
            });

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let app_config_dir = match app_handle.path().app_config_dir() {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("Failed to get app config dir: {e}");
                        return;
                    }
                };
                let config = match config::load(&app_config_dir) {
                    Ok(c) => c,
                    Err(e) => {
                        log::error!("Failed to load config: {e}");
                        return;
                    }
                };
                let settings = &config.window;

                // Set window size
                let _ = window.set_size(tauri::LogicalSize {
                    width: settings.width as u32,
                    height: settings.height as u32,
                });

                // Set window position if available
                if let (Some(x_pos), Some(y_pos)) = (settings.x, settings.y) {
                    #[allow(clippy::cast_possible_truncation)]
                    let _ = window.set_position(tauri::LogicalPosition {
                        x: x_pos as i32,
                        y: y_pos as i32,
                    });
                }

                // Set maximized state
                if settings.maximized {
                    let _ = window.maximize();
                }

                log::info!(
                    "Window settings restored: {}x{}",
                    settings.width,
                    settings.height
                );
            });

            // Setup system tray
            let show_item = MenuItem::with_id(app, "show", "表示", true, None::<&str>)?;
            let sync_item = MenuItem::with_id(app, "tray_sync", "Gmail同期", true, None::<&str>)?;
            let parse_item =
                MenuItem::with_id(app, "tray_parse", "メールパース", true, None::<&str>)?;
            let product_item = MenuItem::with_id(
                app,
                "tray_product_name_parse",
                "商品名解析",
                true,
                None::<&str>,
            )?;
            let batch_submenu = Submenu::with_id_and_items(
                app,
                "batch",
                "バッチ処理",
                true,
                &[&sync_item, &parse_item, &product_item],
            )?;
            let quit_item = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &batch_submenu, &quit_item])?;

            // Initialize tray icon builder and set icon if available to avoid panics
            let mut tray_builder = TrayIconBuilder::new();
            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            } else {
                log::warn!(
                    "No default window icon found; initializing system tray without a custom icon."
                );
            }

            let _tray = tray_builder
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "tray_sync" => {
                        if let (Some(pool), Some(sync_state)) = (
                            app.try_state::<SqlitePool>(),
                            app.try_state::<gmail::SyncState>(),
                        ) {
                            let app_clone = app.clone();
                            let pool_clone = pool.inner().clone();
                            let sync_state_clone = sync_state.inner().clone();
                            tauri::async_runtime::spawn(batch_commands::run_sync_task(
                                app_clone,
                                pool_clone,
                                sync_state_clone,
                            ));
                        }
                    }
                    "tray_parse" => {
                        if let (Some(pool), Some(parse_state)) = (
                            app.try_state::<SqlitePool>(),
                            app.try_state::<parsers::ParseState>(),
                        ) {
                            let app_clone = app.clone();
                            let pool_clone = pool.inner().clone();
                            let parse_state_clone = parse_state.inner().clone();
                            let batch_size = app
                                .path()
                                .app_config_dir()
                                .ok()
                                .and_then(|dir| config::load(&dir).ok())
                                .map(|c| {
                                    let v = c.parse.batch_size;
                                    if v <= 0 {
                                        100usize
                                    } else {
                                        v as usize
                                    }
                                })
                                .unwrap_or(100);
                            tauri::async_runtime::spawn(batch_commands::run_batch_parse_task(
                                app_clone,
                                pool_clone,
                                parse_state_clone,
                                batch_size,
                            ));
                        }
                    }
                    "tray_product_name_parse" => {
                        if let (Some(pool), Some(parse_state)) = (
                            app.try_state::<SqlitePool>(),
                            app.try_state::<ProductNameParseState>(),
                        ) {
                            let app_clone = app.clone();
                            let pool_clone = pool.inner().clone();
                            let parse_state_clone = parse_state.inner().clone();
                            tauri::async_runtime::spawn(
                                batch_commands::run_product_name_parse_task(
                                    app_clone,
                                    pool_clone,
                                    parse_state_clone,
                                ),
                            );
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            log::info!("System tray initialized");

            // Set up notification action listener
            let app_handle = app.handle().clone();
            app.listen("notification-action", move |event| {
                log::info!("Notification action event: {event:?}");
                // Show main window when notification is clicked
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            seed_e2e_db,
            get_db_filename,
            fetch_gmail_emails,
            start_sync,
            cancel_sync,
            get_sync_status,
            update_batch_size,
            update_max_iterations,
            update_max_results_per_page,
            update_timeout_minutes,
            reset_sync_status,
            reset_sync_date,
            get_window_settings,
            save_window_settings,
            get_email_stats,
            get_order_stats,
            get_delivery_stats,
            get_product_master_stats,
            get_misc_stats,
            get_logs,
            get_all_shop_settings,
            create_shop_setting,
            update_shop_setting,
            delete_shop_setting,
            parse_email,
            parse_and_save_email,
            start_batch_parse,
            cancel_parse,
            get_parse_status,
            update_parse_batch_size,
            get_gemini_config,
            update_gemini_batch_size,
            update_gemini_delay_seconds,
            // Gemini API commands
            has_gemini_api_key,
            save_gemini_api_key,
            delete_gemini_api_key,
            start_product_name_parse,
            // Gmail OAuth commands
            has_gmail_oauth_credentials,
            save_gmail_oauth_credentials,
            delete_gmail_oauth_credentials,
            // SerpApi image search commands
            is_google_search_configured,
            save_google_search_api_key,
            delete_google_search_config,
            search_product_images,
            save_image_from_url,
            export_metadata,
            import_metadata,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Shop Settings Commands
#[tauri::command]
async fn get_all_shop_settings(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<gmail::ShopSettings>, String> {
    gmail::get_all_shop_settings(pool.inner()).await
}

#[tauri::command]
async fn create_shop_setting(
    pool: tauri::State<'_, SqlitePool>,
    shop_name: String,
    sender_address: String,
    parser_type: String,
    subject_filters: Option<Vec<String>>,
) -> Result<i64, String> {
    let settings = gmail::CreateShopSettings {
        shop_name,
        sender_address,
        parser_type,
        subject_filters,
    };
    gmail::create_shop_setting(pool.inner(), settings).await
}

#[tauri::command]
async fn update_shop_setting(
    pool: tauri::State<'_, SqlitePool>,
    id: i64,
    shop_name: Option<String>,
    sender_address: Option<String>,
    parser_type: Option<String>,
    is_enabled: Option<bool>,
    subject_filters: Option<Vec<String>>,
) -> Result<(), String> {
    let settings = gmail::UpdateShopSettings {
        shop_name,
        sender_address,
        parser_type,
        is_enabled,
        subject_filters,
    };
    gmail::update_shop_setting(pool.inner(), id, settings).await
}

#[tauri::command]
async fn delete_shop_setting(pool: tauri::State<'_, SqlitePool>, id: i64) -> Result<(), String> {
    gmail::delete_shop_setting(pool.inner(), id).await
}

#[tauri::command]
fn parse_email(parser_type: String, email_body: String) -> Result<parsers::OrderInfo, String> {
    let parser = parsers::get_parser(&parser_type)
        .ok_or_else(|| format!("Unknown parser type: {}", parser_type))?;

    parser.parse(&email_body)
}

#[tauri::command]
async fn parse_and_save_email(
    pool: tauri::State<'_, SqlitePool>,
    email_body: String,
    email_id: Option<i64>,
    shop_domain: Option<String>,
    sender_address: String,
    subject: Option<String>,
) -> Result<i64, String> {
    // shop_settingsから有効な設定を取得
    let shop_settings_repo = SqliteShopSettingsRepository::new(pool.inner().clone());
    let enabled_settings = shop_settings_repo.get_enabled().await?;
    let shop_settings: Vec<(String, String, Option<String>)> = enabled_settings
        .into_iter()
        .map(|s| (s.sender_address, s.parser_type, s.subject_filters))
        .collect();

    // 送信元アドレスと件名フィルターから候補のパーサータイプを取得（extract_email_address + 完全一致）
    let candidate_parsers =
        get_candidate_parsers(&sender_address, subject.as_deref(), &shop_settings);

    if candidate_parsers.is_empty() {
        return Err(format!(
            "No parser found for address: {} with subject: {:?}",
            sender_address, subject
        ));
    }

    // 複数のパーサーを順番に試す（最初に成功したものを使用）
    // パーサーの参照をawaitの前で解放するため、同期ブロック内で完了させる
    let order_info = {
        let mut last_error = String::new();
        let mut result = None;

        for parser_type in &candidate_parsers {
            let parser = match parsers::get_parser(parser_type) {
                Some(p) => p,
                None => {
                    log::warn!("Unknown parser type: {}", parser_type);
                    continue;
                }
            };

            match parser.parse(&email_body) {
                Ok(info) => {
                    log::info!("Successfully parsed with parser: {}", parser_type);
                    result = Some(info);
                    break;
                }
                Err(e) => {
                    log::debug!("Parser {} failed: {}", parser_type, e);
                    last_error = e;
                    // 次のパーサーを試す
                    continue;
                }
            }
        }

        match result {
            Some(info) => info,
            None => return Err(format!("All parsers failed. Last error: {}", last_error)),
        }
    };

    // データベースに保存（非同期処理）
    let order_repo = SqliteOrderRepository::new(pool.inner().clone());
    order_repo
        .save_order(&order_info, email_id, shop_domain, None)
        .await
}

/// メールパース処理を開始
/// BatchRunner<EmailParseTask> を使用
#[tauri::command]
async fn start_batch_parse(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    parse_state: tauri::State<'_, parsers::ParseState>,
    batch_size: Option<usize>,
) -> Result<(), String> {
    let size = if let Some(s) = batch_size {
        s
    } else {
        let app_config_dir = app_handle
            .path()
            .app_config_dir()
            .map_err(|e| format!("Failed to get app config dir: {e}"))?;
        let config = config::load(&app_config_dir)?;
        config.parse.batch_size as usize
    };

    let pool_clone = pool.inner().clone();
    let parse_state_clone = parse_state.inner().clone();
    tauri::async_runtime::spawn(batch_commands::run_batch_parse_task(
        app_handle,
        pool_clone,
        parse_state_clone,
        size,
    ));
    Ok(())
}

#[tauri::command]
async fn cancel_parse(parse_state: tauri::State<'_, parsers::ParseState>) -> Result<(), String> {
    log::info!("Cancelling parse...");
    parse_state.request_cancel();
    Ok(())
}

#[tauri::command]
async fn get_parse_status(
    app_handle: tauri::AppHandle,
    parse_state: tauri::State<'_, parsers::ParseState>,
) -> Result<parsers::ParseMetadata, String> {
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let config = config::load(&app_config_dir)?;

    let parse_status = if parse_state
        .inner()
        .is_running
        .lock()
        .map(|g| *g)
        .unwrap_or(false)
    {
        "running"
    } else if parse_state
        .inner()
        .last_error
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false)
    {
        "error"
    } else {
        "idle"
    };

    let last_error_message = parse_state
        .inner()
        .last_error
        .lock()
        .ok()
        .and_then(|g| g.clone());

    Ok(parsers::ParseMetadata {
        parse_status: parse_status.to_string(),
        last_parse_started_at: None,
        last_parse_completed_at: None,
        total_parsed_count: 0,
        last_error_message,
        batch_size: config.parse.batch_size,
    })
}

#[tauri::command]
async fn update_parse_batch_size(
    app_handle: tauri::AppHandle,
    batch_size: i64,
) -> Result<(), String> {
    log::info!("Updating parse batch size to: {batch_size}");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.parse.batch_size = batch_size;
    config::save(&app_config_dir, &config)
}

// =============================================================================
// Gemini API Commands
// =============================================================================

/// Gemini APIキーが設定されているかチェック
#[tauri::command]
async fn has_gemini_api_key(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    Ok(gemini::has_api_key(&app_data_dir))
}

/// Gemini APIキーを保存
#[tauri::command]
async fn save_gemini_api_key(app_handle: tauri::AppHandle, api_key: String) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    gemini::config::save_api_key(&app_data_dir, &api_key)?;

    log::info!("Gemini API key saved successfully");
    Ok(())
}

/// Gemini APIキーを削除
#[tauri::command]
async fn delete_gemini_api_key(app_handle: tauri::AppHandle) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    gemini::config::delete_api_key(&app_data_dir)?;

    log::info!("Gemini API key deleted successfully");
    Ok(())
}

// =============================================================================
// Gmail OAuth Commands
// =============================================================================

/// Gmail OAuth認証情報が設定されているかチェック
#[tauri::command]
async fn has_gmail_oauth_credentials(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    Ok(gmail::has_oauth_credentials(&app_data_dir))
}

/// Gmail OAuth認証情報を保存（JSONから）
#[tauri::command]
async fn save_gmail_oauth_credentials(
    app_handle: tauri::AppHandle,
    json_content: String,
) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    gmail::save_oauth_credentials_from_json(&app_data_dir, &json_content)?;
    log::info!("Gmail OAuth credentials saved successfully");
    Ok(())
}

/// Gmail OAuth認証情報を削除
#[tauri::command]
async fn delete_gmail_oauth_credentials(app_handle: tauri::AppHandle) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    gmail::delete_oauth_credentials(&app_data_dir)?;
    log::info!("Gmail OAuth credentials deleted successfully");
    Ok(())
}

/// 商品名パースの多重実行ガード用状態
#[derive(Clone, Default)]
pub struct ProductNameParseState {
    is_running: Arc<Mutex<bool>>,
}

impl ProductNameParseState {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    pub fn try_start(&self) -> Result<(), String> {
        let mut running = self
            .is_running
            .lock()
            .map_err(|e| format!("Lock error: {e}"))?;
        if *running {
            return Err("商品名解析は既に実行中です。完了するまでお待ちください。".to_string());
        }
        *running = true;
        Ok(())
    }

    pub fn finish(&self) {
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }
    }
}

/// 商品名パース進捗イベント（後方互換性のため残す）
/// 新しいコードでは BatchProgressEvent を使用してください
#[derive(Debug, Clone, serde::Serialize)]
#[deprecated(note = "Use BatchProgressEvent instead")]
pub struct ProductNameParseProgress {
    pub total_items: usize,
    pub parsed_count: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub status_message: String,
    pub is_complete: bool,
    pub error: Option<String>,
}

/// product_masterに未登録の商品名をGemini APIで解析して登録
/// BatchRunner<ProductNameParseTask> を使用
#[tauri::command]
async fn start_product_name_parse(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    parse_state: tauri::State<'_, ProductNameParseState>,
) -> Result<(), String> {
    let pool_clone = pool.inner().clone();
    let parse_state_clone = parse_state.inner().clone();
    tauri::async_runtime::spawn(batch_commands::run_product_name_parse_task(
        app_handle,
        pool_clone,
        parse_state_clone,
    ));
    Ok(())
}

// =============================================================================
// SerpApi Image Search Commands
// =============================================================================

/// SerpApi が設定済みかチェック（API Key のみ）
#[tauri::command]
async fn is_google_search_configured(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    Ok(google_search::is_configured(&app_data_dir))
}

/// SerpApi API キーを保存
#[tauri::command]
async fn save_google_search_api_key(
    app_handle: tauri::AppHandle,
    api_key: String,
) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    google_search::save_api_key(&app_data_dir, &api_key)?;

    log::info!("SerpApi API key saved successfully");
    Ok(())
}

/// SerpApi API 設定を削除
#[tauri::command]
async fn delete_google_search_config(app_handle: tauri::AppHandle) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    google_search::delete_api_key(&app_data_dir)?;

    log::info!("SerpApi config deleted successfully");
    Ok(())
}

/// 商品画像を検索（SerpApi）
#[tauri::command]
async fn search_product_images(
    app_handle: tauri::AppHandle,
    query: String,
    num_results: Option<u32>,
) -> Result<Vec<google_search::ImageSearchResult>, String> {
    use google_search::ImageSearchClientTrait;

    let num = num_results.unwrap_or(10);

    // E2Eモック時は外部APIを呼ばない
    if is_e2e_mock_mode() {
        log::info!("Using E2E mock image search");
        let client = E2EMockImageSearchClient;
        return client.search_images(&query, num).await;
    }

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    if !google_search::is_configured(&app_data_dir) {
        return Err(
            "SerpApiが設定されていません。設定画面でAPIキーを設定してください。".to_string(),
        );
    }

    let api_key = google_search::load_api_key(&app_data_dir)?;

    let client = google_search::SerpApiClient::new(api_key)?;
    client.search_images(&query, num).await
}

/// 画像ダウンロード用URLの検証（SSRF対策）
fn validate_image_url(url_str: &str) -> Result<(), String> {
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

/// プライベートIPアドレスかどうかを判定
fn is_private_ip(ip: std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            // 10.0.0.0/8
            octets[0] == 10
                // 172.16.0.0/12
                || (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31)
                // 192.168.0.0/16
                || (octets[0] == 192 && octets[1] == 168)
                // 127.0.0.0/8 (localhost)
                || octets[0] == 127
                // 169.254.0.0/16 (link-local, メタデータ含む)
                || (octets[0] == 169 && octets[1] == 254)
        }
        IpAddr::V6(ipv6) => {
            let segments = ipv6.segments();
            // ::1 (localhost)
            (segments[0] == 0 && segments[1] == 0 && segments[2] == 0
                && segments[3] == 0
                && segments[4] == 0
                && segments[5] == 0
                && segments[6] == 0
                && segments[7] == 1)
                // fe80::/10 (link-local)
                || (segments[0] & 0xffc0 == 0xfe80)
                // fc00::/7 (unique local)
                || (segments[0] & 0xfe00 == 0xfc00)
        }
    }
}

/// 画像URLから画像をダウンロードしてimagesテーブルに保存
#[tauri::command]
async fn save_image_from_url(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    item_id: i64,
    image_url: String,
) -> Result<String, String> {
    use bytes::Bytes;
    use http_body_util::{BodyExt, Full};
    use hyper::{Method, Request};
    use hyper_rustls::HttpsConnector;
    use hyper_util::client::legacy::connect::HttpConnector;
    use hyper_util::client::legacy::Client;
    use hyper_util::rt::TokioExecutor;
    use std::time::Duration;

    const MAX_IMAGE_SIZE_BYTES: usize = 10 * 1024 * 1024; // 10MB

    log::info!("Downloading image for item_id: {}", item_id);

    // URL検証（SSRF対策）
    validate_image_url(&image_url)?;

    // HTTPSクライアントを作成（httpsのみ）
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .map_err(|e| format!("Failed to create HTTPS connector: {e}"))?
        .https_only()
        .enable_http1()
        .build();

    let http_client: Client<HttpsConnector<HttpConnector>, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build(https);

    // 画像をダウンロード
    let req = Request::builder()
        .method(Method::GET)
        .uri(&image_url)
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
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
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
        Ok::<_, String>((status, content_type, content_length, body_bytes))
    })
    .await;

    let (status, _content_type, content_length, image_data) = match request_result {
        Ok(Ok((s, ct, cl, b))) => (s, ct, cl, b),
        Ok(Err(e)) => return Err(e),
        Err(_) => return Err("Image download timed out".to_string()),
    };

    if !status.is_success() {
        return Err(format!("Failed to download image: HTTP {}", status));
    }

    // Content-Length でサイズチェック（事前に拒否可能な場合）
    if let Some(len) = content_length {
        if len > MAX_IMAGE_SIZE_BYTES {
            return Err(format!(
                "Image too large ({} bytes). Maximum size is {} MB",
                len,
                MAX_IMAGE_SIZE_BYTES / (1024 * 1024)
            ));
        }
    }

    // 実際のバイト数でサイズチェック
    if image_data.len() > MAX_IMAGE_SIZE_BYTES {
        return Err(format!(
            "Image too large ({} bytes). Maximum size is {} MB",
            image_data.len(),
            MAX_IMAGE_SIZE_BYTES / (1024 * 1024)
        ));
    }

    // 画像フォーマットの検証（マルウェア対策）
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

    // ファイル名を生成（UUID + 拡張子）
    let file_name = format!("{}.{}", uuid::Uuid::new_v4(), extension);

    // 画像保存ディレクトリを作成
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;
    let images_dir = app_data_dir.join("images");
    std::fs::create_dir_all(&images_dir)
        .map_err(|e| format!("Failed to create images directory: {e}"))?;

    // itemsテーブルからitem_name_normalizedを取得
    let item_name_normalized: Option<String> =
        sqlx::query_scalar("SELECT item_name_normalized FROM items WHERE id = ?")
            .bind(item_id)
            .fetch_optional(pool.inner())
            .await
            .map_err(|e| format!("Failed to get item_name_normalized: {e}"))?
            .flatten();

    // 正規化できない商品名には画像を登録できない（item_name_normalized がリレーションキー）
    let normalized = item_name_normalized.as_ref().ok_or_else(|| {
        "この商品は正規化できないため画像を登録できません。商品名に記号のみなどが含まれている可能性があります。".to_string()
    })?;

    // 既存のfile_nameを取得（古い画像削除用）
    let old_file_name: Option<String> =
        sqlx::query_scalar("SELECT file_name FROM images WHERE item_name_normalized = ?")
            .bind(normalized)
            .fetch_optional(pool.inner())
            .await
            .map_err(|e| format!("Failed to get existing image: {e}"))?
            .flatten();

    // 画像ファイルを保存
    let file_path = images_dir.join(&file_name);
    std::fs::write(&file_path, &image_data)
        .map_err(|e| format!("Failed to write image file: {e}"))?;

    log::info!("Image saved to: {}", file_path.display());

    // データベースに保存（既存レコードがあれば更新、なければ挿入）
    let existing: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM images WHERE item_name_normalized = ?")
            .bind(normalized)
            .fetch_optional(pool.inner())
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
        .bind(normalized)
        .execute(pool.inner())
        .await
        .map_err(|e| format!("Failed to update image in database: {e}"))?;
    } else {
        sqlx::query(
            r#"
            INSERT INTO images (item_name_normalized, file_name, created_at)
            VALUES (?, ?, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(normalized)
        .bind(&file_name)
        .execute(pool.inner())
        .await
        .map_err(|e| format!("Failed to save image to database: {e}"))?;
    }

    log::info!(
        "Image record saved to database for item_name_normalized: {}",
        normalized
    );

    // 古い画像ファイルを削除（ディスク容量節約）
    if let Some(ref old_name) = old_file_name {
        if old_name != &file_name {
            let old_path = images_dir.join(old_name);
            if let Err(e) = std::fs::remove_file(&old_path) {
                log::warn!("Failed to delete old image {}: {}", old_name, e);
            }
        }
    }

    Ok(file_name)
}

/// メタデータ（images, shop_settings, product_master）と画像ファイルをZIPにエクスポート
#[tauri::command]
async fn export_metadata(
    app: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    save_path: String,
) -> Result<metadata_export::ExportResult, String> {
    metadata_export::export_metadata(&app, pool.inner(), std::path::Path::new(&save_path)).await
}

/// ZIPからメタデータをインポート（INSERT OR IGNORE でマージ）
#[tauri::command]
async fn import_metadata(
    app: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    zip_path: String,
) -> Result<metadata_export::ImportResult, String> {
    metadata_export::import_metadata(&app, pool.inner(), std::path::Path::new(&zip_path)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        let result = greet("World");
        assert_eq!(result, "Hello, World! You've been greeted from Rust!");
    }

    #[test]
    fn test_greet_empty() {
        let result = greet("");
        assert_eq!(result, "Hello, ! You've been greeted from Rust!");
    }

    #[test]
    fn test_greet_special_characters() {
        let result = greet("世界");
        assert_eq!(result, "Hello, 世界! You've been greeted from Rust!");
    }

    #[test]
    fn test_log_buffer_initialization() {
        // ログバッファの初期化が成功することを確認
        init_log_buffer();

        // 初期化後にログエントリを追加できることを確認
        add_log_entry("INFO", "Test message");

        // ログを取得できることを確認
        let logs = get_logs(None, None);
        assert!(logs.is_ok());
    }

    #[test]
    fn test_log_buffer_multiple_initialization() {
        // 複数回初期化してもクラッシュしないことを確認
        init_log_buffer();
        init_log_buffer();
        init_log_buffer();

        add_log_entry("INFO", "Test after multiple init");
        let logs = get_logs(None, None);
        assert!(logs.is_ok());
    }

    #[test]
    fn test_add_log_entry_safe() {
        // ログバッファが初期化されていない状態でも
        // add_log_entryがクラッシュしないことを確認
        // （このテストの前に他のテストで初期化済みの可能性があるが、
        // エラーハンドリングが機能することを確認）
        add_log_entry("DEBUG", "Safe logging test");
        add_log_entry("INFO", "Another safe log");
        add_log_entry("ERROR", "Error log test");

        // クラッシュせずにここに到達すればOK
        // Test passes if no panic occurs
    }

    #[test]
    fn test_log_buffer_max_entries() {
        init_log_buffer();

        // MAX_LOG_ENTRIES + 100 個のログを追加
        for i in 0..(MAX_LOG_ENTRIES + 100) {
            add_log_entry("INFO", &format!("Log entry {i}"));
        }

        // ログを取得
        let logs = get_logs(None, None).unwrap();

        // MAX_LOG_ENTRIESを超えないことを確認
        assert!(logs.len() <= MAX_LOG_ENTRIES);
    }

    #[test]
    fn test_get_logs_with_filter() {
        init_log_buffer();

        // 異なるレベルのログを追加
        add_log_entry("INFO", "Info message");
        add_log_entry("ERROR", "Error message");
        add_log_entry("DEBUG", "Debug message");

        // ERRORレベルのみを取得
        let error_logs = get_logs(Some("ERROR".to_string()), None).unwrap();
        assert!(error_logs.iter().all(|log| log.level == "ERROR"));
    }

    #[test]
    fn test_get_logs_with_limit() {
        init_log_buffer();

        // 一意のレベルを使用して他テストとの干渉を防ぐ
        for i in 0..10 {
            add_log_entry("LIMIT_TEST", &format!("Message {i}"));
        }

        // フィルタで自分のログだけ取得し、最大5個に制限
        let logs = get_logs(Some("LIMIT_TEST".to_string()), Some(5)).unwrap();
        // 並列テスト実行時に他テストがバッファをリセットする可能性があるため、
        // limit機能が正しく動作することを確認（取得数がlimit以下）
        assert!(
            logs.len() <= 5,
            "limit should restrict results to at most 5 entries"
        );
        // 全てのログが正しいレベルであることを確認
        assert!(logs.iter().all(|log| log.level == "LIMIT_TEST"));
    }

    // ==================== parse_email Tests ====================

    const SAMPLE_HOBBYSEARCH_CONFIRM: &str = r#"
[注文番号] 25-0101-1234

[お届け先情報]
〒100-0001
東京都千代田区千代田1-1-1
テスト 太郎 様

[ご購入内容]
バンダイ 1234567 テスト商品A (プラモデル) HGシリーズ
単価：1,000円 × 個数：2 = 2,000円

小計：5,000円
送料：660円
合計：5,660円
"#;

    #[test]
    fn test_parse_email_success() {
        let result = parse_email(
            "hobbysearch_confirm".to_string(),
            SAMPLE_HOBBYSEARCH_CONFIRM.to_string(),
        );
        assert!(result.is_ok());
        let order_info = result.unwrap();
        assert_eq!(order_info.order_number, "25-0101-1234");
        assert_eq!(order_info.items.len(), 1);
    }

    #[test]
    fn test_parse_email_unknown_parser_type() {
        let result = parse_email("unknown_parser".to_string(), "body".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown parser type"));
    }

    #[test]
    fn test_parse_email_empty_parser_type() {
        let result = parse_email("".to_string(), SAMPLE_HOBBYSEARCH_CONFIRM.to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_email_invalid_body() {
        // パーサーは存在するが、本文が不正（注文番号なし）
        let result = parse_email(
            "hobbysearch_confirm".to_string(),
            "invalid body".to_string(),
        );
        assert!(result.is_err());
    }

    /// hobbysearch_change パーサーのテスト（CIで実行、ダミーデータ使用）
    const SAMPLE_HOBBYSEARCH_CHANGE: &str = r#"
[注文番号] 25-0202-5678

[お届け先情報]
〒100-0001
東京都千代田区千代田1-1-1
テスト 花子 様

[ご購入内容]
バンダイ 1234567 テスト商品A (プラモデル) HGシリーズ
単価：1,000円 × 個数：1 = 1,000円

小計：1,000円
送料：660円
合計：1,660円
"#;

    #[test]
    fn test_parse_email_hobbysearch_change() {
        let result = parse_email(
            "hobbysearch_change".to_string(),
            SAMPLE_HOBBYSEARCH_CHANGE.to_string(),
        );
        assert!(result.is_ok());
        let order_info = result.unwrap();
        assert_eq!(order_info.order_number, "25-0202-5678");
        assert_eq!(order_info.items.len(), 1);
        assert_eq!(order_info.items[0].unit_price, 1000);
        assert_eq!(order_info.items[0].quantity, 1);
    }

    /// hobbysearch_change_yoyaku パーサーのテスト（CIで実行、ダミーデータ使用）
    const SAMPLE_HOBBYSEARCH_CHANGE_YOYAKU: &str = r#"
[注文番号] 25-0303-9999

[お届け先情報]
〒200-0002
東京都中央区銀座1-2-3
予約 太郎 様

[ご予約内容]
バンダイ 2345678 テスト商品B (プラモデル) MGシリーズ
単価：3,000円 × 個数：2 = 6,000円

予約商品合計：6,000円
"#;

    #[test]
    fn test_parse_email_hobbysearch_change_yoyaku() {
        let result = parse_email(
            "hobbysearch_change_yoyaku".to_string(),
            SAMPLE_HOBBYSEARCH_CHANGE_YOYAKU.to_string(),
        );
        assert!(result.is_ok());
        let order_info = result.unwrap();
        assert_eq!(order_info.order_number, "25-0303-9999");
        assert_eq!(order_info.items.len(), 1);
        assert_eq!(order_info.items[0].unit_price, 3000);
        assert_eq!(order_info.items[0].quantity, 2);
    }

    /// hobbysearch_confirm_yoyaku パーサーのテスト（CIで実行、ダミーデータ使用）
    const SAMPLE_HOBBYSEARCH_CONFIRM_YOYAKU: &str = r#"
[注文番号] 25-0505-2222

[お届け先情報]
〒300-0003
東京都港区六本木1-2-3
予約 次郎 様

[ご予約内容]
バンダイ 3456789 テスト商品D (プラモデル) RGシリーズ
単価：2,500円 × 個数：2 = 5,000円

予約商品合計 5,000円
"#;

    #[test]
    fn test_parse_email_hobbysearch_confirm_yoyaku() {
        let result = parse_email(
            "hobbysearch_confirm_yoyaku".to_string(),
            SAMPLE_HOBBYSEARCH_CONFIRM_YOYAKU.to_string(),
        );
        assert!(result.is_ok());
        let order_info = result.unwrap();
        assert_eq!(order_info.order_number, "25-0505-2222");
        assert_eq!(order_info.items.len(), 1);
        assert_eq!(order_info.items[0].unit_price, 2500);
        assert_eq!(order_info.items[0].quantity, 2);
        assert_eq!(order_info.subtotal, Some(5000));
    }

    /// hobbysearch_send パーサーのテスト（CIで実行、ダミーデータ使用）
    const SAMPLE_HOBBYSEARCH_SEND: &str = r#"
[代表注文番号] 25-0404-1111

[運送会社] ヤマト運輸
[配送伝票] 1234-5678-9012

[お届け先情報]
〒300-0003
東京都港区六本木1-2-3
発送 次郎 様

[ご購入内容]
バンダイ 3456789 テスト商品C (プラモデル) RGシリーズ
単価：2,000円 × 個数：1 = 2,000円

小計：2,000円
送料：0円
合計：2,000円
"#;

    #[test]
    fn test_parse_email_hobbysearch_send() {
        let result = parse_email(
            "hobbysearch_send".to_string(),
            SAMPLE_HOBBYSEARCH_SEND.to_string(),
        );
        assert!(result.is_ok());
        let order_info = result.unwrap();
        assert_eq!(order_info.order_number, "25-0404-1111");
        assert_eq!(order_info.items.len(), 1);
        assert!(order_info.delivery_info.is_some());
        let info = order_info.delivery_info.as_ref().unwrap();
        assert_eq!(info.carrier, "ヤマト運輸");
        assert_eq!(info.tracking_number, "1234-5678-9012");
    }

    // ==================== validate_window_size Tests ====================

    #[test]
    fn test_validate_window_size_valid() {
        assert!(validate_window_size(200, 200).is_ok());
        assert!(validate_window_size(1000, 800).is_ok());
        assert!(validate_window_size(10000, 10000).is_ok());
    }

    #[test]
    fn test_validate_window_size_width_too_small() {
        let result = validate_window_size(199, 500);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("幅"));
    }

    #[test]
    fn test_validate_window_size_width_too_large() {
        let result = validate_window_size(10001, 500);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("幅"));
    }

    #[test]
    fn test_validate_window_size_height_too_small() {
        let result = validate_window_size(500, 199);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("高さ"));
    }

    #[test]
    fn test_validate_window_size_height_too_large() {
        let result = validate_window_size(500, 10001);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("高さ"));
    }

    // ==================== validate_max_iterations Tests ====================

    #[test]
    fn test_validate_max_iterations_valid() {
        assert!(validate_max_iterations(1).is_ok());
        assert!(validate_max_iterations(100).is_ok());
    }

    #[test]
    fn test_validate_max_iterations_zero() {
        let result = validate_max_iterations(0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("1以上"));
    }

    #[test]
    fn test_validate_max_iterations_negative() {
        let result = validate_max_iterations(-1);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_image_url_https_ok() {
        assert!(validate_image_url("https://example.com/image.png").is_ok());
        assert!(validate_image_url("https://images.example.co.jp/photo.jpg").is_ok());
    }

    #[test]
    fn test_validate_image_url_http_rejected() {
        assert!(validate_image_url("http://example.com/image.png").is_err());
    }

    #[test]
    fn test_validate_image_url_localhost_rejected() {
        assert!(validate_image_url("https://localhost/image.png").is_err());
        assert!(validate_image_url("https://127.0.0.1/image.png").is_err());
    }

    #[test]
    fn test_validate_image_url_private_ip_rejected() {
        assert!(validate_image_url("https://192.168.1.1/image.png").is_err());
        assert!(validate_image_url("https://10.0.0.1/image.png").is_err());
    }

    #[test]
    fn test_validate_image_url_metadata_rejected() {
        assert!(validate_image_url("https://169.254.169.254/").is_err());
    }

    #[test]
    fn test_is_private_ip() {
        use std::net::IpAddr;
        assert!(is_private_ip("10.0.0.1".parse().unwrap()));
        assert!(is_private_ip("192.168.1.1".parse().unwrap()));
        assert!(is_private_ip("172.16.0.1".parse().unwrap()));
        assert!(!is_private_ip("8.8.8.8".parse::<IpAddr>().unwrap()));
    }
}
