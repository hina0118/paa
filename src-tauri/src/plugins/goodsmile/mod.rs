//! グッドスマイルカンパニー公式ショップ プラグイン
//!
//! SendGrid 経由（`em1807.goodsmile.jp`）で配信されるメールをパースする。
//! 送信元アドレス `shop@goodsmile.jp` から届く注文確認・発送通知に対応する。

pub mod parsers;

use async_trait::async_trait;

use crate::parsers::EmailParser;
use crate::repository::SqliteOrderRepository;

use super::{
    derive_shop_domain, DefaultShopSetting, DispatchError, DispatchOutcome, PluginRegistration,
    VendorPlugin,
};

pub struct GoodSmilePlugin;

#[async_trait]
impl VendorPlugin for GoodSmilePlugin {
    fn parser_types(&self) -> &[&str] {
        &["goodsmile_confirm", "goodsmile_send"]
    }

    fn priority(&self) -> i32 {
        10
    }

    fn get_parser(&self, parser_type: &str) -> Option<Box<dyn EmailParser>> {
        match parser_type {
            "goodsmile_confirm" => Some(Box::new(parsers::confirm::GoodSmileConfirmParser)),
            "goodsmile_send" => Some(Box::new(parsers::send::GoodSmileSendParser)),
            _ => None,
        }
    }

    fn shop_name(&self) -> &str {
        "グッドスマイルカンパニー"
    }

    fn default_shop_settings(&self) -> Vec<DefaultShopSetting> {
        vec![
            DefaultShopSetting {
                shop_name: "グッドスマイルカンパニー".to_string(),
                sender_address: "shop@goodsmile.jp".to_string(),
                parser_type: "goodsmile_confirm".to_string(),
                subject_filters: Some(vec!["ご注文完了のお知らせ".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "グッドスマイルカンパニー".to_string(),
                sender_address: "shop@goodsmile.jp".to_string(),
                parser_type: "goodsmile_send".to_string(),
                subject_filters: Some(vec!["ご注文商品発送のお知らせ".to_string()]),
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

        // goodsmile_send は金額情報を持たないため、既存アイテム（confirm で登録済みの価格）を
        // 上書きしないよう save_order_in_tx のみ呼ぶ。
        // save_order_in_tx はアイテムが既存の場合はスキップするため、価格が保持される。
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
    factory: || Box::new(GoodSmilePlugin),
});
