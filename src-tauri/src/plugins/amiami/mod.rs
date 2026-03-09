//! あみあみプラグイン
//!
//! あみあみは楽天市場経由と直販（amiami.com）の2販路があり、送信元アドレスが異なる。
//! 文字コードは ISO-2022-JP だが、Gmail API 同期時に UTF-8 にデコード済みであることを前提とする。
//!
//! | parser_type              | 送信元                           | 種別               |
//! |--------------------------|----------------------------------|--------------------|
//! | amiami_rakuten_confirm   | amiami@shop.rakuten.co.jp        | 楽天 注文確認      |
//! | amiami_rakuten_send      | amiami_2@shop.rakuten.co.jp      | 楽天 発送案内      |
//! | amiami_confirm           | order@amiami.com                 | 直販 注文確認      |
//! | amiami_send              | shop@amiami.com                  | 直販 発送案内      |
//! | amiami_cancel            | order@amiami.com / shop@amiami.com | キャンセル通知   |

pub mod parsers;

use async_trait::async_trait;

use crate::parsers::EmailParser;
use crate::repository::SqliteOrderRepository;

use super::{
    apply_internal_date, derive_shop_domain, DefaultShopSetting, DispatchError, DispatchOutcome,
    PluginRegistration, VendorPlugin,
};

pub struct AmiamiPlugin;

#[async_trait]
impl VendorPlugin for AmiamiPlugin {
    fn parser_types(&self) -> &[&str] {
        &[
            "amiami_rakuten_confirm",
            "amiami_rakuten_send",
            "amiami_confirm",
            "amiami_send",
            "amiami_cancel",
        ]
    }

    fn priority(&self) -> i32 {
        10
    }

    fn get_parser(&self, parser_type: &str) -> Option<Box<dyn EmailParser>> {
        match parser_type {
            "amiami_rakuten_confirm" => Some(Box::new(
                parsers::rakuten_confirm::AmiamiRakutenConfirmParser,
            )),
            "amiami_rakuten_send" => {
                Some(Box::new(parsers::rakuten_send::AmiamiRakutenSendParser))
            }
            "amiami_confirm" => Some(Box::new(parsers::confirm::AmiamiConfirmParser)),
            "amiami_send" => Some(Box::new(parsers::send::AmiamiSendParser)),
            // cancel は dispatch() 内で直接処理するため get_parser は None を返す
            _ => None,
        }
    }

    fn shop_name(&self) -> &str {
        "あみあみ"
    }

    fn default_shop_settings(&self) -> Vec<DefaultShopSetting> {
        vec![
            DefaultShopSetting {
                shop_name: "あみあみ".to_string(),
                sender_address: "amiami@shop.rakuten.co.jp".to_string(),
                parser_type: "amiami_rakuten_confirm".to_string(),
                subject_filters: Some(vec!["ご注文確認案内".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "あみあみ".to_string(),
                sender_address: "amiami_2@shop.rakuten.co.jp".to_string(),
                parser_type: "amiami_rakuten_send".to_string(),
                subject_filters: Some(vec!["発送案内".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "あみあみ".to_string(),
                sender_address: "order@amiami.com".to_string(),
                parser_type: "amiami_confirm".to_string(),
                subject_filters: Some(vec!["内容確認".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "あみあみ".to_string(),
                sender_address: "shop@amiami.com".to_string(),
                parser_type: "amiami_send".to_string(),
                subject_filters: Some(vec!["発送案内".to_string()]),
            },
            // キャンセル: order@amiami.com からの「キャンセルご依頼の内容確認」
            DefaultShopSetting {
                shop_name: "あみあみ".to_string(),
                sender_address: "order@amiami.com".to_string(),
                parser_type: "amiami_cancel".to_string(),
                subject_filters: Some(vec!["キャンセル".to_string()]),
            },
            // キャンセル: shop@amiami.com からの「キャンセルを承りました」
            DefaultShopSetting {
                shop_name: "あみあみ".to_string(),
                sender_address: "shop@amiami.com".to_string(),
                parser_type: "amiami_cancel".to_string(),
                subject_filters: Some(vec!["キャンセル".to_string()]),
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

        // ── キャンセル ──────────────────────────────────────────────────────────
        if parser_type == "amiami_cancel" {
            let cancel_info = parsers::cancel::AmiamiCancelParser
                .parse_cancel(body)
                .map_err(DispatchError::ParseFailed)?;

            log::debug!(
                "[amiami_cancel] email_id={} order_number={}",
                email_id,
                cancel_info.order_number
            );

            SqliteOrderRepository::apply_cancel_in_tx(
                tx,
                &cancel_info,
                email_id,
                shop_domain,
                None,
            )
            .await
            .map_err(DispatchError::SaveFailed)?;

            return Ok(DispatchOutcome::CancelApplied {
                order_number: cancel_info.order_number,
            });
        }

        // ── 通常注文（confirm / send）──────────────────────────────────────────
        let mut order_info = {
            let parser = self.get_parser(parser_type).ok_or_else(|| {
                DispatchError::ParseFailed(format!("No parser for type: {}", parser_type))
            })?;
            parser.parse(body).map_err(DispatchError::ParseFailed)?
        };

        // 注文確認メールは注文日が本文に含まれないため internal_date で補完する
        if matches!(
            parser_type,
            "amiami_rakuten_confirm" | "amiami_confirm"
        ) {
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
    factory: || Box::new(AmiamiPlugin),
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amiami_plugin_parser_types_includes_cancel() {
        let plugin = AmiamiPlugin;
        assert!(plugin.parser_types().contains(&"amiami_cancel"));
    }

    #[test]
    fn test_amiami_plugin_get_parser_cancel_returns_none() {
        // cancel は dispatch() 内で直接処理するため get_parser は None を返す
        let plugin = AmiamiPlugin;
        assert!(plugin.get_parser("amiami_cancel").is_none());
    }

    #[test]
    fn test_amiami_plugin_default_shop_settings_includes_cancel() {
        let settings = AmiamiPlugin.default_shop_settings();
        let cancel_settings: Vec<_> = settings
            .iter()
            .filter(|s| s.parser_type == "amiami_cancel")
            .collect();
        assert_eq!(cancel_settings.len(), 2, "cancel settings should have 2 entries");

        let cancel_senders: Vec<&str> = cancel_settings
            .iter()
            .map(|s| s.sender_address.as_str())
            .collect();
        assert!(cancel_senders.contains(&"order@amiami.com"));
        assert!(cancel_senders.contains(&"shop@amiami.com"));
    }
}
