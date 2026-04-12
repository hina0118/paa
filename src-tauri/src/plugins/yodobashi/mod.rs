//! ヨドバシ・ドット・コムプラグイン
//!
//! - 注文確認: `thanks_gochuumon@yodobashi.com`
//! - キャンセル: `cancel@yodobashi.com`

pub mod parsers;

use async_trait::async_trait;

use crate::parsers::EmailParser;
use crate::repository::SqliteOrderRepository;

use super::{
    derive_shop_domain, DefaultShopSetting, DispatchError, DispatchOutcome, PluginRegistration,
    VendorPlugin,
};

pub struct YodobashiPlugin;

#[async_trait]
impl VendorPlugin for YodobashiPlugin {
    fn parser_types(&self) -> &[&str] {
        &["yodobashi_confirm", "yodobashi_cancel"]
    }

    fn priority(&self) -> i32 {
        10
    }

    fn get_parser(&self, parser_type: &str) -> Option<Box<dyn EmailParser>> {
        match parser_type {
            "yodobashi_confirm" => Some(Box::new(parsers::confirm::YodobashiConfirmParser)),
            _ => None,
        }
    }

    fn prefer_plain_text(&self) -> bool {
        true
    }

    fn shop_name(&self) -> &str {
        "ヨドバシ・ドット・コム"
    }

    fn default_shop_settings(&self) -> Vec<DefaultShopSetting> {
        vec![
            DefaultShopSetting {
                shop_name: "ヨドバシ・ドット・コム".to_string(),
                sender_address: "thanks_gochuumon@yodobashi.com".to_string(),
                parser_type: "yodobashi_confirm".to_string(),
                subject_filters: Some(vec![
                    "ヨドバシ・ドット・コム：ご注文ありがとうございます".to_string()
                ]),
            },
            DefaultShopSetting {
                shop_name: "ヨドバシ・ドット・コム".to_string(),
                sender_address: "cancel@yodobashi.com".to_string(),
                parser_type: "yodobashi_cancel".to_string(),
                subject_filters: Some(vec![
                    "ヨドバシ・ドット・コム：ご注文内容変更のご連絡".to_string()
                ]),
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
        _internal_date: Option<i64>,
        body: &str,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    ) -> Result<DispatchOutcome, DispatchError> {
        let shop_domain = derive_shop_domain(from_address);

        match parser_type {
            "yodobashi_cancel" => {
                let cancel_infos = parsers::cancel::YodobashiCancelParser
                    .parse_cancel(body)
                    .map_err(DispatchError::ParseFailed)?;

                let order_number = cancel_infos[0].order_number.clone();

                log::debug!(
                    "[yodobashi_cancel] email_id={} order_number={} items={}",
                    email_id,
                    order_number,
                    cancel_infos.len()
                );

                // 複数商品が個別にキャンセルされる場合があるため商品ごとに適用する
                for cancel_info in &cancel_infos {
                    SqliteOrderRepository::apply_cancel_in_tx(
                        tx,
                        cancel_info,
                        email_id,
                        shop_domain.clone(),
                        None,
                    )
                    .await
                    .map_err(DispatchError::SaveFailed)?;
                }

                Ok(DispatchOutcome::CancelApplied { order_number })
            }

            _ => {
                // yodobashi_confirm およびその他
                let order_info = {
                    let parser = self.get_parser(parser_type).ok_or_else(|| {
                        DispatchError::ParseFailed(format!("No parser for type: {}", parser_type))
                    })?;
                    parser.parse(body).map_err(DispatchError::ParseFailed)?
                };

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
    }
}

inventory::submit!(PluginRegistration {
    factory: || Box::new(YodobashiPlugin),
});
