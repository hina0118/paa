//! ふるいちオンライン プラグイン
//!
//! `info@furu1.online` から配信される注文確認・発送通知メールをパースする。
//! 文字コードは quoted-printable UTF-8。

pub mod parsers;

use async_trait::async_trait;

use crate::parsers::EmailParser;
use crate::repository::SqliteOrderRepository;

use super::{
    apply_internal_date, derive_shop_domain, DefaultShopSetting, DispatchError, DispatchOutcome,
    PluginRegistration, VendorPlugin,
};

pub struct FuruichiOnlinePlugin;

#[async_trait]
impl VendorPlugin for FuruichiOnlinePlugin {
    fn parser_types(&self) -> &[&str] {
        &["furuichi_confirm", "furuichi_send"]
    }

    fn priority(&self) -> i32 {
        10
    }

    fn get_parser(&self, parser_type: &str) -> Option<Box<dyn EmailParser>> {
        match parser_type {
            "furuichi_confirm" => Some(Box::new(parsers::confirm::FuruichiConfirmParser)),
            "furuichi_send" => Some(Box::new(parsers::send::FuruichiSendParser)),
            _ => None,
        }
    }

    fn shop_name(&self) -> &str {
        "ふるいちオンライン"
    }

    fn default_shop_settings(&self) -> Vec<DefaultShopSetting> {
        vec![
            DefaultShopSetting {
                shop_name: "ふるいちオンライン".to_string(),
                sender_address: "info@furu1.online".to_string(),
                parser_type: "furuichi_confirm".to_string(),
                subject_filters: Some(vec![
                    "【ふるいちオンライン】 ご注文ありがとうございます".to_string()
                ]),
            },
            DefaultShopSetting {
                shop_name: "ふるいちオンライン".to_string(),
                sender_address: "info@furu1.online".to_string(),
                parser_type: "furuichi_send".to_string(),
                subject_filters: Some(vec!["【ふるいちオンライン】商品発送のお知らせ".to_string()]),
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

        // confirm はメール本文に注文日が含まれるが、パース失敗時の fallback として呼ぶ
        // apply_internal_date は order_date が Some の場合は何もしない
        if parser_type == "furuichi_confirm" {
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
    factory: || Box::new(FuruichiOnlinePlugin),
});
