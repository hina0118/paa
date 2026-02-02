use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Listener, Manager};
use tauri_plugin_sql::{Migration, MigrationKind};

pub mod gemini;
pub mod gmail;
pub mod gmail_client;
pub mod logic;
pub mod parsers;
pub mod repository;

use crate::logic::email_parser::get_candidate_parsers;
use crate::repository::{
    EmailStats, EmailStatsRepository, OrderRepository, ParseMetadataRepository,
    ShopSettingsRepository, SqliteEmailStatsRepository, SqliteOrderRepository,
    SqliteParseMetadataRepository, SqliteProductMasterRepository, SqliteShopSettingsRepository,
    SqliteSyncMetadataRepository, SqliteWindowSettingsRepository, SyncMetadataRepository,
    WindowSettings, WindowSettingsRepository,
};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[tauri::command]
async fn start_sync(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    sync_state: tauri::State<'_, gmail::SyncState>,
) -> Result<(), String> {
    log::info!("Starting incremental Gmail sync...");

    // Spawn async task to avoid blocking
    let pool_clone = pool.inner().clone();
    let sync_state_clone = sync_state.inner().clone();
    let app_clone = app_handle;

    tauri::async_runtime::spawn(async move {
        if let Err(e) =
            gmail::sync_gmail_incremental(&app_clone, &pool_clone, &sync_state_clone, 50).await
        {
            log::error!("Sync failed: {e}");

            // Emit error event
            let error_msg = e.clone();
            let error_event = gmail::SyncProgressEvent {
                batch_number: 0,
                batch_size: 0,
                total_synced: 0,
                newly_saved: 0,
                status_message: format!("Sync error: {e}"),
                is_complete: true,
                error: Some(error_msg),
            };

            let _ = app_clone.emit("sync-progress", error_event);

            // Update database status to error
            let repo = SqliteSyncMetadataRepository::new(pool_clone.clone());
            let _ = repo.update_error_status(&e).await;
        }
    });

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
    pool: tauri::State<'_, SqlitePool>,
) -> Result<gmail::SyncMetadata, String> {
    let repo = SqliteSyncMetadataRepository::new(pool.inner().clone());
    let metadata = repo.get_sync_metadata().await?;
    Ok(metadata)
}

#[tauri::command]
async fn reset_sync_status(pool: tauri::State<'_, SqlitePool>) -> Result<(), String> {
    log::info!("Resetting stuck sync status to 'idle'");
    let repo = SqliteSyncMetadataRepository::new(pool.inner().clone());
    repo.reset_sync_status().await
}

#[tauri::command]
async fn reset_sync_date(pool: tauri::State<'_, SqlitePool>) -> Result<(), String> {
    log::info!("Resetting oldest_fetched_date to allow re-sync from latest emails");
    let repo = SqliteSyncMetadataRepository::new(pool.inner().clone());
    repo.reset_sync_date().await
}

#[tauri::command]
async fn update_batch_size(
    pool: tauri::State<'_, SqlitePool>,
    batch_size: i64,
) -> Result<(), String> {
    log::info!("Updating batch size to: {batch_size}");
    let repo = SqliteSyncMetadataRepository::new(pool.inner().clone());
    repo.update_batch_size(batch_size).await
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
    pool: tauri::State<'_, SqlitePool>,
    max_iterations: i64,
) -> Result<(), String> {
    validate_max_iterations(max_iterations)?;

    log::info!("Updating max iterations to: {max_iterations}");
    let repo = SqliteSyncMetadataRepository::new(pool.inner().clone());
    repo.update_max_iterations(max_iterations).await
}

#[tauri::command]
async fn get_window_settings(pool: tauri::State<'_, SqlitePool>) -> Result<WindowSettings, String> {
    let repo = SqliteWindowSettingsRepository::new(pool.inner().clone());
    repo.get_window_settings().await
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
    pool: tauri::State<'_, SqlitePool>,
    width: i64,
    height: i64,
    x: Option<i64>,
    y: Option<i64>,
    maximized: bool,
) -> Result<(), String> {
    validate_window_size(width, height)?;

    let repo = SqliteWindowSettingsRepository::new(pool.inner().clone());
    let settings = WindowSettings {
        width,
        height,
        x,
        y,
        maximized,
    };
    repo.save_window_settings(settings).await
}

#[tauri::command]
async fn fetch_gmail_emails(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    sync_state: tauri::State<'_, gmail::SyncState>,
) -> Result<gmail::FetchResult, String> {
    log::info!("Starting Gmail email fetch (via start_sync)...");
    log::info!("If a browser window doesn't open automatically, please check the console for the authentication URL.");

    // Use the new incremental sync internally
    gmail::sync_gmail_incremental(&app_handle, pool.inner(), sync_state.inner(), 50).await?;

    // Return a simple result (actual progress is sent via events)
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
    let migrations = vec![
        Migration {
            version: 1,
            description: "init",
            sql: include_str!("../migrations/001_init.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 2,
            description: "product_master",
            sql: include_str!("../migrations/002_product_master.sql"),
            kind: MigrationKind::Up,
        },
    ];

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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

            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");

            let db_path = app_data_dir.join("paa_data.db");
            let db_url = format!("sqlite:{}", db_path.to_string_lossy());

            log::info!("Database path: {}", db_path.display());

            // tauri-plugin-sqlを登録（フロントエンド用、マイグレーションも管理）
            app.handle().plugin(
                tauri_plugin_sql::Builder::default()
                    .add_migrations(&db_url, migrations)
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

            let pool_for_window = pool;
            tauri::async_runtime::spawn(async move {
                let repo = SqliteWindowSettingsRepository::new(pool_for_window.clone());
                if let Ok(settings) = repo.get_window_settings().await {
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
                }
            });

            // Setup system tray
            let show_item = MenuItem::with_id(app, "show", "表示", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

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
            fetch_gmail_emails,
            start_sync,
            cancel_sync,
            get_sync_status,
            update_batch_size,
            update_max_iterations,
            reset_sync_status,
            reset_sync_date,
            get_window_settings,
            save_window_settings,
            get_email_stats,
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
            // Gemini API commands
            has_gemini_api_key,
            save_gemini_api_key,
            delete_gemini_api_key,
            start_product_name_parse,
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

#[tauri::command]
async fn start_batch_parse(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    parse_state: tauri::State<'_, parsers::ParseState>,
    batch_size: Option<usize>,
) -> Result<(), String> {
    log::info!("Starting batch parse...");

    // batch_sizeが指定されていない場合はparse_metadataから取得
    let size = if let Some(size) = batch_size {
        size
    } else {
        let repo = SqliteParseMetadataRepository::new(pool.inner().clone());
        let batch_size = repo.get_batch_size().await?;
        batch_size as usize
    };

    let pool_clone = pool.inner().clone();
    let parse_state_clone = parse_state.inner().clone();

    tauri::async_runtime::spawn(async move {
        if let Err(e) =
            parsers::batch_parse_emails(&app_handle, &pool_clone, &parse_state_clone, size).await
        {
            log::error!("Batch parse failed: {}", e);

            // エラーイベントを送信
            let error_event = parsers::ParseProgressEvent {
                batch_number: 0,
                total_emails: 0,
                parsed_count: 0,
                success_count: 0,
                failed_count: 0,
                status_message: format!("Parse error: {}", e),
                is_complete: true,
                error: Some(e.clone()),
            };

            let _ = app_handle.emit("parse-progress", error_event);

            // データベースのステータスをエラーに更新
            let repo = SqliteParseMetadataRepository::new(pool_clone.clone());
            let _ = repo
                .update_parse_status("error", None, None, None, Some(e.clone()))
                .await;
        }
    });

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
    pool: tauri::State<'_, SqlitePool>,
) -> Result<parsers::ParseMetadata, String> {
    let repo = SqliteParseMetadataRepository::new(pool.inner().clone());
    repo.get_parse_metadata().await
}

#[tauri::command]
async fn update_parse_batch_size(
    pool: tauri::State<'_, SqlitePool>,
    batch_size: i64,
) -> Result<(), String> {
    log::info!("Updating parse batch size to: {batch_size}");
    let repo = SqliteParseMetadataRepository::new(pool.inner().clone());
    repo.update_batch_size(batch_size).await
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

/// 商品名パース進捗イベント
#[derive(Debug, Clone, serde::Serialize)]
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
#[tauri::command]
async fn start_product_name_parse(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    parse_state: tauri::State<'_, ProductNameParseState>,
) -> Result<(), String> {
    use tauri::Emitter;

    log::info!("Starting product name parse with Gemini API...");

    // 失敗し得る初期化を先に行い、try_start() は spawn 直前に呼ぶ
    // （早期 return 時に finish() が呼ばれず「実行中」のままになるのを防ぐ）
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {e}"))?;

    if !gemini::has_api_key(&app_data_dir) {
        return Err(
            "Gemini APIキーが設定されていません。設定画面でAPIキーを設定してください。"
                .to_string(),
        );
    }

    let api_key = gemini::load_api_key(&app_data_dir)?;
    let gemini_client = gemini::GeminiClient::new(api_key)?;
    let product_repo = SqliteProductMasterRepository::new(pool.inner().clone());
    let service = gemini::ProductParseService::new(gemini_client, product_repo);

    // 多重実行ガード（初期化成功後にのみ取得）
    parse_state.try_start()?;

    let parse_state_clone = parse_state.inner().clone();
    let pool_clone = pool.inner().clone();

    tauri::async_runtime::spawn(async move {
        // items テーブルから product_master に未登録の商品名のみを取得
        // 商品名単位で一意にするため GROUP BY TRIM(i.item_name) で集約
        let items: Vec<(String, Option<String>)> = match sqlx::query_as(
            r#"
            SELECT
              TRIM(i.item_name) AS item_name,
              MIN(o.shop_domain) AS shop_domain
            FROM items i
            JOIN orders o ON i.order_id = o.id
            LEFT JOIN product_master pm ON TRIM(i.item_name) = pm.raw_name
            WHERE i.item_name IS NOT NULL
              AND i.item_name != ''
              AND pm.id IS NULL
            GROUP BY TRIM(i.item_name)
            "#,
        )
        .fetch_all(&pool_clone)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                log::error!("Failed to fetch unparsed items: {}", e);
                let error_event = ProductNameParseProgress {
                    total_items: 0,
                    parsed_count: 0,
                    success_count: 0,
                    failed_count: 0,
                    status_message: format!("商品情報の取得に失敗: {}", e),
                    is_complete: true,
                    error: Some(e.to_string()),
                };
                let _ = app_handle.emit("product-name-parse-progress", error_event);
                parse_state_clone.finish();
                return;
            }
        };

        let total_items = items.len();
        log::info!(
            "Found {} unparsed items (not in product_master)",
            total_items
        );

        if total_items == 0 {
            let complete_event = ProductNameParseProgress {
                total_items: 0,
                parsed_count: 0,
                success_count: 0,
                failed_count: 0,
                status_message: "未解析の商品はありません（すべてproduct_masterに登録済み）"
                    .to_string(),
                is_complete: true,
                error: None,
            };
            let _ = app_handle.emit("product-name-parse-progress", complete_event);
            parse_state_clone.finish();
            return;
        }

        // 進捗開始イベント
        let start_event = ProductNameParseProgress {
            total_items,
            parsed_count: 0,
            success_count: 0,
            failed_count: 0,
            status_message: format!("商品名パース開始: {} 件（未解析分）", total_items),
            is_complete: false,
            error: None,
        };
        let _ = app_handle.emit("product-name-parse-progress", start_event);

        // Gemini API でバッチ処理（client.rs 内で10件ずつ + 10秒ディレイ）
        // ProductParseService.parse_products_batch は内部で product_master へ保存する
        match service.parse_products_batch(&items).await {
            Ok(batch_result) => {
                let success_count = batch_result.success_count;
                let failed_count = batch_result.failed_count;

                log::info!(
                    "Product name parse completed: success={}, failed={} (requested: {})",
                    success_count,
                    failed_count,
                    total_items
                );

                let complete_event = ProductNameParseProgress {
                    total_items,
                    parsed_count: total_items,
                    success_count,
                    failed_count,
                    status_message: format!(
                        "商品名パース完了: 成功 {} 件、失敗 {} 件（リクエスト件数: {}）",
                        success_count,
                        failed_count,
                        total_items
                    ),
                    is_complete: true,
                    error: None,
                };
                let _ = app_handle.emit("product-name-parse-progress", complete_event);
            }
            Err(e) => {
                log::error!("Gemini API batch parse failed: {}", e);
                let error_event = ProductNameParseProgress {
                    total_items,
                    parsed_count: 0,
                    success_count: 0,
                    failed_count: total_items,
                    status_message: format!("Gemini API エラー: {}", e),
                    is_complete: true,
                    error: Some(e),
                };
                let _ = app_handle.emit("product-name-parse-progress", error_event);
            }
        }

        // 多重実行ガード解除（成功・失敗・エラー問わず必ず呼ぶ）
        parse_state_clone.finish();
    });

    Ok(())
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
}
