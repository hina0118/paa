//! Amazon.co.jp プラグイン
//!
//! Amazon.co.jp からの注文確認メールをパースして保存する。
//!
//! # 対応フォーマット
//! - 新フォーマット（件名: `注文済み:`）
//! - 旧フォーマット・単一注文（件名: `Amazon.co.jp ご注文の確認`）
//! - 旧フォーマット・複数注文（件名: `Amazon.co.jpでのご注文`）

pub mod html_parser;
pub mod parsers;

use async_trait::async_trait;

use super::{
    apply_internal_date, derive_shop_domain, DefaultShopSetting, DispatchError, DispatchOutcome,
    PluginRegistration, VendorPlugin,
};
use crate::parsers::EmailParser;
use crate::repository::SqliteOrderRepository;

pub struct AmazonPlugin;

#[async_trait]
impl VendorPlugin for AmazonPlugin {
    fn parser_types(&self) -> &[&str] {
        &["amazon_confirm"]
    }

    fn priority(&self) -> i32 {
        10
    }

    fn get_parser(&self, _parser_type: &str) -> Option<Box<dyn EmailParser>> {
        // dispatch() 内で直接パーサーを呼ぶため None を返す
        None
    }

    fn prefer_plain_text(&self) -> bool {
        // Amazon のメールは plain text フォーマットでパースする。
        // body_html が存在する場合も body_plain を優先して使用する。
        true
    }

    fn shop_name(&self) -> &str {
        "Amazon.co.jp"
    }

    fn default_shop_settings(&self) -> Vec<DefaultShopSetting> {
        vec![DefaultShopSetting {
            shop_name: "Amazon.co.jp".to_string(),
            sender_address: "auto-confirm@amazon.co.jp".to_string(),
            parser_type: "amazon_confirm".to_string(),
            subject_filters: Some(vec![
                "Amazon.co.jp ご注文の確認".to_string(),
                "Amazon.co.jpでのご注文".to_string(),
                "注文済み:".to_string(),
            ]),
        }]
    }

    async fn dispatch(
        &self,
        parser_type: &str,
        email_id: i64,
        from_address: Option<&str>,
        shop_name: &str,
        internal_date: Option<i64>,
        body: &str,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    ) -> Result<DispatchOutcome, DispatchError> {
        if parser_type != "amazon_confirm" {
            return Err(DispatchError::ParseFailed(format!(
                "amazon: 未対応の parser_type '{parser_type}'"
            )));
        }

        let parser = parsers::confirm::AmazonConfirmParser;
        let shop_domain = derive_shop_domain(from_address);

        // 複数注文フォーマット（parse_multi が Some を返す場合）
        if let Some(result) = parser.parse_multi(body) {
            let mut orders = result.map_err(DispatchError::ParseFailed)?;

            if orders.is_empty() {
                return Err(DispatchError::ParseFailed(
                    "parse_multi が空の注文リストを返しました".to_string(),
                ));
            }

            // 注文日が未設定の場合は内部日付で補完
            for order in &mut orders {
                apply_internal_date(order, internal_date);
            }

            let mut saved_orders = Vec::with_capacity(orders.len());
            for order_info in orders {
                let order_id = SqliteOrderRepository::save_order_in_tx(
                    tx,
                    &order_info,
                    Some(email_id),
                    shop_domain.clone(),
                    Some(shop_name.to_string()),
                )
                .await
                .map_err(DispatchError::SaveFailed)?;

                // 注文詳細ページ URL を htmls / order_htmls に登録する
                let detail_url = order_detail_url(&order_info.order_number);
                SqliteOrderRepository::insert_html_url_for_order_in_tx(tx, order_id, &detail_url)
                    .await
                    .map_err(DispatchError::SaveFailed)?;

                saved_orders.push(order_info);
            }

            return Ok(DispatchOutcome::MultiOrderSaved(saved_orders));
        }

        // 単一注文
        let mut order_info = parser.parse(body).map_err(DispatchError::ParseFailed)?;
        apply_internal_date(&mut order_info, internal_date);

        let order_id = SqliteOrderRepository::save_order_in_tx(
            tx,
            &order_info,
            Some(email_id),
            shop_domain,
            Some(shop_name.to_string()),
        )
        .await
        .map_err(DispatchError::SaveFailed)?;

        // 注文詳細ページ URL を htmls / order_htmls に登録する
        let detail_url = order_detail_url(&order_info.order_number);
        SqliteOrderRepository::insert_html_url_for_order_in_tx(tx, order_id, &detail_url)
            .await
            .map_err(DispatchError::SaveFailed)?;

        Ok(DispatchOutcome::OrderSaved(Box::new(order_info)))
    }
}

/// Amazon 注文詳細ページの URL を組み立てる
fn order_detail_url(order_number: &str) -> String {
    format!(
        "https://www.amazon.co.jp/your-orders/order-details?orderID={}",
        order_number
    )
}

inventory::submit!(PluginRegistration {
    factory: || Box::new(AmazonPlugin),
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amazon_plugin_parser_types() {
        let plugin = AmazonPlugin;
        assert!(plugin.parser_types().contains(&"amazon_confirm"));
    }

    #[test]
    fn test_amazon_plugin_shop_name() {
        assert_eq!(AmazonPlugin.shop_name(), "Amazon.co.jp");
    }

    #[test]
    fn test_amazon_default_shop_settings() {
        let settings = AmazonPlugin.default_shop_settings();
        assert_eq!(settings.len(), 1);

        let s = &settings[0];
        assert_eq!(s.sender_address, "auto-confirm@amazon.co.jp");
        assert_eq!(s.parser_type, "amazon_confirm");

        let filters = s.subject_filters.as_ref().unwrap();
        assert!(filters.contains(&"Amazon.co.jp ご注文の確認".to_string()));
        assert!(filters.contains(&"Amazon.co.jpでのご注文".to_string()));
        assert!(filters.contains(&"注文済み:".to_string()));
    }

    #[test]
    fn test_amazon_get_parser_returns_none() {
        assert!(AmazonPlugin.get_parser("amazon_confirm").is_none());
    }
}
