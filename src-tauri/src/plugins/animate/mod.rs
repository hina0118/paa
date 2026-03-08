//! アニメイト通販プラグイン
//!
//! `info@animate-onlineshop.jp` から配信される注文確認・出荷完了メールをパースする。
//! 文字コードは ISO-2022-JP だが、Gmail API 同期時に UTF-8 にデコード済みであることを前提とする。

pub mod parsers;

use async_trait::async_trait;

use crate::parsers::EmailParser;
use crate::repository::SqliteOrderRepository;

use super::{
    apply_internal_date, derive_shop_domain, DefaultShopSetting, DispatchError, DispatchOutcome,
    PluginRegistration, VendorPlugin,
};

pub struct AnimatePlugin;

#[async_trait]
impl VendorPlugin for AnimatePlugin {
    fn parser_types(&self) -> &[&str] {
        &["animate_confirm", "animate_send"]
    }

    fn priority(&self) -> i32 {
        10
    }

    fn get_parser(&self, parser_type: &str) -> Option<Box<dyn EmailParser>> {
        match parser_type {
            "animate_confirm" => Some(Box::new(parsers::confirm::AnimateConfirmParser)),
            "animate_send" => Some(Box::new(parsers::send::AnimateSendParser)),
            _ => None,
        }
    }

    fn shop_name(&self) -> &str {
        "アニメイト通販"
    }

    fn default_shop_settings(&self) -> Vec<DefaultShopSetting> {
        vec![
            DefaultShopSetting {
                shop_name: "アニメイト通販".to_string(),
                sender_address: "info@animate-onlineshop.jp".to_string(),
                parser_type: "animate_confirm".to_string(),
                // 「出荷完了のお知らせ」との誤マッチを防ぐため、ブランド名まで含めた
                // より具体的なフィルターを使用する（件名: 「【アニメイト通販】ご注文の確認」）
                subject_filters: Some(vec!["【アニメイト通販】ご注文の確認".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "アニメイト通販".to_string(),
                sender_address: "info@animate-onlineshop.jp".to_string(),
                parser_type: "animate_send".to_string(),
                subject_filters: Some(vec!["出荷完了".to_string()]),
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
        if parser_type == "animate_confirm" {
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
    factory: || Box::new(AnimatePlugin),
});
