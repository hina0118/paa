use sqlx::sqlite::SqlitePool;
use tauri::Manager;

use crate::config;
use crate::logic::email_parser::get_candidate_parsers;
use crate::orchestration;
use crate::parsers;
use crate::repository::{
    OrderRepository, ShopSettingsRepository, SqliteOrderRepository, SqliteShopSettingsRepository,
};

#[tauri::command]
pub fn parse_email(parser_type: String, email_body: String) -> Result<parsers::OrderInfo, String> {
    let parser = parsers::get_parser(&parser_type)
        .ok_or_else(|| format!("Unknown parser type: {}", parser_type))?;

    parser.parse(&email_body)
}

#[tauri::command]
pub async fn parse_and_save_email(
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

/// メールパース処理を開始
/// BatchRunner<EmailParseTask> を使用
#[tauri::command]
pub async fn start_batch_parse(
    app_handle: tauri::AppHandle,
    pool: tauri::State<'_, SqlitePool>,
    parse_state: tauri::State<'_, parsers::ParseState>,
    batch_size: Option<usize>,
) -> Result<(), String> {
    let size = if let Some(s) = batch_size {
        s.max(1)
    } else {
        let app_config_dir = app_handle
            .path()
            .app_config_dir()
            .map_err(|e| format!("Failed to get app config dir: {e}"))?;
        let config = config::load(&app_config_dir)?;
        orchestration::clamp_batch_size(config.parse.batch_size, 100)
    };

    let pool_clone = pool.inner().clone();
    let parse_state_clone = parse_state.inner().clone();
    tauri::async_runtime::spawn(orchestration::run_batch_parse_task(
        app_handle,
        pool_clone,
        parse_state_clone,
        size,
    ));
    Ok(())
}

#[tauri::command]
pub async fn cancel_parse(
    parse_state: tauri::State<'_, parsers::ParseState>,
) -> Result<(), String> {
    log::info!("Cancelling parse...");
    parse_state.request_cancel();
    Ok(())
}

#[tauri::command]
pub async fn get_parse_status(
    app_handle: tauri::AppHandle,
    parse_state: tauri::State<'_, parsers::ParseState>,
) -> Result<parsers::ParseMetadata, String> {
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let config = config::load(&app_config_dir)?;

    let parse_status = if parse_state
        .inner()
        .is_running
        .lock()
        .map(|g| *g)
        .unwrap_or(false)
    {
        "running"
    } else if parse_state
        .inner()
        .last_error
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false)
    {
        "error"
    } else {
        "idle"
    };

    let last_error_message = parse_state
        .inner()
        .last_error
        .lock()
        .ok()
        .and_then(|g| g.clone());

    Ok(parsers::ParseMetadata {
        parse_status: parse_status.to_string(),
        last_parse_started_at: None,
        last_parse_completed_at: None,
        total_parsed_count: 0,
        last_error_message,
        batch_size: config.parse.batch_size,
    })
}

#[tauri::command]
pub async fn update_parse_batch_size(
    app_handle: tauri::AppHandle,
    batch_size: i64,
) -> Result<(), String> {
    log::info!("Updating parse batch size to: {batch_size}");
    let app_config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get app config dir: {e}"))?;
    let mut config = config::load(&app_config_dir)?;
    config.parse.batch_size = batch_size;
    config::save(&app_config_dir, &config)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let result = parse_email(
            "hobbysearch_confirm".to_string(),
            "invalid body".to_string(),
        );
        assert!(result.is_err());
    }

    const SAMPLE_HOBBYSEARCH_CHANGE: &str = r#"
[注文番号] 25-0202-5678

[お届け先情報]
〒100-0001
東京都千代田区千代田1-1-1
テスト 花子 様

[ご購入内容]
バンダイ 1234567 テスト商品A (プラモデル) HGシリーズ
単価：1,000円 × 個数：1 = 1,000円

小計：1,000円
送料：660円
合計：1,660円
"#;

    #[test]
    fn test_parse_email_hobbysearch_change() {
        let result = parse_email(
            "hobbysearch_change".to_string(),
            SAMPLE_HOBBYSEARCH_CHANGE.to_string(),
        );
        assert!(result.is_ok());
        let order_info = result.unwrap();
        assert_eq!(order_info.order_number, "25-0202-5678");
        assert_eq!(order_info.items.len(), 1);
        assert_eq!(order_info.items[0].unit_price, 1000);
        assert_eq!(order_info.items[0].quantity, 1);
    }

    const SAMPLE_HOBBYSEARCH_CHANGE_YOYAKU: &str = r#"
[注文番号] 25-0303-9999

[お届け先情報]
〒200-0002
東京都中央区銀座1-2-3
予約 太郎 様

[ご予約内容]
バンダイ 2345678 テスト商品B (プラモデル) MGシリーズ
単価：3,000円 × 個数：2 = 6,000円

予約商品合計：6,000円
"#;

    #[test]
    fn test_parse_email_hobbysearch_change_yoyaku() {
        let result = parse_email(
            "hobbysearch_change_yoyaku".to_string(),
            SAMPLE_HOBBYSEARCH_CHANGE_YOYAKU.to_string(),
        );
        assert!(result.is_ok());
        let order_info = result.unwrap();
        assert_eq!(order_info.order_number, "25-0303-9999");
        assert_eq!(order_info.items.len(), 1);
        assert_eq!(order_info.items[0].unit_price, 3000);
        assert_eq!(order_info.items[0].quantity, 2);
    }

    const SAMPLE_HOBBYSEARCH_CONFIRM_YOYAKU: &str = r#"
[注文番号] 25-0505-2222

[お届け先情報]
〒300-0003
東京都港区六本木1-2-3
予約 次郎 様

[ご予約内容]
バンダイ 3456789 テスト商品D (プラモデル) RGシリーズ
単価：2,500円 × 個数：2 = 5,000円

予約商品合計 5,000円
"#;

    #[test]
    fn test_parse_email_hobbysearch_confirm_yoyaku() {
        let result = parse_email(
            "hobbysearch_confirm_yoyaku".to_string(),
            SAMPLE_HOBBYSEARCH_CONFIRM_YOYAKU.to_string(),
        );
        assert!(result.is_ok());
        let order_info = result.unwrap();
        assert_eq!(order_info.order_number, "25-0505-2222");
        assert_eq!(order_info.items.len(), 1);
        assert_eq!(order_info.items[0].unit_price, 2500);
        assert_eq!(order_info.items[0].quantity, 2);
        assert_eq!(order_info.subtotal, Some(5000));
    }

    const SAMPLE_HOBBYSEARCH_SEND: &str = r#"
[代表注文番号] 25-0404-1111

[運送会社] ヤマト運輸
[配送伝票] 1234-5678-9012

[お届け先情報]
〒300-0003
東京都港区六本木1-2-3
発送 次郎 様

[ご購入内容]
バンダイ 3456789 テスト商品C (プラモデル) RGシリーズ
単価：2,000円 × 個数：1 = 2,000円

小計：2,000円
送料：0円
合計：2,000円
"#;

    #[test]
    fn test_parse_email_hobbysearch_send() {
        let result = parse_email(
            "hobbysearch_send".to_string(),
            SAMPLE_HOBBYSEARCH_SEND.to_string(),
        );
        assert!(result.is_ok());
        let order_info = result.unwrap();
        assert_eq!(order_info.order_number, "25-0404-1111");
        assert_eq!(order_info.items.len(), 1);
        assert!(order_info.delivery_info.is_some());
        let info = order_info.delivery_info.as_ref().unwrap();
        assert_eq!(info.carrier, "ヤマト運輸");
        assert_eq!(info.tracking_number, "1234-5678-9012");
    }
}
