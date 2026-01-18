use tauri::{Emitter, Manager};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri_plugin_sql::{Migration, MigrationKind};
use sqlx::sqlite::SqlitePool;

mod gmail;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
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
    let app_clone = app_handle.clone();

    tauri::async_runtime::spawn(async move {
        if let Err(e) = gmail::sync_gmail_incremental(&app_clone, &pool_clone, &sync_state_clone, 50).await {
            log::error!("Sync failed: {}", e);

            // Emit error event
            let error_msg = e.clone();
            let error_event = gmail::SyncProgressEvent {
                batch_number: 0,
                batch_size: 0,
                total_synced: 0,
                newly_saved: 0,
                status_message: format!("Sync error: {}", e),
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
async fn cancel_sync(
    sync_state: tauri::State<'_, gmail::SyncState>,
) -> Result<(), String> {
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
    .map_err(|e| format!("Failed to fetch sync status: {}", e))?;

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
         WHERE id = 1 AND sync_status = 'syncing'"
    )
    .execute(pool.inner())
    .await
    .map_err(|e| format!("Failed to reset sync status: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn update_batch_size(
    pool: tauri::State<'_, SqlitePool>,
    batch_size: i64,
) -> Result<(), String> {
    log::info!("Updating batch size to: {}", batch_size);

    sqlx::query(
        "UPDATE sync_metadata SET batch_size = ?1 WHERE id = 1"
    )
    .bind(batch_size)
    .execute(pool.inner())
    .await
    .map_err(|e| format!("Failed to update batch size: {}", e))?;

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

    log::info!("Updating max iterations to: {}", max_iterations);

    sqlx::query(
        "UPDATE sync_metadata SET max_iterations = ?1 WHERE id = 1"
    )
    .bind(max_iterations)
    .execute(pool.inner())
    .await
    .map_err(|e| format!("Failed to update max iterations: {}", e))?;

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
async fn get_window_settings(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<WindowSettings, String> {
    let row: (i64, i64, Option<i64>, Option<i64>, i64) = sqlx::query_as(
        "SELECT width, height, x, y, maximized FROM window_settings WHERE id = 1"
    )
    .fetch_one(pool.inner())
    .await
    .map_err(|e| format!("Failed to fetch window settings: {}", e))?;

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

    if width < MIN_SIZE || width > MAX_SIZE {
        return Err(format!("ウィンドウの幅は{}〜{}の範囲である必要があります", MIN_SIZE, MAX_SIZE));
    }
    if height < MIN_SIZE || height > MAX_SIZE {
        return Err(format!("ウィンドウの高さは{}〜{}の範囲である必要があります", MIN_SIZE, MAX_SIZE));
    }

    sqlx::query(
        "UPDATE window_settings SET width = ?1, height = ?2, x = ?3, y = ?4, maximized = ?5 WHERE id = 1"
    )
    .bind(width)
    .bind(height)
    .bind(x)
    .bind(y)
    .bind(if maximized { 1 } else { 0 })
    .execute(pool.inner())
    .await
    .map_err(|e| format!("Failed to save window settings: {}", e))?;

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
        }
    ];

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            // ロガーの初期化
            env_logger::Builder::from_default_env()
                .filter_level(log::LevelFilter::Info)
                .init();

            let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");

            let db_path = app_data_dir.join("paa_data.db");
            let db_url = format!("sqlite:{}", db_path.to_string_lossy());

            log::info!("Database path: {}", db_path.display());

            // tauri-plugin-sqlを登録（フロントエンド用、マイグレーションも管理）
            app.handle().plugin(
                tauri_plugin_sql::Builder::default()
                    .add_migrations(&db_url, migrations)
                    .build()
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

            // Restore window settings and setup close handler
            let window = app.get_webview_window("main").expect("Failed to get main window");

            // Handle window close request - hide instead of closing
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_clone.hide();
                }
            });

            let pool_for_window = pool.clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(settings) = sqlx::query_as::<_, (i64, i64, Option<i64>, Option<i64>, i64)>(
                    "SELECT width, height, x, y, maximized FROM window_settings WHERE id = 1"
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
                        let _ = window.set_position(tauri::LogicalPosition {
                            x: x_pos as i32,
                            y: y_pos as i32,
                        });
                    }

                    // Set maximized state
                    if maximized != 0 {
                        let _ = window.maximize();
                    }

                    log::info!("Window settings restored: {}x{}", width, height);
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
                log::warn!("No default window icon found; initializing system tray without a custom icon.");
            }

            let _tray = tray_builder
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
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
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            ..
                        } => {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            log::info!("System tray initialized");

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
            get_window_settings,
            save_window_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
}
