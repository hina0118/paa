use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Listener, Manager};
use tauri_plugin_sql::{Migration, MigrationKind};

pub mod gmail;
pub mod gmail_client;
pub mod logic;
pub mod parsers;
pub mod repository;

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
            let _ = sqlx::query(
                "UPDATE sync_metadata SET sync_status = 'error', last_error_message = ?1 WHERE id = 1"
            )
            .bind(&e)
            .execute(&pool_clone)
            .await;
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
    let row: (String, Option<String>, i64, i64, Option<String>, Option<String>, i64) = sqlx::query_as(
        "SELECT sync_status, oldest_fetched_date, total_synced_count, batch_size, last_sync_started_at, last_sync_completed_at, max_iterations FROM sync_metadata WHERE id = 1"
    )
    .fetch_one(pool.inner())
    .await
    .map_err(|e| format!("Failed to fetch sync status: {e}"))?;

    Ok(gmail::SyncMetadata {
        sync_status: row.0,
        oldest_fetched_date: row.1,
        total_synced_count: row.2,
        batch_size: row.3,
        last_sync_started_at: row.4,
        last_sync_completed_at: row.5,
        max_iterations: row.6,
    })
}

#[tauri::command]
async fn reset_sync_status(pool: tauri::State<'_, SqlitePool>) -> Result<(), String> {
    log::info!("Resetting stuck sync status to 'idle'");

    sqlx::query(
        "UPDATE sync_metadata
         SET sync_status = 'idle'
         WHERE id = 1 AND sync_status = 'syncing'",
    )
    .execute(pool.inner())
    .await
    .map_err(|e| format!("Failed to reset sync status: {e}"))?;

    Ok(())
}

#[tauri::command]
async fn reset_sync_date(pool: tauri::State<'_, SqlitePool>) -> Result<(), String> {
    log::info!("Resetting oldest_fetched_date to allow re-sync from latest emails");

    sqlx::query(
        "UPDATE sync_metadata
         SET oldest_fetched_date = NULL
         WHERE id = 1",
    )
    .execute(pool.inner())
    .await
    .map_err(|e| format!("Failed to reset sync date: {e}"))?;

    Ok(())
}

#[tauri::command]
async fn update_batch_size(
    pool: tauri::State<'_, SqlitePool>,
    batch_size: i64,
) -> Result<(), String> {
    log::info!("Updating batch size to: {batch_size}");

    sqlx::query("UPDATE sync_metadata SET batch_size = ?1 WHERE id = 1")
        .bind(batch_size)
        .execute(pool.inner())
        .await
        .map_err(|e| format!("Failed to update batch size: {e}"))?;

    Ok(())
}

#[tauri::command]
async fn update_max_iterations(
    pool: tauri::State<'_, SqlitePool>,
    max_iterations: i64,
) -> Result<(), String> {
    if max_iterations <= 0 {
        return Err("最大繰り返し回数は1以上である必要があります".to_string());
    }

    log::info!("Updating max iterations to: {max_iterations}");

    sqlx::query("UPDATE sync_metadata SET max_iterations = ?1 WHERE id = 1")
        .bind(max_iterations)
        .execute(pool.inner())
        .await
        .map_err(|e| format!("Failed to update max iterations: {e}"))?;

    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct WindowSettings {
    width: i64,
    height: i64,
    x: Option<i64>,
    y: Option<i64>,
    maximized: bool,
}

#[tauri::command]
async fn get_window_settings(pool: tauri::State<'_, SqlitePool>) -> Result<WindowSettings, String> {
    let row: (i64, i64, Option<i64>, Option<i64>, i64) =
        sqlx::query_as("SELECT width, height, x, y, maximized FROM window_settings WHERE id = 1")
            .fetch_one(pool.inner())
            .await
            .map_err(|e| format!("Failed to fetch window settings: {e}"))?;

    Ok(WindowSettings {
        width: row.0,
        height: row.1,
        x: row.2,
        y: row.3,
        maximized: row.4 != 0,
    })
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
    // Validate window size (minimum 200, maximum 10000)
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

    sqlx::query(
        "UPDATE window_settings SET width = ?1, height = ?2, x = ?3, y = ?4, maximized = ?5 WHERE id = 1"
    )
    .bind(width)
    .bind(height)
    .bind(x)
    .bind(y)
    .bind(i32::from(maximized))
    .execute(pool.inner())
    .await
    .map_err(|e| format!("Failed to save window settings: {e}"))?;

    Ok(())
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

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailStats {
    pub total_emails: i64,
    pub with_body_plain: i64,
    pub with_body_html: i64,
    pub without_body: i64,
    pub avg_plain_length: f64,
    pub avg_html_length: f64,
}

/// メール統計情報を取得
///
/// CTEを使用してLENGTH計算を一度だけ実行し、パフォーマンスを最適化
#[tauri::command]
async fn get_email_stats(pool: tauri::State<'_, SqlitePool>) -> Result<EmailStats, String> {
    let stats: (i64, i64, i64, i64, Option<f64>, Option<f64>) = sqlx::query_as(
        r"
        WITH email_lengths AS (
            SELECT
                body_plain,
                body_html,
                CASE WHEN body_plain IS NOT NULL THEN LENGTH(body_plain) ELSE 0 END AS plain_length,
                CASE WHEN body_html IS NOT NULL THEN LENGTH(body_html) ELSE 0 END AS html_length
            FROM emails
        )
        SELECT
            COUNT(*) AS total,
            COUNT(CASE WHEN body_plain IS NOT NULL AND plain_length > 0 THEN 1 END) AS with_plain,
            COUNT(CASE WHEN body_html IS NOT NULL AND html_length > 0 THEN 1 END) AS with_html,
            COUNT(CASE WHEN (body_plain IS NULL OR plain_length = 0) AND (body_html IS NULL OR html_length = 0) THEN 1 END) AS without_body,
            AVG(CASE WHEN body_plain IS NOT NULL AND plain_length > 0 THEN plain_length END) AS avg_plain,
            AVG(CASE WHEN body_html IS NOT NULL AND html_length > 0 THEN html_length END) AS avg_html
        FROM email_lengths
        "
    )
    .fetch_one(pool.inner())
    .await
    .map_err(|e| format!("Failed to fetch email stats: {e}"))?;

    Ok(EmailStats {
        total_emails: stats.0,
        with_body_plain: stats.1,
        with_body_html: stats.2,
        without_body: stats.3,
        avg_plain_length: stats.4.unwrap_or(0.0),
        avg_html_length: stats.5.unwrap_or(0.0),
    })
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
                    timestamp: chrono::Local::now()
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
            description: "create initial tables",
            sql: include_str!("../migrations/001_initial_tables.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 2,
            description: "create emails table",
            sql: include_str!("../migrations/002_create_emails_table.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 3,
            description: "create orders table",
            sql: include_str!("../migrations/003_create_orders_table.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 4,
            description: "create items table",
            sql: include_str!("../migrations/004_create_items_table.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 5,
            description: "create images table",
            sql: include_str!("../migrations/005_create_images_table.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 6,
            description: "create deliveries table",
            sql: include_str!("../migrations/006_create_deliveries_table.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 7,
            description: "create htmls table",
            sql: include_str!("../migrations/007_create_htmls_table.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 8,
            description: "create order_emails table",
            sql: include_str!("../migrations/008_create_order_emails_table.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 9,
            description: "create order_htmls table",
            sql: include_str!("../migrations/009_create_order_htmls_table.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 10,
            description: "create sync_metadata table",
            sql: include_str!("../migrations/010_create_sync_metadata_table.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 11,
            description: "add internal_date to emails",
            sql: include_str!("../migrations/011_add_internal_date_to_emails.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 12,
            description: "add max_iterations to sync_metadata",
            sql: include_str!("../migrations/012_add_max_iterations_to_sync_metadata.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 13,
            description: "create window_settings table",
            sql: include_str!("../migrations/013_create_window_settings_table.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 14,
            description: "create shop_settings table",
            sql: include_str!("../migrations/014_create_shop_settings_table.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 15,
            description: "add subject_filters to shop_settings",
            sql: include_str!("../migrations/015_add_subject_filter_to_shop_settings.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 16,
            description: "create parse_metadata table",
            sql: include_str!("../migrations/016_create_parse_metadata_table.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 17,
            description: "add from_address to emails",
            sql: include_str!("../migrations/017_add_from_address_to_emails.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 18,
            description: "add batch_size to parse_metadata",
            sql: include_str!("../migrations/018_add_batch_size_to_parse_metadata.sql"),
            kind: MigrationKind::Up,
        },
        Migration {
            version: 19,
            description: "add subject to emails",
            sql: include_str!("../migrations/019_add_subject_to_emails.sql"),
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

                    // コンソールにも出力
                    writeln!(
                        buf,
                        "[{} {:5} {}] {}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
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
                if let Ok(settings) =
                    sqlx::query_as::<_, (i64, i64, Option<i64>, Option<i64>, i64)>(
                        "SELECT width, height, x, y, maximized FROM window_settings WHERE id = 1",
                    )
                    .fetch_one(&pool_for_window)
                    .await
                {
                    let (width, height, x, y, maximized) = settings;

                    // Set window size
                    let _ = window.set_size(tauri::LogicalSize {
                        width: width as u32,
                        height: height as u32,
                    });

                    // Set window position if available
                    if let (Some(x_pos), Some(y_pos)) = (x, y) {
                        #[allow(clippy::cast_possible_truncation)]
                        let _ = window.set_position(tauri::LogicalPosition {
                            x: x_pos as i32,
                            y: y_pos as i32,
                        });
                    }

                    // Set maximized state
                    if maximized != 0 {
                        let _ = window.maximize();
                    }

                    log::info!("Window settings restored: {width}x{height}");
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
            update_parse_batch_size
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
    let shop_settings: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT sender_address, parser_type, subject_filters FROM shop_settings WHERE is_enabled = 1"
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| format!("Failed to fetch shop settings: {}", e))?;

    // 送信元アドレスと件名フィルターから候補のパーサータイプを全て取得
    let candidate_parsers: Vec<&str> = shop_settings
        .iter()
        .filter_map(|(addr, parser_type, subject_filters_json)| {
            // 送信元アドレスが一致するか確認
            if !sender_address.contains(addr) {
                return None;
            }

            // 件名フィルターがない場合は、アドレス一致だけでOK
            let Some(filters_json) = subject_filters_json else {
                return Some(parser_type.as_str());
            };

            // 件名フィルターがある場合は、件名も確認
            let Ok(filters) = serde_json::from_str::<Vec<String>>(filters_json) else {
                return Some(parser_type.as_str()); // JSONパースエラー時はフィルター無視
            };

            // 件名がない場合は除外
            let subj = subject.as_ref()?;

            // いずれかのフィルターに一致すればOK
            if filters.iter().any(|filter| subj.contains(filter)) {
                Some(parser_type.as_str())
            } else {
                None
            }
        })
        .collect();

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
    parsers::save_order_to_db(pool.inner(), &order_info, email_id, shop_domain.as_deref()).await
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
        let row: (i64,) = sqlx::query_as("SELECT batch_size FROM parse_metadata WHERE id = 1")
            .fetch_one(pool.inner())
            .await
            .map_err(|e| format!("Failed to fetch batch size: {}", e))?;
        row.0 as usize
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
            let _ = sqlx::query(
                "UPDATE parse_metadata SET parse_status = 'error', last_error_message = ?1 WHERE id = 1"
            )
            .bind(&e)
            .execute(&pool_clone)
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
    let row: (String, Option<String>, Option<String>, i64, Option<String>, i64) = sqlx::query_as(
        "SELECT parse_status, last_parse_started_at, last_parse_completed_at, total_parsed_count, last_error_message, batch_size FROM parse_metadata WHERE id = 1"
    )
    .fetch_one(pool.inner())
    .await
    .map_err(|e| format!("Failed to fetch parse status: {}", e))?;

    Ok(parsers::ParseMetadata {
        parse_status: row.0,
        last_parse_started_at: row.1,
        last_parse_completed_at: row.2,
        total_parsed_count: row.3,
        last_error_message: row.4,
        batch_size: row.5,
    })
}

#[tauri::command]
async fn update_parse_batch_size(
    pool: tauri::State<'_, SqlitePool>,
    batch_size: i64,
) -> Result<(), String> {
    log::info!("Updating parse batch size to: {batch_size}");

    sqlx::query("UPDATE parse_metadata SET batch_size = ?1 WHERE id = 1")
        .bind(batch_size)
        .execute(pool.inner())
        .await
        .map_err(|e| format!("Failed to update parse batch size: {}", e))?;

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
}
