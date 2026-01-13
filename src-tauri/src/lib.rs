use tauri::Manager;
use tauri_plugin_sql::{Migration, MigrationKind};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
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
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");

            let db_path = app_data_dir.join("paa_data.db");
            let db_url = format!("sqlite:{}", db_path.to_string_lossy());

            app.handle().plugin(
                tauri_plugin_sql::Builder::default()
                    .add_migrations(&db_url, migrations)
                    .build()
            )?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
