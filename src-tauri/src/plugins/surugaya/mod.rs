//! 駿河屋プラグイン
//!
//! 駿河屋の注文確認メール・発送案内メールのパース対応。
//! 文字コードは ISO-2022-JP だが、Gmail API 同期時に UTF-8 にデコード済みであることを前提とする。
//!
//! | parser_type       | 送信元               | 種別       |
//! |-------------------|----------------------|------------|
//! | surugaya_confirm  | order@suruga-ya.jp   | 注文確認   |
//! | surugaya_send     | order@suruga-ya.jp   | 発送案内   |

pub mod parsers;

use async_trait::async_trait;

use crate::parsers::EmailParser;
use crate::repository::SqliteOrderRepository;

use super::{
    apply_internal_date, derive_shop_domain, DefaultShopSetting, DispatchError, DispatchOutcome,
    PluginRegistration, VendorPlugin,
};

pub struct SurugayaPlugin;

#[async_trait]
impl VendorPlugin for SurugayaPlugin {
    fn parser_types(&self) -> &[&str] {
        &["surugaya_confirm", "surugaya_send"]
    }

    fn priority(&self) -> i32 {
        10
    }

    fn get_parser(&self, parser_type: &str) -> Option<Box<dyn EmailParser>> {
        match parser_type {
            "surugaya_confirm" => Some(Box::new(parsers::confirm::SurugayaConfirmParser)),
            "surugaya_send" => Some(Box::new(parsers::send::SurugayaSendParser)),
            _ => None,
        }
    }

    fn shop_name(&self) -> &str {
        "駿河屋"
    }

    fn default_shop_settings(&self) -> Vec<DefaultShopSetting> {
        vec![
            DefaultShopSetting {
                shop_name: "駿河屋".to_string(),
                sender_address: "order@suruga-ya.jp".to_string(),
                parser_type: "surugaya_confirm".to_string(),
                subject_filters: Some(vec!["ご注文ありがとうございます".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "駿河屋".to_string(),
                sender_address: "order@suruga-ya.jp".to_string(),
                parser_type: "surugaya_send".to_string(),
                subject_filters: Some(vec!["発送のお知らせ".to_string()]),
            },
        ]
    }

    #[allow(clippy::too_many_arguments)]
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
        let shop_domain = derive_shop_domain(from_address);

        let mut order_info = {
            let parser = self.get_parser(parser_type).ok_or_else(|| {
                DispatchError::ParseFailed(format!("No parser for type: {}", parser_type))
            })?;
            parser.parse(body).map_err(DispatchError::ParseFailed)?
        };

        // 注文確認メールは注文日が本文に含まれないため internal_date で補完する
        if parser_type == "surugaya_confirm" {
            apply_internal_date(&mut order_info, internal_date);
        }

        log::debug!(
            "[{}] email_id={} order_number={}",
            parser_type,
            email_id,
            order_info.order_number
        );

        SqliteOrderRepository::save_order_in_tx(
            tx,
            &order_info,
            Some(email_id),
            shop_domain,
            Some(shop_name.to_string()),
        )
        .await
        .map_err(DispatchError::SaveFailed)?;

        Ok(DispatchOutcome::OrderSaved(Box::new(order_info)))
    }
}

inventory::submit!(PluginRegistration {
    factory: || Box::new(SurugayaPlugin),
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surugaya_plugin_parser_types() {
        let plugin = SurugayaPlugin;
        assert!(plugin.parser_types().contains(&"surugaya_confirm"));
        assert!(plugin.parser_types().contains(&"surugaya_send"));
    }

    #[test]
    fn test_surugaya_plugin_get_parser_confirm() {
        let plugin = SurugayaPlugin;
        assert!(plugin.get_parser("surugaya_confirm").is_some());
    }

    #[test]
    fn test_surugaya_plugin_get_parser_send() {
        let plugin = SurugayaPlugin;
        assert!(plugin.get_parser("surugaya_send").is_some());
    }

    #[test]
    fn test_surugaya_plugin_get_parser_unknown_returns_none() {
        let plugin = SurugayaPlugin;
        assert!(plugin.get_parser("unknown").is_none());
    }

    #[test]
    fn test_surugaya_plugin_default_shop_settings() {
        let settings = SurugayaPlugin.default_shop_settings();
        assert_eq!(settings.len(), 2);

        let confirm = settings
            .iter()
            .find(|s| s.parser_type == "surugaya_confirm")
            .unwrap();
        assert_eq!(confirm.sender_address, "order@suruga-ya.jp");
        assert_eq!(
            confirm.subject_filters.as_deref(),
            Some(["ご注文ありがとうございます".to_string()].as_slice())
        );

        let send = settings
            .iter()
            .find(|s| s.parser_type == "surugaya_send")
            .unwrap();
        assert_eq!(send.sender_address, "order@suruga-ya.jp");
        assert_eq!(
            send.subject_filters.as_deref(),
            Some(["発送のお知らせ".to_string()].as_slice())
        );
    }
}
