use sqlx::sqlite::SqlitePool;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem, Submenu};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{Listener, Manager};
use tauri_plugin_sql::{Migration, MigrationKind};

pub mod batch_runner;
pub mod clipboard_watcher;
pub mod commands;
pub mod config;
pub mod delivery_check;
pub mod e2e_mocks;
pub mod e2e_seed;
pub mod gemini;
pub mod gmail;
pub mod gmail_client;
pub mod google_search;
pub mod image_utils;
pub mod logic;
pub mod metadata;
pub mod orchestration;
pub mod parsers;
pub mod plugins;
pub mod repository;

/// items_fts の trigram トークナイザーは SQLite 3.43 で追加。3.43 以降であることを確認する。
fn is_sqlite_version_supported(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    let major: u32 = parts[0].parse().unwrap_or(0);
    let minor: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    major > 3 || (major == 3 && minor >= 43)
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
            commands::init_log_buffer();

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
                    commands::add_log_entry(&record.level().to_string(), &format!("{}", record.args()));

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

            // クリップボード監視（画像URL検知 → フロントへ通知）
            // 例外があってもクラッシュしないように監視側で吸収する
            {
                let app_handle = app.handle().clone();
                let config = clipboard_watcher::WatcherConfig::default();

                // グレースフルシャットダウン用のシグナル
                let shutdown_signal = Arc::new(AtomicBool::new(false));
                let shutdown_signal_clone = shutdown_signal.clone();

                // shutdown_signal を app state として管理し、quit ハンドラからアクセス可能にする
                app.manage(shutdown_signal.clone());

                // 監視スレッドをバックグラウンドで起動（終了は shutdown_signal で制御）
                tauri::async_runtime::spawn_blocking(move || {
                    clipboard_watcher::run_clipboard_watcher(app_handle, config, shutdown_signal_clone);
                });
            }

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

                // DB接続オプション（create_if_missing 有効化、foreign_keys でテストと挙動を統一）
                // WAL モードで tauri-plugin-sql（フロントエンド）との lock 競合を解消。
                // DELETE ジャーナルモード（デフォルト）では、フロントエンドの SHARED LOCK が
                // バックエンドの INSERT を即時ブロックして "database is locked" (code 5) が発生する。
                // WAL モードでは reader が writer をブロックしないため競合が解消される。
                // busy_timeout: tauri-plugin-sql 側の接続が書き込み中の場合に最大 10 秒待機してリトライ。
                let options = SqliteConnectOptions::from_str(&db_url)
                    .expect("Failed to parse database URL")
                    .create_if_missing(true)
                    .foreign_keys(true)
                    .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                    .busy_timeout(std::time::Duration::from_secs(10));

                // DB接続プール作成
                let pool = SqlitePoolOptions::new()
                    .connect_with(options)
                    .await
                    .expect("Failed to create sqlx pool");

                // items_fts の trigram トークナイザーは SQLite 3.43+ が前提。起動時にバージョンチェック
                let version: (String,) =
                    sqlx::query_as("SELECT sqlite_version()")
                        .fetch_one(&pool)
                        .await
                        .expect("Failed to query SQLite version");
                let v = version.0.as_str();
                if !is_sqlite_version_supported(v) {
                    panic!(
                        "SQLite 3.43 以降が必要です（現在: {}）。trigram FTS5 を使用しています。\
                         bundled SQLite を使うには sqlx の sqlite feature を確認してください。",
                        v
                    );
                }
                log::info!("SQLite version: {} (trigram FTS5 supported)", v);

                pool
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
            app.manage(commands::ProductNameParseState::new());
            log::info!("Product name parse state initialized");

            // Initialize delivery check state
            app.manage(commands::DeliveryCheckState::new());
            log::info!("Delivery check state initialized");

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
            let delivery_check_item = MenuItem::with_id(
                app,
                "tray_delivery_check",
                "配送状況確認",
                true,
                None::<&str>,
            )?;
            let batch_submenu = Submenu::with_id_and_items(
                app,
                "batch",
                "バッチ処理",
                true,
                &[&sync_item, &parse_item, &product_item, &delivery_check_item],
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
                            tauri::async_runtime::spawn(orchestration::run_sync_task(
                                app_clone,
                                pool_clone,
                                sync_state_clone,
                            ));
                        } else {
                            log::warn!("Cannot run tray sync: pool or sync_state not initialized");
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
                            let batch_size = match app.path().app_config_dir() {
                                Ok(dir) => match config::load(&dir) {
                                    Ok(c) => orchestration::clamp_batch_size(c.parse.batch_size, 100),
                                    Err(e) => {
                                        log::warn!(
                                            "Failed to load config from {:?}: {}. Falling back to default batch_size=100",
                                            dir, e
                                        );
                                        100
                                    }
                                },
                                Err(e) => {
                                    log::warn!(
                                        "Failed to get app_config_dir: {}. Falling back to default batch_size=100",
                                        e
                                    );
                                    100
                                }
                            };
                            tauri::async_runtime::spawn(orchestration::run_batch_parse_task(
                                app_clone,
                                pool_clone,
                                parse_state_clone,
                                batch_size,
                            ));
                        } else {
                            log::warn!("Cannot run tray parse: pool or parse_state not initialized");
                        }
                    }
                    "tray_product_name_parse" => {
                        if let (Some(pool), Some(parse_state)) = (
                            app.try_state::<SqlitePool>(),
                            app.try_state::<commands::ProductNameParseState>(),
                        ) {
                            let app_clone = app.clone();
                            let pool_clone = pool.inner().clone();
                            let parse_state_clone = parse_state.inner().clone();
                            tauri::async_runtime::spawn(
                                orchestration::run_product_name_parse_task(
                                    app_clone,
                                    pool_clone,
                                    parse_state_clone,
                                    false, // トレイ経由では try_start を本関数内で行う
                                ),
                            );
                        } else {
                            log::warn!(
                                "Cannot run tray product name parse: pool or parse_state not initialized"
                            );
                        }
                    }
                    "tray_delivery_check" => {
                        if let (Some(pool), Some(check_state)) = (
                            app.try_state::<SqlitePool>(),
                            app.try_state::<commands::DeliveryCheckState>(),
                        ) {
                            let app_clone = app.clone();
                            let pool_clone = pool.inner().clone();
                            let check_state_clone = check_state.inner().clone();
                            if let Err(e) = check_state_clone.try_start() {
                                log::warn!("Cannot start delivery check from tray: {e}");
                            } else {
                                tauri::async_runtime::spawn(orchestration::run_delivery_check_task(
                                    app_clone,
                                    pool_clone,
                                    check_state_clone,
                                ));
                            }
                        } else {
                            log::warn!(
                                "Cannot run tray delivery check: pool or check_state not initialized"
                            );
                        }
                    }
                    "quit" => {
                        // クリップボード監視をグレースフルに停止
                        if let Some(shutdown_signal) = app.try_state::<Arc<AtomicBool>>() {
                            let shutdown_signal = shutdown_signal.inner().clone();
                            // シャットダウン要求を通知
                            shutdown_signal.store(true, Ordering::Relaxed);

                            // 監視スレッドの終了完了を明示的に待つ仕組みは現状ないため、
                            // シャットダウン要求を送ったら即座にアプリケーションを終了する。
                            app.exit(0);
                        } else {
                            // 監視スレッドがいない場合は即座に終了
                            app.exit(0);
                        }
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
            commands::seed_e2e_db,
            commands::get_db_filename,
            commands::fetch_gmail_emails,
            commands::start_sync,
            commands::cancel_sync,
            commands::get_sync_status,
            commands::update_batch_size,
            commands::update_max_iterations,
            commands::update_max_results_per_page,
            commands::update_timeout_minutes,
            commands::reset_sync_status,
            commands::reset_sync_date,
            commands::save_window_settings,
            commands::get_email_stats,
            commands::get_order_stats,
            commands::get_delivery_stats,
            commands::get_product_master_stats,
            commands::get_misc_stats,
            commands::get_logs,
            commands::get_all_shop_settings,
            commands::create_shop_setting,
            commands::update_shop_setting,
            commands::delete_shop_setting,
            commands::toggle_shop_enabled,
            commands::init_default_shop_settings,
            commands::parse_email,
            commands::parse_and_save_email,
            commands::start_batch_parse,
            commands::cancel_parse,
            commands::get_parse_status,
            commands::update_parse_batch_size,
            commands::get_gemini_config,
            commands::update_gemini_batch_size,
            commands::update_gemini_delay_seconds,
            commands::has_gemini_api_key,
            commands::save_gemini_api_key,
            commands::delete_gemini_api_key,
            commands::start_product_name_parse,
            commands::has_gmail_oauth_credentials,
            commands::save_gmail_oauth_credentials,
            commands::delete_gmail_oauth_credentials,
            commands::is_google_search_configured,
            commands::save_google_search_api_key,
            commands::delete_google_search_config,
            commands::search_product_images,
            commands::save_image_from_url,
            commands::export_metadata,
            commands::import_metadata,
            commands::restore_metadata,
            commands::save_item_override,
            commands::save_order_override,
            commands::delete_item_override,
            commands::delete_item_override_by_key,
            commands::delete_order_override,
            commands::delete_order_override_by_key,
            commands::exclude_item,
            commands::exclude_order,
            commands::restore_excluded_item,
            commands::restore_excluded_order,
            commands::get_all_excluded_items,
            commands::get_all_excluded_orders,
            commands::get_product_master_list,
            commands::update_product_master,
            commands::start_delivery_check,
            commands::cancel_delivery_check,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== is_sqlite_version_supported Tests ====================

    #[test]
    fn test_is_sqlite_version_supported_true_cases() {
        assert!(is_sqlite_version_supported("3.43.0"));
        assert!(is_sqlite_version_supported("3.43.1"));
        assert!(is_sqlite_version_supported("3.50.0"));
        assert!(is_sqlite_version_supported("4.0.0"));
        // patch無しでも minor 判定ができれば OK
        assert!(is_sqlite_version_supported("3.43"));
    }

    #[test]
    fn test_is_sqlite_version_supported_false_cases() {
        assert!(!is_sqlite_version_supported("3.42.9"));
        assert!(!is_sqlite_version_supported("3.0.0"));
        assert!(!is_sqlite_version_supported("2.999.0"));
        assert!(!is_sqlite_version_supported("3")); // minor が無い
        assert!(!is_sqlite_version_supported("")); // 空
        assert!(!is_sqlite_version_supported("abc"));
        assert!(!is_sqlite_version_supported("3.x.0"));
        assert!(!is_sqlite_version_supported("x.43.0"));
    }

    // ==================== image_utils Tests ====================

    #[test]
    fn test_validate_image_url_https_ok() {
        assert!(image_utils::validate_image_url("https://example.com/image.png").is_ok());
        assert!(image_utils::validate_image_url("https://images.example.co.jp/photo.jpg").is_ok());
    }

    #[test]
    fn test_validate_image_url_http_rejected() {
        assert!(image_utils::validate_image_url("http://example.com/image.png").is_err());
    }

    #[test]
    fn test_validate_image_url_localhost_rejected() {
        assert!(image_utils::validate_image_url("https://localhost/image.png").is_err());
        assert!(image_utils::validate_image_url("https://127.0.0.1/image.png").is_err());
    }

    #[test]
    fn test_validate_image_url_private_ip_rejected() {
        assert!(image_utils::validate_image_url("https://192.168.1.1/image.png").is_err());
        assert!(image_utils::validate_image_url("https://10.0.0.1/image.png").is_err());
    }

    #[test]
    fn test_validate_image_url_metadata_rejected() {
        assert!(image_utils::validate_image_url("https://169.254.169.254/").is_err());
    }

    #[test]
    fn test_is_private_ip() {
        use std::net::IpAddr;
        assert!(image_utils::is_private_ip("10.0.0.1".parse().unwrap()));
        assert!(image_utils::is_private_ip("192.168.1.1".parse().unwrap()));
        assert!(image_utils::is_private_ip("172.16.0.1".parse().unwrap()));
        assert!(!image_utils::is_private_ip(
            "8.8.8.8".parse::<IpAddr>().unwrap()
        ));
    }
}
