//! プレミアムバンダイ プラグイン
//!
//! 送信元アドレス `evidence_bc@p-bandai.jp` / `evidence_info@p-bandai.jp` から届く
//! 注文確認・おまとめ完了・発送通知メールに対応する。

pub mod parsers;

use async_trait::async_trait;
use chrono::DateTime;

use crate::parsers::EmailParser;
use crate::repository::SqliteOrderRepository;

use super::{
    apply_internal_date, derive_shop_domain, DefaultShopSetting, DispatchError, DispatchOutcome,
    PluginRegistration, VendorPlugin,
};

pub struct PremiumBandaiPlugin;

#[async_trait]
impl VendorPlugin for PremiumBandaiPlugin {
    fn parser_types(&self) -> &[&str] {
        &[
            "premium_bandai_confirm",
            "premium_bandai_omatome",
            "premium_bandai_send",
        ]
    }

    fn priority(&self) -> i32 {
        10
    }

    fn get_parser(&self, parser_type: &str) -> Option<Box<dyn EmailParser>> {
        match parser_type {
            "premium_bandai_confirm" => {
                Some(Box::new(parsers::confirm::PremiumBandaiConfirmParser))
            }
            "premium_bandai_omatome" => {
                Some(Box::new(parsers::omatome::PremiumBandaiOmatomeParser))
            }
            "premium_bandai_send" => Some(Box::new(parsers::send::PremiumBandaiSendParser)),
            _ => None,
        }
    }

    fn shop_name(&self) -> &str {
        "プレミアムバンダイ"
    }

    fn default_shop_settings(&self) -> Vec<DefaultShopSetting> {
        vec![
            DefaultShopSetting {
                shop_name: "プレミアムバンダイ".to_string(),
                sender_address: "evidence_bc@p-bandai.jp".to_string(),
                parser_type: "premium_bandai_confirm".to_string(),
                subject_filters: Some(vec!["ご注文完了のお知らせ".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "プレミアムバンダイ".to_string(),
                sender_address: "evidence_bc@p-bandai.jp".to_string(),
                parser_type: "premium_bandai_omatome".to_string(),
                subject_filters: Some(vec!["ご注文おまとめ完了のお知らせ".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "プレミアムバンダイ".to_string(),
                sender_address: "evidence_bc@p-bandai.jp".to_string(),
                parser_type: "premium_bandai_send".to_string(),
                subject_filters: Some(vec!["商品発送完了のお知らせ".to_string()]),
            },
            // evidence_info@p-bandai.jp からも同一件名で届く場合に対応
            DefaultShopSetting {
                shop_name: "プレミアムバンダイ".to_string(),
                sender_address: "evidence_info@p-bandai.jp".to_string(),
                parser_type: "premium_bandai_confirm".to_string(),
                subject_filters: Some(vec!["ご注文完了のお知らせ".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "プレミアムバンダイ".to_string(),
                sender_address: "evidence_info@p-bandai.jp".to_string(),
                parser_type: "premium_bandai_omatome".to_string(),
                subject_filters: Some(vec!["ご注文おまとめ完了のお知らせ".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "プレミアムバンダイ".to_string(),
                sender_address: "evidence_info@p-bandai.jp".to_string(),
                parser_type: "premium_bandai_send".to_string(),
                subject_filters: Some(vec!["商品発送完了のお知らせ".to_string()]),
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

        match parser_type {
            // ── おまとめ完了：元注文を商品名マッチングで無効化し、新注文を保存 ──
            "premium_bandai_omatome" => {
                let mut order_info = {
                    let parser = self.get_parser(parser_type).ok_or_else(|| {
                        DispatchError::ParseFailed(format!("No parser for type: {}", parser_type))
                    })?;
                    parser.parse(body).map_err(DispatchError::ParseFailed)?
                };

                apply_internal_date(&mut order_info, internal_date);

                let change_email_internal_date =
                    internal_date.and_then(|ts| DateTime::from_timestamp_millis(ts).map(|_| ts));

                let save_result: Result<i64, String> = if let Some(ts) = change_email_internal_date
                {
                    match SqliteOrderRepository::apply_change_items_in_tx(
                        tx,
                        &order_info,
                        shop_domain.clone(),
                        Some(ts),
                    )
                    .await
                    {
                        Ok(()) => {
                            if let Some(ref d) = shop_domain {
                                if !d.is_empty() {
                                    if let Err(e) =
                                        cleanup_phantom_omatome_items_in_tx(tx, &order_info, d, ts)
                                            .await
                                    {
                                        log::warn!(
                                            "[premium_bandai_omatome] phantom cleanup failed: {}",
                                            e
                                        );
                                    }
                                }
                            }
                            SqliteOrderRepository::save_order_in_tx(
                                tx,
                                &order_info,
                                Some(email_id),
                                shop_domain,
                                Some(shop_name.to_string()),
                            )
                            .await
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    log::warn!(
                        "[premium_bandai_omatome] Invalid internal_date for email {}, fallback to save_order",
                        email_id
                    );
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

                log::debug!(
                    "[premium_bandai_omatome] email_id={} order_number={}",
                    email_id,
                    order_info.order_number
                );

                Ok(DispatchOutcome::OrderSaved(Box::new(order_info)))
            }

            // ── confirm / send：通常保存 ──
            _ => {
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

                // premium_bandai_send は金額情報を持たないため、
                // 既存アイテム（confirm で登録済みの価格）を上書きしないよう save_order_in_tx を使う。
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

/// おまとめメールが confirm より先に届いた場合に発生するファントムアイテムをクリーンアップする。
///
/// `apply_change_items_in_tx` は confirm 注文でアイテムが見つかった時点で停止するため、
/// 同じ商品を持つ先行おまとめ注文（ファントム）に残ったアイテムが削除されないことがある。
/// この関数は候補の中からおまとめメール由来の注文のみを対象とし、
/// 現在のおまとめ注文と一致するアイテムを完全削除する。
async fn cleanup_phantom_omatome_items_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    order_info: &crate::parsers::OrderInfo,
    shop_domain: &str,
    cutoff_ts: i64,
) -> Result<(), String> {
    let omatome_order_ids: Vec<i64> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT o.id FROM orders o
        JOIN order_emails oe ON oe.order_id = o.id
        JOIN emails e ON e.id = oe.email_id
        WHERE o.order_number COLLATE NOCASE != ?
        AND o.shop_domain = ?
        AND o.id NOT IN (
            SELECT d.order_id FROM deliveries d
            WHERE d.delivery_status IN ('shipped', 'in_transit', 'out_for_delivery', 'delivered')
        )
        AND (
            (o.order_date IS NOT NULL AND o.order_date < datetime(? / 1000, 'unixepoch', '+9 hours'))
            OR (o.order_date IS NULL AND o.created_at < datetime(? / 1000, 'unixepoch'))
        )
        AND e.subject LIKE '%おまとめ完了%'
        ORDER BY o.id DESC
        "#,
    )
    .bind(&order_info.order_number)
    .bind(shop_domain)
    .bind(cutoff_ts)
    .fetch_all(tx.as_mut())
    .await
    .map_err(|e| format!("Failed to fetch phantom omatome orders: {e}"))?;

    if omatome_order_ids.is_empty() {
        return Ok(());
    }

    let mut orders_to_delete: std::collections::HashSet<i64> = std::collections::HashSet::new();

    for item in &order_info.items {
        let item_name = item.name.trim();
        if item_name.is_empty() {
            continue;
        }

        for &order_id in &omatome_order_ids {
            let result =
                sqlx::query("DELETE FROM items WHERE order_id = ? AND TRIM(item_name) = ?")
                    .bind(order_id)
                    .bind(item_name)
                    .execute(tx.as_mut())
                    .await
                    .map_err(|e| format!("Failed to delete phantom omatome item: {e}"))?;

            if result.rows_affected() > 0 {
                log::info!(
                    "[premium_bandai_omatome] phantom cleanup: deleted {} item(s) {:?} from order {}",
                    result.rows_affected(),
                    item_name,
                    order_id
                );
                orders_to_delete.insert(order_id);
            }
        }
    }

    // 空になった注文の deliveries を削除（orders と order_emails は保持）
    for order_id in orders_to_delete {
        let (remaining,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE order_id = ?")
            .bind(order_id)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| format!("Failed to count items: {e}"))?;
        if remaining == 0 {
            sqlx::query("DELETE FROM deliveries WHERE order_id = ?")
                .bind(order_id)
                .execute(tx.as_mut())
                .await
                .map_err(|e| format!("Failed to delete deliveries for phantom order: {e}"))?;
            log::info!(
                "[premium_bandai_omatome] phantom cleanup: cleaned up empty order {} (order and order_emails retained)",
                order_id
            );
        }
    }

    Ok(())
}

inventory::submit!(PluginRegistration {
    factory: || Box::new(PremiumBandaiPlugin),
});
