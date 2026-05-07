use serde::Serialize;

#[derive(Serialize)]
pub struct TableDefinition {
    pub name: &'static str,
    pub label: &'static str,
}

#[tauri::command]
pub fn get_table_definitions() -> Vec<TableDefinition> {
    vec![
        TableDefinition {
            name: "emails",
            label: "メール",
        },
        TableDefinition {
            name: "orders",
            label: "注文",
        },
        TableDefinition {
            name: "items",
            label: "商品アイテム",
        },
        TableDefinition {
            name: "images",
            label: "画像",
        },
        TableDefinition {
            name: "deliveries",
            label: "配送情報",
        },
        TableDefinition {
            name: "htmls",
            label: "HTML本文",
        },
        TableDefinition {
            name: "order_emails",
            label: "注文-メール",
        },
        TableDefinition {
            name: "order_htmls",
            label: "注文-HTML",
        },
        TableDefinition {
            name: "shop_settings",
            label: "店舗設定",
        },
        TableDefinition {
            name: "product_master",
            label: "商品マスタ",
        },
        TableDefinition {
            name: "item_overrides",
            label: "アイテム上書き",
        },
        TableDefinition {
            name: "order_overrides",
            label: "注文上書き",
        },
        TableDefinition {
            name: "excluded_items",
            label: "除外アイテム",
        },
        TableDefinition {
            name: "excluded_orders",
            label: "除外注文",
        },
        TableDefinition {
            name: "tracking_check_logs",
            label: "配送確認ログ",
        },
        TableDefinition {
            name: "news_clips",
            label: "ニュースクリップ",
        },
        TableDefinition {
            name: "item_exclusion_patterns",
            label: "除外キーワードパターン",
        },
    ]
}
