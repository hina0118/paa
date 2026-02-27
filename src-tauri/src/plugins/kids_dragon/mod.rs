//! キッズドラゴン（HOBBY SHOP キッズドラゴン）プラグイン
//!
//! おちゃのこネット（www41.ocnk.net）経由で配信されるメールをパースする。
//! 送信元アドレス `satiusukurukuru@yahoo.co.jp` から届く注文確認・発送通知に対応する。

pub mod parsers;

use async_trait::async_trait;

use crate::parsers::EmailParser;
use crate::repository::SqliteOrderRepository;

use super::{
    derive_shop_domain, DefaultShopSetting, DispatchError, DispatchOutcome, PluginRegistration,
    VendorPlugin,
};

pub struct KidsDragonPlugin;

#[async_trait]
impl VendorPlugin for KidsDragonPlugin {
    fn parser_types(&self) -> &[&str] {
        &["kids_dragon_confirm", "kids_dragon_send"]
    }

    fn priority(&self) -> i32 {
        10
    }

    fn get_parser(&self, parser_type: &str) -> Option<Box<dyn EmailParser>> {
        match parser_type {
            "kids_dragon_confirm" => Some(Box::new(parsers::confirm::KidsDragonConfirmParser)),
            "kids_dragon_send" => Some(Box::new(parsers::send::KidsDragonSendParser)),
            _ => None,
        }
    }

    fn shop_name(&self) -> &str {
        "キッズドラゴン"
    }

    fn default_shop_settings(&self) -> Vec<DefaultShopSetting> {
        vec![
            DefaultShopSetting {
                shop_name: "キッズドラゴン".to_string(),
                sender_address: "satiusukurukuru@yahoo.co.jp".to_string(),
                parser_type: "kids_dragon_confirm".to_string(),
                subject_filters: Some(vec![
                    "ご注文有難うございます　キッズドラゴンです".to_string()
                ]),
            },
            DefaultShopSetting {
                shop_name: "キッズドラゴン".to_string(),
                sender_address: "satiusukurukuru@yahoo.co.jp".to_string(),
                parser_type: "kids_dragon_send".to_string(),
                subject_filters: Some(vec!["発送が完了致しました".to_string()]),
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

        // parser は同期処理のみ。await をまたがないようブロックで即 drop する。
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

        let order_id = SqliteOrderRepository::save_order_in_tx(
            tx,
            &order_info,
            Some(email_id),
            shop_domain,
            Some(shop_name.to_string()),
        )
        .await
        .map_err(DispatchError::SaveFailed)?;

        // 発送通知は発送時の商品リストが最終状態のため、既存アイテムを置き換える。
        // 注文確認（confirm）より商品が増減している場合（分割発送等）に対応する。
        if parser_type == "kids_dragon_send" {
            SqliteOrderRepository::replace_items_for_order_in_tx(tx, order_id, &order_info)
                .await
                .map_err(DispatchError::SaveFailed)?;

            log::debug!(
                "[kids_dragon_send] Replaced items for order_id={} (order_number={})",
                order_id,
                order_info.order_number
            );
        }

        Ok(DispatchOutcome::OrderSaved(Box::new(order_info)))
    }
}

inventory::submit!(PluginRegistration {
    factory: || Box::new(KidsDragonPlugin),
});
