//! コトブキヤオンラインショップ プラグイン
//!
//! `onlineshop@kotobukiya-ec.com` から配信される注文確認メールを取り込む。
//!
//! TODO: 実際のメール本文フォーマットを確認後、パーサーを実装する。

pub mod parsers;

use async_trait::async_trait;

use crate::parsers::EmailParser;
use crate::repository::SqliteOrderRepository;

use super::{
    apply_internal_date, derive_shop_domain, DefaultShopSetting, DispatchError, DispatchOutcome,
    PluginRegistration, VendorPlugin,
};

pub struct KotobukiyaPlugin;

#[async_trait]
impl VendorPlugin for KotobukiyaPlugin {
    fn parser_types(&self) -> &[&str] {
        &["kotobukiya_confirm"]
    }

    fn priority(&self) -> i32 {
        10
    }

    fn get_parser(&self, parser_type: &str) -> Option<Box<dyn EmailParser>> {
        match parser_type {
            "kotobukiya_confirm" => Some(Box::new(parsers::confirm::KotobukiyaConfirmParser)),
            _ => None,
        }
    }

    fn shop_name(&self) -> &str {
        "コトブキヤオンラインショップ"
    }

    fn default_shop_settings(&self) -> Vec<DefaultShopSetting> {
        vec![DefaultShopSetting {
            shop_name: "コトブキヤオンラインショップ".to_string(),
            sender_address: "onlineshop@kotobukiya-ec.com".to_string(),
            parser_type: "kotobukiya_confirm".to_string(),
            subject_filters: Some(vec![
                "ご注文確認のお知らせ［コトブキヤオンラインショップ］".to_string()
            ]),
        }]
    }

    fn prefer_plain_text(&self) -> bool {
        true
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

        apply_internal_date(&mut order_info, internal_date);

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
    factory: || Box::new(KotobukiyaPlugin),
});
