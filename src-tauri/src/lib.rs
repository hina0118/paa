use tauri::Manager;
use tauri_plugin_sql::{Migration, MigrationKind};
use serde_json::{json, Value};
use rusqlite::{Connection, params};
use std::collections::HashMap;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_table_data(
    app: tauri::AppHandle,
    table_name: String,
    limit: u32,
    offset: u32,
) -> Result<Vec<Value>, String> {
    // Validate table name to prevent SQL injection
    let valid_tables = [
        "emails",
        "orders",
        "items",
        "images",
        "deliveries",
        "htmls",
        "order_emails",
        "order_htmls",
    ];
    if !valid_tables.contains(&table_name.as_str()) {
        return Err(format!("Invalid table name: {}", table_name));
    }

    // Get database path
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_data_dir.join("paa_data.db");

    // Connect to database
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    // Execute query
    let query = format!("SELECT * FROM {} LIMIT ? OFFSET ?", table_name);
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;

    // Get column names
    let column_names: Vec<String> = stmt
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();

    // Execute query and convert to JSON
    let rows = stmt
        .query_map(params![limit, offset], |row| {
            let mut map: HashMap<String, Value> = HashMap::new();
            for (idx, col_name) in column_names.iter().enumerate() {
                let value: Result<Value, _> = row.get(idx).and_then(|v: rusqlite::types::Value| {
                    Ok(match v {
                        rusqlite::types::Value::Null => Value::Null,
                        rusqlite::types::Value::Integer(i) => json!(i),
                        rusqlite::types::Value::Real(f) => json!(f),
                        rusqlite::types::Value::Text(s) => json!(s),
                        rusqlite::types::Value::Blob(b) => json!(b),
                    })
                });
                map.insert(col_name.clone(), value.unwrap_or(Value::Null));
            }
            Ok(json!(map))
        })
        .map_err(|e| e.to_string())?;

    let result: Result<Vec<Value>, _> = rows.collect();
    result.map_err(|e| e.to_string())
}

#[tauri::command]
fn get_table_schema(
    app: tauri::AppHandle,
    table_name: String,
) -> Result<Vec<Value>, String> {
    // Validate table name
    let valid_tables = [
        "emails",
        "orders",
        "items",
        "images",
        "deliveries",
        "htmls",
        "order_emails",
        "order_htmls",
    ];
    if !valid_tables.contains(&table_name.as_str()) {
        return Err(format!("Invalid table name: {}", table_name));
    }

    // Get database path
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_data_dir.join("paa_data.db");

    // Connect to database
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    // Get table schema
    let query = format!("PRAGMA table_info({})", table_name);
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(json!({
                "cid": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "type": row.get::<_, String>(2)?,
                "notnull": row.get::<_, i64>(3)?,
                "dflt_value": row.get::<_, Option<String>>(4)?,
                "pk": row.get::<_, i64>(5)?
            }))
        })
        .map_err(|e| e.to_string())?;

    let result: Result<Vec<Value>, _> = rows.collect();
    result.map_err(|e| e.to_string())
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
        .invoke_handler(tauri::generate_handler![greet, get_table_data, get_table_schema])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
