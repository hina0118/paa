//! DMM 通販プラグイン
//!
//! `parsers/` の各パーサーを使用して DMM 固有の処理を実装する。
//!
//! # alternate_domains
//! DMM の注文確認メールは `mail.dmm.com` / `mono.dmm.com` のどちらかから届く。
//! キャンセル・注文番号変更メールは `mail.dmm.com` から届くが、注文検索では両方を試す。

pub mod parsers;

use async_trait::async_trait;

use crate::parsers::EmailParser;
use crate::repository::SqliteOrderRepository;

use super::{
    apply_internal_date, derive_shop_domain, DefaultShopSetting, DispatchError, DispatchOutcome,
    PluginRegistration, VendorPlugin,
};

pub struct DmmPlugin;

#[async_trait]
impl VendorPlugin for DmmPlugin {
    fn parser_types(&self) -> &[&str] {
        &[
            "dmm_confirm",
            "dmm_send",
            "dmm_cancel",
            "dmm_order_number_change",
            "dmm_split_complete",
            "dmm_merge_complete",
        ]
    }

    fn priority(&self) -> i32 {
        10
    }

    /// `OrderInfo` を返すパーサーのみ。
    /// cancel / order_number_change / merge_complete は `dispatch()` 内で直接処理する。
    fn get_parser(&self, parser_type: &str) -> Option<Box<dyn EmailParser>> {
        match parser_type {
            "dmm_confirm" => Some(Box::new(parsers::confirm::DmmConfirmParser)),
            "dmm_send" => Some(Box::new(parsers::send::DmmSendParser)),
            "dmm_split_complete" => Some(Box::new(parsers::split_complete::DmmSplitCompleteParser)),
            _ => None,
        }
    }

    fn alternate_domains(&self, domain: &str) -> Option<Vec<String>> {
        match domain {
            "mail.dmm.com" => Some(vec!["mono.dmm.com".into()]),
            "mono.dmm.com" => Some(vec!["mail.dmm.com".into()]),
            _ => None,
        }
    }

    fn shop_name(&self) -> &str {
        "DMM通販"
    }

    fn default_shop_settings(&self) -> Vec<DefaultShopSetting> {
        vec![
            DefaultShopSetting {
                shop_name: "DMM通販".to_string(),
                sender_address: "info@mail.dmm.com".to_string(),
                parser_type: "dmm_confirm".to_string(),
                subject_filters: Some(vec![
                    "DMM通販：ご注文手続き完了のお知らせ".to_string(),
                    "DMM通販:ご注文手続き完了のお知らせ".to_string(),
                    "ご注文手続き完了のお知らせ".to_string(),
                ]),
            },
            DefaultShopSetting {
                shop_name: "DMM通販".to_string(),
                sender_address: "info@mono.dmm.com".to_string(),
                parser_type: "dmm_confirm".to_string(),
                subject_filters: Some(vec![
                    "DMM通販：ご注文手続き完了のお知らせ".to_string(),
                    "DMM通販:ご注文手続き完了のお知らせ".to_string(),
                    "ご注文手続き完了のお知らせ".to_string(),
                ]),
            },
            DefaultShopSetting {
                shop_name: "DMM通販".to_string(),
                sender_address: "info@mail.dmm.com".to_string(),
                parser_type: "dmm_cancel".to_string(),
                subject_filters: Some(vec!["DMM通販：ご注文キャンセルのお知らせ".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "DMM通販".to_string(),
                sender_address: "info@mail.dmm.com".to_string(),
                parser_type: "dmm_order_number_change".to_string(),
                subject_filters: Some(vec![
                    "DMM通販：配送センター変更に伴うご注文番号変更のお知らせ".to_string(),
                ]),
            },
            DefaultShopSetting {
                shop_name: "DMM通販".to_string(),
                sender_address: "info@mail.dmm.com".to_string(),
                parser_type: "dmm_split_complete".to_string(),
                subject_filters: Some(vec!["DMM通販：ご注文分割完了のお知らせ".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "DMM通販".to_string(),
                sender_address: "info@mail.dmm.com".to_string(),
                parser_type: "dmm_merge_complete".to_string(),
                subject_filters: Some(vec!["DMM通販：ご注文まとめ完了のお知らせ".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "DMM通販".to_string(),
                sender_address: "info@mail.dmm.com".to_string(),
                parser_type: "dmm_send".to_string(),
                subject_filters: Some(vec![
                    "DMM通販：ご注文商品を発送いたしました".to_string(),
                    "ご注文商品を発送いたしました".to_string(),
                ]),
            },
            DefaultShopSetting {
                shop_name: "DMM通販".to_string(),
                sender_address: "info@mono.dmm.com".to_string(),
                parser_type: "dmm_send".to_string(),
                subject_filters: Some(vec![
                    "DMM通販：ご注文商品を発送いたしました".to_string(),
                    "ご注文商品を発送いたしました".to_string(),
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
        internal_date: Option<i64>,
        body: &str,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    ) -> Result<DispatchOutcome, DispatchError> {
        let shop_domain = derive_shop_domain(from_address);
        let alt_domains = self.alternate_domains(shop_domain.as_deref().unwrap_or(""));

        match parser_type {
            // ── キャンセル ─────────────────────────────────────────────────────
            "dmm_cancel" => {
                let cancel_info = parsers::cancel::DmmCancelParser
                    .parse_cancel(body)
                    .map_err(DispatchError::ParseFailed)?;

                log::debug!(
                    "[dmm_cancel] email_id={} internal_date={:?} order_number={}",
                    email_id,
                    internal_date,
                    cancel_info.order_number
                );

                SqliteOrderRepository::apply_cancel_in_tx(
                    tx,
                    &cancel_info,
                    email_id,
                    shop_domain,
                    alt_domains,
                )
                .await
                .map_err(DispatchError::SaveFailed)?;

                Ok(DispatchOutcome::CancelApplied {
                    order_number: cancel_info.order_number,
                })
            }

            // ── 注文番号変更 ────────────────────────────────────────────────────
            "dmm_order_number_change" => {
                let change_info = parsers::order_number_change::DmmOrderNumberChangeParser
                    .parse_order_number_change(body)
                    .map_err(DispatchError::ParseFailed)?;

                log::debug!(
                    "[dmm_order_number_change] email_id={} {} -> {}",
                    email_id,
                    change_info.old_order_number,
                    change_info.new_order_number
                );

                SqliteOrderRepository::apply_order_number_change_in_tx(
                    tx,
                    &change_info,
                    email_id,
                    internal_date,
                    shop_domain,
                    Some(shop_name.to_string()),
                    alt_domains,
                )
                .await
                .map_err(DispatchError::SaveFailed)?;

                Ok(DispatchOutcome::OrderNumberChanged {
                    new_order_number: change_info.new_order_number,
                })
            }

            // ── まとめ完了 ───────────────────────────────────────────────────────
            "dmm_merge_complete" => {
                let consolidation_info = parsers::merge_complete::DmmMergeCompleteParser
                    .parse_consolidation(body)
                    .map_err(DispatchError::ParseFailed)?;

                log::debug!(
                    "[dmm_merge_complete] email_id={} {:?} -> {}",
                    email_id,
                    consolidation_info.old_order_numbers,
                    consolidation_info.new_order_number
                );

                SqliteOrderRepository::apply_consolidation_in_tx(
                    tx,
                    &consolidation_info,
                    email_id,
                    shop_domain,
                    alt_domains,
                )
                .await
                .map_err(DispatchError::SaveFailed)?;

                Ok(DispatchOutcome::ConsolidationApplied {
                    new_order_number: consolidation_info.new_order_number,
                })
            }

            // ── 分割完了（複数注文）────────────────────────────────────────────
            "dmm_split_complete" => {
                let parser = parsers::split_complete::DmmSplitCompleteParser;

                let orders = parser
                    .parse_multi(body)
                    .ok_or_else(|| {
                        DispatchError::ParseFailed("parse_multi returned None".to_string())
                    })?
                    .map_err(DispatchError::ParseFailed)?;

                if orders.is_empty() {
                    return Err(DispatchError::ParseFailed(
                        "Parser returned empty orders".to_string(),
                    ));
                }

                let total_orders = orders.len();
                let mut saved_orders = Vec::with_capacity(total_orders);

                for (idx, mut order_info) in orders.into_iter().enumerate() {
                    // dmm_split_complete: order_date が None の場合は internal_date を補完
                    apply_internal_date(&mut order_info, internal_date);

                    let save_result = if idx == 0 {
                        SqliteOrderRepository::apply_split_first_order_in_tx(
                            tx,
                            &order_info,
                            Some(email_id),
                            shop_domain.clone(),
                            Some(shop_name.to_string()),
                            alt_domains.clone(),
                        )
                        .await
                    } else {
                        SqliteOrderRepository::save_order_in_tx(
                            tx,
                            &order_info,
                            Some(email_id),
                            shop_domain.clone(),
                            Some(shop_name.to_string()),
                        )
                        .await
                    };

                    match save_result {
                        Ok(order_id) => {
                            log::debug!(
                                "[dmm_split_complete] Saved order {} ({}/{}) for email {}",
                                order_id,
                                idx + 1,
                                total_orders,
                                email_id
                            );
                            saved_orders.push(order_info);
                        }
                        Err(e) => {
                            log::error!(
                                "[dmm_split_complete] Failed to save order {}/{} for email {}: {}",
                                idx + 1,
                                total_orders,
                                email_id,
                                e
                            );
                            return Err(DispatchError::SaveFailed(format!(
                                "Split order save failed for email {}: {}",
                                email_id, e
                            )));
                        }
                    }
                }

                Ok(DispatchOutcome::MultiOrderSaved(saved_orders))
            }

            // ── 通常注文（confirm / send）──────────────────────────────────────
            _ => {
                // parser は同期処理のみ。await をまたがないようブロックで即 drop する。
                let mut order_info = {
                    let parser = self.get_parser(parser_type).ok_or_else(|| {
                        DispatchError::ParseFailed(format!("No parser for type: {}", parser_type))
                    })?;
                    parser.parse(body).map_err(DispatchError::ParseFailed)?
                };

                // dmm_confirm: internal_date を order_date に使用
                if parser_type == "dmm_confirm" {
                    apply_internal_date(&mut order_info, internal_date);
                }

                let save_result = if parser_type == "dmm_send" {
                    // 発送完了: 発送メール時点の items + 金額で元注文を更新しつつ delivery を shipped に変更
                    SqliteOrderRepository::apply_send_and_replace_items_in_tx(
                        tx,
                        &order_info,
                        Some(email_id),
                        shop_domain,
                        Some(shop_name.to_string()),
                        alt_domains,
                    )
                    .await
                } else {
                    SqliteOrderRepository::save_order_in_tx(
                        tx,
                        &order_info,
                        Some(email_id),
                        shop_domain,
                        Some(shop_name.to_string()),
                    )
                    .await
                };

                save_result.map_err(DispatchError::SaveFailed)?;

                Ok(DispatchOutcome::OrderSaved(Box::new(order_info)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dmm_plugin_parser_types() {
        let plugin = DmmPlugin;
        let types = plugin.parser_types();
        assert!(types.contains(&"dmm_confirm"));
        assert!(types.contains(&"dmm_send"));
        assert!(types.contains(&"dmm_cancel"));
        assert!(types.contains(&"dmm_order_number_change"));
        assert!(types.contains(&"dmm_split_complete"));
        assert!(types.contains(&"dmm_merge_complete"));
    }

    #[test]
    fn test_dmm_plugin_priority() {
        assert_eq!(DmmPlugin.priority(), 10);
    }

    #[test]
    fn test_dmm_plugin_get_parser_confirm() {
        let plugin = DmmPlugin;
        assert!(plugin.get_parser("dmm_confirm").is_some());
    }

    #[test]
    fn test_dmm_plugin_get_parser_send() {
        let plugin = DmmPlugin;
        assert!(plugin.get_parser("dmm_send").is_some());
    }

    #[test]
    fn test_dmm_plugin_get_parser_split_complete() {
        let plugin = DmmPlugin;
        assert!(plugin.get_parser("dmm_split_complete").is_some());
    }

    #[test]
    fn test_dmm_plugin_get_parser_cancel_returns_none() {
        // cancel / order_number_change / merge_complete は dispatch() 内で直接処理
        let plugin = DmmPlugin;
        assert!(plugin.get_parser("dmm_cancel").is_none());
        assert!(plugin.get_parser("dmm_order_number_change").is_none());
        assert!(plugin.get_parser("dmm_merge_complete").is_none());
    }

    #[test]
    fn test_dmm_alternate_domains_mail() {
        let plugin = DmmPlugin;
        assert_eq!(
            plugin.alternate_domains("mail.dmm.com"),
            Some(vec!["mono.dmm.com".to_string()])
        );
    }

    #[test]
    fn test_dmm_alternate_domains_mono() {
        let plugin = DmmPlugin;
        assert_eq!(
            plugin.alternate_domains("mono.dmm.com"),
            Some(vec!["mail.dmm.com".to_string()])
        );
    }

    #[test]
    fn test_dmm_alternate_domains_other() {
        let plugin = DmmPlugin;
        assert_eq!(plugin.alternate_domains("example.com"), None);
        assert_eq!(plugin.alternate_domains(""), None);
    }

    #[test]
    fn test_dmm_shop_name() {
        assert_eq!(DmmPlugin.shop_name(), "DMM通販");
    }

    #[test]
    fn test_dmm_default_shop_settings_count() {
        assert_eq!(DmmPlugin.default_shop_settings().len(), 8);
    }

    #[test]
    fn test_dmm_default_shop_settings_parser_types() {
        let settings = DmmPlugin.default_shop_settings();
        let parser_types: Vec<&str> = settings.iter().map(|s| s.parser_type.as_str()).collect();
        assert!(parser_types.contains(&"dmm_confirm"));
        assert!(parser_types.contains(&"dmm_cancel"));
        assert!(parser_types.contains(&"dmm_order_number_change"));
        assert!(parser_types.contains(&"dmm_split_complete"));
        assert!(parser_types.contains(&"dmm_merge_complete"));
        assert!(parser_types.contains(&"dmm_send"));
    }
}

inventory::submit!(PluginRegistration {
    factory: || Box::new(DmmPlugin),
});
