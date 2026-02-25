//! ホビーサーチプラグイン
//!
//! `parsers/hobbysearch/` の各パーサーを使用してホビーサーチ固有の処理を実装する。

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::DateTime;

use crate::parsers::hobbysearch;
use crate::parsers::EmailParser;
use crate::repository::OrderRepository;

use super::{
    apply_internal_date, derive_shop_domain, save_images_for_order, DispatchError, DispatchOutcome,
    VendorPlugin,
};

pub struct HobbySearchPlugin;

#[async_trait]
impl VendorPlugin for HobbySearchPlugin {
    fn parser_types(&self) -> &[&str] {
        &[
            "hobbysearch_confirm",
            "hobbysearch_confirm_yoyaku",
            "hobbysearch_change",
            "hobbysearch_change_yoyaku",
            "hobbysearch_send",
            "hobbysearch_cancel",
        ]
    }

    fn priority(&self) -> i32 {
        10
    }

    /// `OrderInfo` を返すパーサーのみ。
    /// cancel は `dispatch()` 内で直接処理する。
    fn get_parser(&self, parser_type: &str) -> Option<Box<dyn EmailParser>> {
        match parser_type {
            "hobbysearch_confirm" => Some(Box::new(hobbysearch::confirm::HobbySearchConfirmParser)),
            "hobbysearch_confirm_yoyaku" => Some(Box::new(
                hobbysearch::confirm_yoyaku::HobbySearchConfirmYoyakuParser,
            )),
            "hobbysearch_change" => Some(Box::new(hobbysearch::change::HobbySearchChangeParser)),
            "hobbysearch_change_yoyaku" => Some(Box::new(
                hobbysearch::change_yoyaku::HobbySearchChangeYoyakuParser,
            )),
            "hobbysearch_send" => Some(Box::new(hobbysearch::send::HobbySearchSendParser)),
            _ => None,
        }
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
        order_repo: &dyn OrderRepository,
        image_save_ctx: &Option<(Arc<sqlx::SqlitePool>, PathBuf)>,
    ) -> Result<DispatchOutcome, DispatchError> {
        let shop_domain = derive_shop_domain(from_address);

        match parser_type {
            // ── キャンセル ─────────────────────────────────────────────────────
            "hobbysearch_cancel" => {
                let cancel_info = hobbysearch::cancel::HobbySearchCancelParser
                    .parse_cancel(body)
                    .map_err(DispatchError::ParseFailed)?;

                log::debug!(
                    "[hobbysearch_cancel] email_id={} order_number={}",
                    email_id,
                    cancel_info.order_number
                );

                order_repo
                    .apply_cancel(
                        &cancel_info,
                        email_id,
                        shop_domain,
                        Some(shop_name.to_string()),
                        None, // ホビーサーチは追加ドメインなし
                    )
                    .await
                    .map_err(DispatchError::SaveFailed)?;

                Ok(DispatchOutcome::CancelApplied {
                    order_number: cancel_info.order_number,
                })
            }

            // ── 組み換え（変更・予約変更）──────────────────────────────────────
            "hobbysearch_change" | "hobbysearch_change_yoyaku" => {
                // parser は同期処理のみ。await をまたがないようブロックで即 drop する。
                let mut order_info = {
                    let parser = self.get_parser(parser_type).ok_or_else(|| {
                        DispatchError::ParseFailed(format!("No parser for type: {}", parser_type))
                    })?;
                    parser.parse(body).map_err(DispatchError::ParseFailed)?
                };

                // hobbysearch_change / change_yoyaku: internal_date を order_date に使用
                apply_internal_date(&mut order_info, internal_date);

                // internal_date が無効値の場合、apply_change_items_and_save_order をスキップして
                // save_order にフォールバックする（データ欠損よりは安全）
                let change_email_internal_date =
                    internal_date.and_then(|ts| DateTime::from_timestamp_millis(ts).map(|_| ts));

                let save_result = if let Some(ts) = change_email_internal_date {
                    order_repo
                        .apply_change_items_and_save_order(
                            &order_info,
                            Some(email_id),
                            shop_domain,
                            Some(shop_name.to_string()),
                            Some(ts),
                        )
                        .await
                } else {
                    log::warn!(
                        "[hobbysearch_change] Invalid internal_date for email {}, fallback to save_order",
                        email_id
                    );
                    order_repo
                        .save_order(
                            &order_info,
                            Some(email_id),
                            shop_domain,
                            Some(shop_name.to_string()),
                        )
                        .await
                };

                save_result.map_err(DispatchError::SaveFailed)?;

                save_images_for_order(&order_info, image_save_ctx).await;

                Ok(DispatchOutcome::OrderSaved(Box::new(order_info)))
            }

            // ── 通常注文（confirm / confirm_yoyaku / send）────────────────────
            _ => {
                // parse_multi を先に試す（hobbysearch_send の複数注文同時発送に対応）。
                // None なら parse() で単一注文として処理する。
                // parser は同期処理のみ。await をまたがないようブロックで即 drop する。
                let (multi_orders, single_order) = {
                    let parser = self.get_parser(parser_type).ok_or_else(|| {
                        DispatchError::ParseFailed(format!("No parser for type: {}", parser_type))
                    })?;

                    match parser.parse_multi(body) {
                        Some(Ok(orders)) if !orders.is_empty() => (Some(orders), None),
                        Some(Ok(_)) => {
                            return Err(DispatchError::ParseFailed(
                                "Parser returned empty orders".to_string(),
                            ));
                        }
                        Some(Err(e)) => {
                            return Err(DispatchError::ParseFailed(e));
                        }
                        None => (
                            None,
                            Some(parser.parse(body).map_err(DispatchError::ParseFailed)?),
                        ),
                    }
                };

                if let Some(orders) = multi_orders {
                    // 複数注文（hobbysearch_send の複数注文同時発送）
                    let total_orders = orders.len();
                    let mut saved_orders = Vec::with_capacity(total_orders);

                    for (idx, order_info) in orders.into_iter().enumerate() {
                        order_repo
                            .save_order(
                                &order_info,
                                Some(email_id),
                                shop_domain.clone(),
                                Some(shop_name.to_string()),
                            )
                            .await
                            .map_err(|e| {
                                DispatchError::SaveFailed(format!(
                                    "Multi-order save failed ({}/{}) for email {}: {}",
                                    idx + 1,
                                    total_orders,
                                    email_id,
                                    e
                                ))
                            })?;

                        log::debug!(
                            "[{}] Saved order ({}/{}) for email {}",
                            parser_type,
                            idx + 1,
                            total_orders,
                            email_id
                        );
                        save_images_for_order(&order_info, image_save_ctx).await;
                        saved_orders.push(order_info);
                    }

                    return Ok(DispatchOutcome::MultiOrderSaved(saved_orders));
                }

                // 単一注文（confirm / confirm_yoyaku / send で [注文番号] セクションなし）
                // SAFETY: multi_orders が None のとき single_order は必ず Some
                let mut order_info = single_order.unwrap();

                // hobbysearch_confirm / confirm_yoyaku: internal_date を order_date に使用
                if matches!(
                    parser_type,
                    "hobbysearch_confirm" | "hobbysearch_confirm_yoyaku"
                ) {
                    apply_internal_date(&mut order_info, internal_date);
                }

                order_repo
                    .save_order(
                        &order_info,
                        Some(email_id),
                        shop_domain,
                        Some(shop_name.to_string()),
                    )
                    .await
                    .map_err(DispatchError::SaveFailed)?;

                save_images_for_order(&order_info, image_save_ctx).await;

                Ok(DispatchOutcome::OrderSaved(Box::new(order_info)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hobbysearch_plugin_parser_types() {
        let plugin = HobbySearchPlugin;
        let types = plugin.parser_types();
        assert!(types.contains(&"hobbysearch_confirm"));
        assert!(types.contains(&"hobbysearch_confirm_yoyaku"));
        assert!(types.contains(&"hobbysearch_change"));
        assert!(types.contains(&"hobbysearch_change_yoyaku"));
        assert!(types.contains(&"hobbysearch_send"));
        assert!(types.contains(&"hobbysearch_cancel"));
    }

    #[test]
    fn test_hobbysearch_plugin_priority() {
        assert_eq!(HobbySearchPlugin.priority(), 10);
    }

    #[test]
    fn test_hobbysearch_plugin_get_parser_confirm() {
        let plugin = HobbySearchPlugin;
        assert!(plugin.get_parser("hobbysearch_confirm").is_some());
    }

    #[test]
    fn test_hobbysearch_plugin_get_parser_confirm_yoyaku() {
        let plugin = HobbySearchPlugin;
        assert!(plugin.get_parser("hobbysearch_confirm_yoyaku").is_some());
    }

    #[test]
    fn test_hobbysearch_plugin_get_parser_change() {
        let plugin = HobbySearchPlugin;
        assert!(plugin.get_parser("hobbysearch_change").is_some());
    }

    #[test]
    fn test_hobbysearch_plugin_get_parser_change_yoyaku() {
        let plugin = HobbySearchPlugin;
        assert!(plugin.get_parser("hobbysearch_change_yoyaku").is_some());
    }

    #[test]
    fn test_hobbysearch_plugin_get_parser_send() {
        let plugin = HobbySearchPlugin;
        assert!(plugin.get_parser("hobbysearch_send").is_some());
    }

    #[test]
    fn test_hobbysearch_plugin_get_parser_cancel_returns_none() {
        // cancel は dispatch() 内で直接処理するため get_parser は None を返す
        let plugin = HobbySearchPlugin;
        assert!(plugin.get_parser("hobbysearch_cancel").is_none());
    }

    #[test]
    fn test_hobbysearch_no_alternate_domains() {
        let plugin = HobbySearchPlugin;
        assert_eq!(plugin.alternate_domains("order.hobbysearch.co.jp"), None);
        assert_eq!(plugin.alternate_domains(""), None);
    }
}
