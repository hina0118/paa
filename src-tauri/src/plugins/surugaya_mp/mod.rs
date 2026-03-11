//! 駿河屋マーケットプレイスプラグイン
//!
//! 駿河屋マーケットプレイスの注文受付・商品発送メールのパース対応。
//! 本店（surugaya）とは取引番号形式・送信元・メール形式が異なる別サービス。
//!
//! # メール本文について
//! メール本文は `text/html` (ISO-2022-JP) 形式だが、Gmail API 同期時に UTF-8 デコード済み。
//! 商品明細はメール本文に含まれないため、マイページHTMLパースで補完する。
//!
//! | parser_type          | 送信元                  | 種別     |
//! |----------------------|-------------------------|----------|
//! | surugaya_mp_confirm  | order@suruga-ya.jp      | 注文受付 |
//! | surugaya_mp_send     | reference@suruga-ya.jp  | 商品発送 |

pub mod html_parser;
pub mod parsers;

use async_trait::async_trait;

use crate::parsers::EmailParser;
use crate::repository::SqliteOrderRepository;

use super::{
    apply_internal_date, derive_shop_domain, DefaultShopSetting, DispatchError, DispatchOutcome,
    PluginRegistration, VendorPlugin,
};

pub struct SurugayaMpPlugin;

#[async_trait]
impl VendorPlugin for SurugayaMpPlugin {
    fn parser_types(&self) -> &[&str] {
        &["surugaya_mp_confirm", "surugaya_mp_send"]
    }

    fn priority(&self) -> i32 {
        10
    }

    fn get_parser(&self, parser_type: &str) -> Option<Box<dyn EmailParser>> {
        match parser_type {
            "surugaya_mp_confirm" => Some(Box::new(parsers::confirm::SurugayaMpConfirmParser)),
            "surugaya_mp_send" => Some(Box::new(parsers::send::SurugayaMpSendParser)),
            _ => None,
        }
    }

    fn shop_name(&self) -> &str {
        "駿河屋マーケットプレイス"
    }

    fn default_shop_settings(&self) -> Vec<DefaultShopSetting> {
        vec![
            DefaultShopSetting {
                shop_name: "駿河屋マーケットプレイス".to_string(),
                sender_address: "order@suruga-ya.jp".to_string(),
                parser_type: "surugaya_mp_confirm".to_string(),
                // 本店（surugaya_confirm）は「ご注文ありがとうございます」なので件名で区別できる
                subject_filters: Some(vec!["ご注文受付のお知らせ".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "駿河屋マーケットプレイス".to_string(),
                sender_address: "reference@suruga-ya.jp".to_string(),
                parser_type: "surugaya_mp_send".to_string(),
                subject_filters: Some(vec!["商品発送のお知らせ".to_string()]),
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

        // 注文日は本文に含まれないため internal_date で補完
        apply_internal_date(&mut order_info, internal_date);

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

        // マイページURLを htmls / order_htmls に登録する
        // confirm・send いずれのメールにも URL が含まれる
        if let Some(url) = extract_mypage_url(parser_type, body) {
            SqliteOrderRepository::insert_html_url_for_order_in_tx(tx, order_id, &url)
                .await
                .map_err(DispatchError::SaveFailed)?;
        }

        Ok(DispatchOutcome::OrderSaved(Box::new(order_info)))
    }
}

/// parser_type に応じてメール本文からマイページURLを抽出する
fn extract_mypage_url(parser_type: &str, body: &str) -> Option<String> {
    match parser_type {
        "surugaya_mp_confirm" => parsers::confirm::parse_mypage_url(body),
        "surugaya_mp_send" => parsers::send::parse_mypage_url(body),
        _ => None,
    }
}

inventory::submit!(PluginRegistration {
    factory: || Box::new(SurugayaMpPlugin),
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_parser_types() {
        let plugin = SurugayaMpPlugin;
        assert!(plugin.parser_types().contains(&"surugaya_mp_confirm"));
        assert!(plugin.parser_types().contains(&"surugaya_mp_send"));
    }

    #[test]
    fn test_plugin_get_parser_confirm() {
        assert!(SurugayaMpPlugin.get_parser("surugaya_mp_confirm").is_some());
    }

    #[test]
    fn test_plugin_get_parser_send() {
        assert!(SurugayaMpPlugin.get_parser("surugaya_mp_send").is_some());
    }

    #[test]
    fn test_plugin_get_parser_unknown_returns_none() {
        assert!(SurugayaMpPlugin.get_parser("unknown").is_none());
    }

    #[test]
    fn test_plugin_default_shop_settings() {
        let settings = SurugayaMpPlugin.default_shop_settings();
        assert_eq!(settings.len(), 2);

        let confirm = settings
            .iter()
            .find(|s| s.parser_type == "surugaya_mp_confirm")
            .unwrap();
        assert_eq!(confirm.sender_address, "order@suruga-ya.jp");
        assert_eq!(
            confirm.subject_filters.as_deref(),
            Some(["ご注文受付のお知らせ".to_string()].as_slice())
        );

        let send = settings
            .iter()
            .find(|s| s.parser_type == "surugaya_mp_send")
            .unwrap();
        assert_eq!(send.sender_address, "reference@suruga-ya.jp");
        assert_eq!(
            send.subject_filters.as_deref(),
            Some(["商品発送のお知らせ".to_string()].as_slice())
        );
    }

    #[test]
    fn test_shop_name() {
        assert_eq!(SurugayaMpPlugin.shop_name(), "駿河屋マーケットプレイス");
    }
}
