use serde::Serialize;

#[derive(Serialize)]
pub struct TableDefinition {
    pub name: &'static str,
    pub label: &'static str,
}

static COLUMN_LABELS: &[(&str, &[(&str, &str)])] = &[
    (
        "emails",
        &[
            ("id", "ID"),
            ("message_id", "メッセージID"),
            ("body_plain", "本文（プレーン）"),
            ("body_html", "本文（HTML）"),
            ("analysis_status", "解析ステータス"),
            ("internal_date", "内部日時"),
            ("from_address", "送信元アドレス"),
            ("subject", "件名"),
            ("created_at", "作成日時"),
            ("updated_at", "更新日時"),
        ],
    ),
    (
        "orders",
        &[
            ("id", "ID"),
            ("shop_domain", "ショップドメイン"),
            ("shop_name", "ショップ名"),
            ("order_number", "注文番号"),
            ("order_date", "注文日"),
            ("created_at", "作成日時"),
            ("updated_at", "更新日時"),
        ],
    ),
    (
        "items",
        &[
            ("id", "ID"),
            ("order_id", "注文ID"),
            ("item_name", "商品名"),
            ("item_name_normalized", "正規化商品名"),
            ("price", "価格"),
            ("quantity", "数量"),
            ("category", "カテゴリ"),
            ("brand", "ブランド"),
            ("created_at", "作成日時"),
            ("updated_at", "更新日時"),
        ],
    ),
    (
        "images",
        &[
            ("id", "ID"),
            ("item_name_normalized", "正規化商品名"),
            ("file_name", "ファイル名"),
            ("created_at", "作成日時"),
        ],
    ),
    (
        "deliveries",
        &[
            ("id", "ID"),
            ("order_id", "注文ID"),
            ("tracking_number", "追跡番号"),
            ("carrier", "配送業者"),
            ("delivery_status", "配送ステータス"),
            ("estimated_delivery", "配送予定日"),
            ("actual_delivery", "実際の配送日"),
            ("last_checked_at", "最終確認日時"),
            ("created_at", "作成日時"),
            ("updated_at", "更新日時"),
        ],
    ),
    (
        "htmls",
        &[
            ("id", "ID"),
            ("url", "URL"),
            ("html_content", "HTML内容"),
            ("analysis_status", "解析ステータス"),
            ("created_at", "作成日時"),
            ("updated_at", "更新日時"),
        ],
    ),
    (
        "order_emails",
        &[
            ("id", "ID"),
            ("order_id", "注文ID"),
            ("email_id", "メールID"),
            ("created_at", "作成日時"),
        ],
    ),
    (
        "order_htmls",
        &[
            ("id", "ID"),
            ("order_id", "注文ID"),
            ("html_id", "HTML ID"),
            ("created_at", "作成日時"),
        ],
    ),
    (
        "shop_settings",
        &[
            ("id", "ID"),
            ("shop_name", "ショップ名"),
            ("sender_address", "送信元アドレス"),
            ("parser_type", "パーサー種別"),
            ("is_enabled", "有効フラグ"),
            ("subject_filters", "件名フィルター"),
            ("created_at", "作成日時"),
            ("updated_at", "更新日時"),
        ],
    ),
    (
        "product_master",
        &[
            ("id", "ID"),
            ("raw_name", "元の名称"),
            ("normalized_name", "正規化名称"),
            ("maker", "メーカー"),
            ("series", "シリーズ"),
            ("product_name", "商品名"),
            ("scale", "スケール"),
            ("is_reissue", "再版フラグ"),
            ("platform_hint", "プラットフォームヒント"),
            ("created_at", "作成日時"),
            ("updated_at", "更新日時"),
        ],
    ),
    (
        "item_overrides",
        &[
            ("id", "ID"),
            ("shop_domain", "ショップドメイン"),
            ("order_number", "注文番号"),
            ("original_item_name", "元の商品名"),
            ("original_brand", "元のブランド"),
            ("item_name", "商品名"),
            ("price", "価格"),
            ("quantity", "数量"),
            ("brand", "ブランド"),
            ("category", "カテゴリ"),
            ("created_at", "作成日時"),
            ("updated_at", "更新日時"),
        ],
    ),
    (
        "order_overrides",
        &[
            ("id", "ID"),
            ("shop_domain", "ショップドメイン"),
            ("order_number", "注文番号"),
            ("new_order_number", "新注文番号"),
            ("order_date", "注文日"),
            ("shop_name", "ショップ名"),
            ("created_at", "作成日時"),
            ("updated_at", "更新日時"),
        ],
    ),
    (
        "excluded_items",
        &[
            ("id", "ID"),
            ("shop_domain", "ショップドメイン"),
            ("order_number", "注文番号"),
            ("item_name", "商品名"),
            ("brand", "ブランド"),
            ("reason", "除外理由"),
            ("created_at", "作成日時"),
        ],
    ),
    (
        "excluded_orders",
        &[
            ("id", "ID"),
            ("shop_domain", "ショップドメイン"),
            ("order_number", "注文番号"),
            ("reason", "除外理由"),
            ("created_at", "作成日時"),
        ],
    ),
    (
        "tracking_check_logs",
        &[
            ("id", "ID"),
            ("tracking_number", "追跡番号"),
            ("checked_at", "確認日時"),
            ("check_status", "チェック結果"),
            ("delivery_status", "配送ステータス"),
            ("description", "説明"),
            ("location", "場所"),
            ("error_message", "エラーメッセージ"),
            ("created_at", "作成日時"),
        ],
    ),
    (
        "news_clips",
        &[
            ("id", "ID"),
            ("title", "タイトル"),
            ("url", "URL"),
            ("source_name", "情報源"),
            ("published_at", "公開日時"),
            ("summary", "要約"),
            ("tags", "タグ"),
            ("clipped_at", "クリップ日時"),
        ],
    ),
    (
        "item_exclusion_patterns",
        &[
            ("id", "ID"),
            ("shop_domain", "ショップドメイン"),
            ("keyword", "キーワード"),
            ("match_type", "マッチ種別"),
            ("note", "メモ"),
            ("created_at", "作成日時"),
        ],
    ),
];

pub fn get_column_label(table: &str, column: &str) -> String {
    COLUMN_LABELS
        .iter()
        .find(|(t, _)| *t == table)
        .and_then(|(_, cols)| cols.iter().find(|(c, _)| *c == column))
        .map(|(_, label)| label.to_string())
        .unwrap_or_else(|| column.to_string())
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
