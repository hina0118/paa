use tauri::Manager;
use tauri_plugin_sql::{Migration, MigrationKind};
use sqlx::sqlite::SqlitePool;

mod gmail;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn initialize_database(pool: tauri::State<'_, SqlitePool>) -> Result<(), String> {
    log::info!("Initializing database...");

    // 簡単なクエリを実行してDB接続とマイグレーションをトリガー
    sqlx::query("SELECT 1")
        .execute(pool.inner())
        .await
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    log::info!("Database initialized successfully");
    Ok(())
}

#[tauri::command]
async fn fetch_gmail_emails(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>
) -> Result<gmail::FetchResult, String> {
    log::info!("Starting Gmail email fetch...");
    log::info!("If a browser window doesn't open automatically, please check the console for the authentication URL.");

    let client = gmail::GmailClient::new(&app_handle).await?;

    // 過去30日間のメールのみを取得
    let today = chrono::Local::now();
    let thirty_days_ago = today - chrono::Duration::days(30);
    let after_date = thirty_days_ago.format("%Y/%m/%d");

    let query = format!(
        r#"subject:(注文 OR 予約 OR ありがとうございます) after:{}"#,
        after_date
    );

    log::info!("Search query: {}", query);

    let messages = client.fetch_messages(&query).await?;
    log::info!("Fetched {} messages from Gmail", messages.len());

    // バックエンドでDBに保存
    let result = gmail::save_messages_to_db(&pool, messages).await?;

    Ok(result)
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

            app.manage(pool);
            log::info!("sqlx pool created for backend use");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, initialize_database, fetch_gmail_emails])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
