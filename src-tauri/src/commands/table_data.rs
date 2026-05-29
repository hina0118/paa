use serde::{Deserialize, Serialize};
use sqlx::{Row, ValueRef};
use tauri::State;

use super::table_definitions::get_column_label;

#[derive(Deserialize)]
pub struct ColumnFilter {
    pub column: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct TableQueryParams {
    pub table_name: String,
    pub page: u32,
    pub page_size: u32,
    pub filters: Vec<ColumnFilter>,
    pub sort_column: Option<String>,
    pub sort_direction: Option<String>,
}

#[derive(Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub label: String,
    pub data_type: String,
    pub is_pk: bool,
}

#[derive(Serialize)]
pub struct TableDataResponse {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<serde_json::Map<String, serde_json::Value>>,
    pub total_count: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

fn valid_table_names() -> &'static [&'static str] {
    &[
        "emails",
        "orders",
        "items",
        "images",
        "deliveries",
        "htmls",
        "order_emails",
        "order_htmls",
        "shop_settings",
        "product_master",
        "item_overrides",
        "order_overrides",
        "excluded_items",
        "excluded_orders",
        "tracking_check_logs",
        "news_clips",
        "item_exclusion_patterns",
    ]
}

fn is_valid_table_name(name: &str) -> bool {
    valid_table_names().contains(&name)
}

fn escape_like_value(value: &str) -> String {
    value
        .replace('!', "!!")
        .replace('%', "!%")
        .replace('_', "!_")
}

#[tauri::command]
pub async fn query_table_data(
    params: TableQueryParams,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<TableDataResponse, String> {
    if !is_valid_table_name(&params.table_name) {
        return Err(format!("Invalid table name: {}", params.table_name));
    }

    let page_size = params.page_size.clamp(1, 500);
    let table = &params.table_name;

    // Get schema via PRAGMA (table name validated above)
    let pragma_sql = format!("PRAGMA table_info({table})");
    let schema_rows = sqlx::query(&pragma_sql)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| format!("Failed to get schema: {e}"))?;

    if schema_rows.is_empty() {
        return Err(format!("Table \"{table}\" does not exist or has no columns"));
    }

    let columns: Vec<ColumnInfo> = schema_rows
        .iter()
        .map(|row| {
            let name: String = row.get("name");
            let data_type: String = row.get("type");
            let pk: i64 = row.get("pk");
            let label = get_column_label(table, &name);
            ColumnInfo {
                label,
                name,
                data_type,
                is_pk: pk > 0,
            }
        })
        .collect();

    let column_names: Vec<&str> = columns.iter().map(|c| c.name.as_str()).collect();

    // Build WHERE clause; only allow columns present in schema
    let mut where_parts: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    for filter in &params.filters {
        if filter.value.trim().is_empty() {
            continue;
        }
        if !column_names.contains(&filter.column.as_str()) {
            continue;
        }
        let quoted = format!("\"{}\"", filter.column.replace('"', "\"\""));
        where_parts.push(format!("{quoted} LIKE ? ESCAPE '!'"));
        bind_values.push(format!("%{}%", escape_like_value(filter.value.trim())));
    }

    let where_clause = if where_parts.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_parts.join(" AND "))
    };

    // COUNT
    let count_sql = format!("SELECT COUNT(*) as cnt FROM {table} {where_clause}");
    let mut count_query = sqlx::query(&count_sql);
    for v in &bind_values {
        count_query = count_query.bind(v);
    }
    let count_row = count_query
        .fetch_one(pool.inner())
        .await
        .map_err(|e| format!("Failed to count rows: {e}"))?;
    let total_count: i64 = count_row.get("cnt");
    let total_count = total_count as u64;

    // ORDER BY
    let order_clause = if let Some(col) = &params.sort_column {
        if column_names.contains(&col.as_str()) {
            let dir = match params.sort_direction.as_deref() {
                Some("desc") => "DESC",
                _ => "ASC",
            };
            let quoted = format!("\"{}\"", col.replace('"', "\"\""));
            format!("ORDER BY {quoted} {dir}")
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // SELECT with pagination
    let offset = params.page * page_size;
    let data_sql =
        format!("SELECT * FROM {table} {where_clause} {order_clause} LIMIT ? OFFSET ?");
    let mut data_query = sqlx::query(&data_sql);
    for v in &bind_values {
        data_query = data_query.bind(v);
    }
    data_query = data_query.bind(page_size as i64).bind(offset as i64);

    let data_rows = data_query
        .fetch_all(pool.inner())
        .await
        .map_err(|e| format!("Failed to fetch rows: {e}"))?;

    let rows: Vec<serde_json::Map<String, serde_json::Value>> = data_rows
        .iter()
        .map(|row| {
            let mut map = serde_json::Map::new();
            for col in &columns {
                let value: serde_json::Value = match row.try_get_raw(col.name.as_str()) {
                    Ok(raw) => {
                        if raw.is_null() {
                            serde_json::Value::Null
                        } else if let Ok(v) = row.try_get::<i64, _>(col.name.as_str()) {
                            serde_json::Value::Number(v.into())
                        } else if let Ok(v) = row.try_get::<f64, _>(col.name.as_str()) {
                            serde_json::Number::from_f64(v)
                                .map(serde_json::Value::Number)
                                .unwrap_or(serde_json::Value::Null)
                        } else if let Ok(v) = row.try_get::<String, _>(col.name.as_str()) {
                            serde_json::Value::String(v)
                        } else {
                            serde_json::Value::Null
                        }
                    }
                    Err(_) => serde_json::Value::Null,
                };
                map.insert(col.name.clone(), value);
            }
            map
        })
        .collect();

    let page_size_u64 = u64::from(page_size);
    // div_ceil is stable since 1.73.0; project MSRV is 1.70.0 so we use manual arithmetic
    #[allow(clippy::manual_checked_ops)]
    let total_pages = if page_size_u64 == 0 {
        0
    } else {
        (total_count + page_size_u64 - 1) / page_size_u64
    };

    Ok(TableDataResponse {
        columns,
        rows,
        total_count,
        page: params.page,
        page_size,
        total_pages: total_pages as u32,
    })
}
