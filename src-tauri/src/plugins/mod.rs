//! VendorPlugin トレイト定義
//!
//! 店舗ごとのメールパース処理をプラグインとして抽象化するトレイト。
//! 新しい店舗を追加する場合は `VendorPlugin` を実装し、`inventory::submit!` で自動登録する。
//!
//! # 設計方針
//! - `dispatch()` がパース + 保存を一括処理し、呼び出し元（`email_parse_task.rs`）をシンプルに保つ
//! - `DispatchError::ParseFailed` は「次のパーサーを試す」、`DispatchError::SaveFailed` は「このメールをリトライ」
//! - `alternate_domains()` はプラグイン側で管理（DMM の mail/mono 二重チェック等）

// pub mod にすることでリンカーがモジュールを保持し、inventory::submit! の静的初期化が LTO でも除外されない
pub mod amazon;
pub mod amiami;
pub mod animate;
pub mod dmm;
pub mod furuichi_online;
pub mod goodsmile;
pub mod hobbyjapan;
pub mod hobbysearch;
pub mod kids_dragon;
pub mod premium_bandai;
pub mod sagawa;
pub mod surugaya;
pub mod surugaya_mp;
pub mod yodobashi;

// ─────────────────────────────────────────────────────────────────────────────
// inventory による自動登録
// ─────────────────────────────────────────────────────────────────────────────

/// 自動登録用ラッパー型
///
/// 各プラグインファイルの末尾で `inventory::submit!` を呼び出すことで
/// グローバルレジストリに自動登録される。
pub struct PluginRegistration {
    pub factory: fn() -> Box<dyn VendorPlugin>,
}

inventory::collect!(PluginRegistration);

/// 全登録済みプラグインを収集してレジストリを構築する
pub fn build_registry() -> Vec<Box<dyn VendorPlugin>> {
    inventory::iter::<PluginRegistration>
        .into_iter()
        .map(|r| (r.factory)())
        .collect()
}

/// `parser_type` に対応するプラグインを返す
///
/// 複数のプラグインが同一の `parser_type` に対応する場合は `priority()` が最大のものを返す。
pub fn find_plugin<'a>(
    registry: &'a [Box<dyn VendorPlugin>],
    parser_type: &str,
) -> Option<&'a dyn VendorPlugin> {
    registry
        .iter()
        .filter(|p| p.parser_types().contains(&parser_type))
        .max_by_key(|p| p.priority())
        .map(|p| p.as_ref())
}

use async_trait::async_trait;
use chrono::DateTime;
use std::path::PathBuf;
use std::sync::Arc;

use crate::parsers::{EmailParser, OrderInfo};
use crate::repository::ShopSettingsRepository;

// ─────────────────────────────────────────────────────────────────────────────
// DefaultShopSetting
// ─────────────────────────────────────────────────────────────────────────────

/// プラグインが DB に自動挿入するデフォルト shop_settings レコード
pub struct DefaultShopSetting {
    pub shop_name: String,
    pub sender_address: String,
    pub parser_type: String,
    pub subject_filters: Option<Vec<String>>,
}

// ─────────────────────────────────────────────────────────────────────────────
// ensure_default_settings
// ─────────────────────────────────────────────────────────────────────────────

/// 登録済みプラグインの `default_shop_settings()` を走査し、DB に存在しないレコードを挿入する。
///
/// 冪等（`INSERT OR IGNORE` ベース）。アプリ起動時に呼び出す。
pub async fn ensure_default_settings(
    registry: &[Box<dyn VendorPlugin>],
    repo: &dyn ShopSettingsRepository,
) -> Result<(), String> {
    for plugin in registry {
        for setting in plugin.default_shop_settings() {
            repo.insert_if_not_exists(&setting).await?;
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// DispatchOutcome / DispatchError
// ─────────────────────────────────────────────────────────────────────────────

/// ディスパッチ成功時の結果種別
///
/// `email_parse_task.rs` が `EmailParseOutput` を組み立てるために使用する。
pub enum DispatchOutcome {
    /// 通常注文を保存した（confirm / send など）
    OrderSaved(Box<OrderInfo>),
    /// キャンセルを適用した
    CancelApplied { order_number: String },
    /// 注文番号変更を適用した
    OrderNumberChanged { new_order_number: String },
    /// まとめ完了を適用した
    ConsolidationApplied { new_order_number: String },
    /// 複数注文を保存した（split_complete 等）。先頭の `OrderInfo` を代表として保持する。
    MultiOrderSaved(Vec<OrderInfo>),
    /// 配達完了メールを処理した（tracking_check_logs + deliveries を更新済み）
    DeliveryCompleted { tracking_number: String },
}

/// ディスパッチ失敗時のエラー種別
#[derive(Debug)]
pub enum DispatchError {
    /// パース失敗 → 呼び出し元は次のパーサーを試す
    ParseFailed(String),
    /// 保存 / 適用失敗 → 呼び出し元はこのメールをリトライ対象にする
    SaveFailed(String),
}

impl DispatchError {
    pub fn message(&self) -> &str {
        match self {
            Self::ParseFailed(e) | Self::SaveFailed(e) => e,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// VendorPlugin トレイト
// ─────────────────────────────────────────────────────────────────────────────

/// 店舗ごとのメールパース処理を抽象化するトレイト
///
/// # 優先度
/// 複数のプラグインが同一の `parser_type` に対応する場合、`priority()` の値が大きい方が選ばれる。
/// デフォルト 0（汎用）、店舗専用プラグインは 10 以上を推奨。
#[async_trait]
pub trait VendorPlugin: Send + Sync {
    /// このプラグインが対応する parser_type 一覧
    fn parser_types(&self) -> &[&str];

    /// parser_type を指定して EmailParser を取得
    ///
    /// キャンセル・注文番号変更・まとめ完了など、`OrderInfo` を返さない特殊パーサーでは `None` を返す。
    fn get_parser(&self, parser_type: &str) -> Option<Box<dyn EmailParser>>;

    /// メール1通のパース + 保存を一括処理
    ///
    /// # 引数
    /// - `parser_type`: 処理するパーサー種別
    /// - `email_id`: メール ID（DB 登録用）
    /// - `from_address`: 送信元アドレス（`"Name <email>"` 形式も可）
    /// - `shop_name`: ショップ名
    /// - `internal_date`: メール受信日時（Unix ミリ秒）
    /// - `body`: メール本文
    /// - `tx`: 外部から渡されたトランザクション（コミットは呼び出し元で行う）
    ///
    /// # エラー
    /// - `DispatchError::ParseFailed` → 呼び出し元は次のパーサーを試す
    /// - `DispatchError::SaveFailed` → 呼び出し元はこのメールをリトライ対象にする
    ///
    /// # 画像登録
    /// 画像登録は呼び出し元（`email_parse_task.rs`）が `tx.commit()` 後に行う。
    /// `dispatch()` 内では行わない（`tx` の RESERVED LOCK と画像 INSERT が競合して
    /// SQLITE_BUSY になるため）。
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
    ) -> Result<DispatchOutcome, DispatchError>;

    /// 別ドメイン検索（DMM の mail/mono 二重チェック等）
    fn alternate_domains(&self, domain: &str) -> Option<Vec<String>> {
        let _ = domain;
        None
    }

    /// プラグインの優先度（デフォルト 0）
    fn priority(&self) -> i32 {
        0
    }

    /// 店舗の表示名
    fn shop_name(&self) -> &str;

    /// このプラグインが必要とするデフォルト shop_settings レコード一覧
    ///
    /// DB に同レコードが存在しない場合に自動挿入される（`ensure_default_settings` で利用）。
    fn default_shop_settings(&self) -> Vec<DefaultShopSetting>;

    /// `true` を返すプラグインには `body_html` ではなく `body_plain` が渡される。
    ///
    /// デフォルトは `false`（HTML 優先）。
    /// Amazon 等のプレーンテキストパーサーは `true` をオーバーライドする。
    fn prefer_plain_text(&self) -> bool {
        false
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// プラグイン共通定数
// ─────────────────────────────────────────────────────────────────────────────

/// 日本郵便追跡サービスの URL - 入力フォーム（ゆうパック・ゆうパケット共通）
pub(crate) const JAPANPOST_TRACKING_URL: &str =
    "https://trackings.post.japanpost.jp/services/srv/search/input";

/// 日本郵便追跡サービスの URL - 直接検索（追跡番号をクエリパラメータで渡す形式）
pub(crate) const JAPANPOST_TRACKING_URL_DIRECT: &str =
    "https://trackings.post.japanpost.jp/services/srv/search/direct";

// ─────────────────────────────────────────────────────────────────────────────
// プラグイン共通ヘルパー（pub(crate) で各プラグインから参照）
// ─────────────────────────────────────────────────────────────────────────────

/// from_address から shop_domain（ドメイン文字列）を抽出する
pub(crate) fn derive_shop_domain(from_address: Option<&str>) -> Option<String> {
    use crate::logic::email_parser::extract_domain;
    use crate::logic::sync_logic::extract_email_address;
    from_address
        .and_then(extract_email_address)
        .and_then(|email| extract_domain(&email).map(|s| s.to_string()))
}

/// `order_date` が未設定の場合に `internal_date` から補完する
///
/// ホビーサーチ confirm / change 系と DMM confirm は受信日時を注文日として使用する。
pub(crate) fn apply_internal_date(order_info: &mut OrderInfo, internal_date: Option<i64>) {
    if order_info.order_date.is_some() {
        return;
    }
    if let Some(ts_ms) = internal_date {
        let dt = match DateTime::from_timestamp_millis(ts_ms) {
            Some(d) => d,
            None => {
                log::warn!(
                    "[plugins] Failed to parse internal_date {} (invalid timestamp), using current time as order_date fallback",
                    ts_ms
                );
                chrono::Utc::now()
            }
        };
        order_info.order_date = Some(dt.format("%Y-%m-%d %H:%M:%S").to_string());
    }
}

/// `OrderInfo` に含まれる商品画像 URL を `images` テーブルに登録する
///
/// `image_save_ctx` が `None` の場合は何もしない。
pub(crate) async fn save_images_for_order(
    order_info: &OrderInfo,
    image_save_ctx: &Option<(Arc<sqlx::SqlitePool>, PathBuf)>,
) {
    let Some((ref pool, ref images_dir)) = image_save_ctx else {
        return;
    };
    for item in &order_info.items {
        let Some(ref url) = item.image_url else {
            continue;
        };
        let normalized = crate::gemini::normalize_product_name(&item.name);
        if normalized.is_empty() {
            continue;
        }
        if let Err(e) = crate::image_utils::save_image_from_url_for_item(
            pool.as_ref(),
            images_dir,
            &normalized,
            url,
            true, // パース: 既存レコードがあればスキップ
        )
        .await
        {
            log::warn!(
                "[plugins] Failed to save image for item '{}': {}",
                item.name,
                e
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_registry_is_not_empty() {
        let registry = build_registry();
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_find_plugin_dmm_confirm() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "dmm_confirm");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_dmm_cancel() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "dmm_cancel");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_dmm_send() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "dmm_send");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_dmm_split_complete() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "dmm_split_complete");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_dmm_order_number_change() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "dmm_order_number_change");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_dmm_merge_complete() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "dmm_merge_complete");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_hobbysearch_confirm() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "hobbysearch_confirm");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_hobbysearch_cancel() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "hobbysearch_cancel");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_amazon_confirm() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "amazon_confirm");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_yodobashi_confirm() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "yodobashi_confirm");
        assert!(plugin.is_some());
    }

    #[test]
    fn test_find_plugin_unknown_returns_none() {
        let registry = build_registry();
        let plugin = find_plugin(&registry, "unknown_parser");
        assert!(plugin.is_none());
    }

    #[test]
    fn test_find_plugin_priority_resolution() {
        // 同一 parser_type に複数プラグインが対応する場合、priority 最大が選ばれること
        // 現在の実装では DmmPlugin priority=10、HobbySearchPlugin priority=10 で重複なし
        let registry = build_registry();
        // DmmPlugin のみが対応する型では DmmPlugin が返る
        let plugin = find_plugin(&registry, "dmm_merge_complete");
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().priority(), 10);
    }

    #[test]
    fn test_all_dmm_parser_types_have_plugin() {
        let registry = build_registry();
        let dmm_types = [
            "dmm_confirm",
            "dmm_send",
            "dmm_cancel",
            "dmm_order_number_change",
            "dmm_split_complete",
            "dmm_merge_complete",
        ];
        for pt in &dmm_types {
            assert!(find_plugin(&registry, pt).is_some(), "No plugin for {}", pt);
        }
    }

    #[test]
    fn test_all_hobbysearch_parser_types_have_plugin() {
        let registry = build_registry();
        let hs_types = [
            "hobbysearch_confirm",
            "hobbysearch_confirm_yoyaku",
            "hobbysearch_change",
            "hobbysearch_change_yoyaku",
            "hobbysearch_send",
            "hobbysearch_cancel",
        ];
        for pt in &hs_types {
            assert!(find_plugin(&registry, pt).is_some(), "No plugin for {}", pt);
        }
    }

    #[test]
    fn test_all_premium_bandai_parser_types_have_plugin() {
        let registry = build_registry();
        let pb_types = [
            "premium_bandai_confirm",
            "premium_bandai_omatome",
            "premium_bandai_send",
        ];
        for pt in &pb_types {
            assert!(find_plugin(&registry, pt).is_some(), "No plugin for {}", pt);
        }
    }

    #[test]
    fn test_all_animate_parser_types_have_plugin() {
        let registry = build_registry();
        let animate_types = ["animate_confirm", "animate_send"];
        for pt in &animate_types {
            assert!(find_plugin(&registry, pt).is_some(), "No plugin for {}", pt);
        }
    }

    #[test]
    fn test_all_furuichi_parser_types_have_plugin() {
        let registry = build_registry();
        let furuichi_types = ["furuichi_confirm", "furuichi_send"];
        for pt in &furuichi_types {
            assert!(find_plugin(&registry, pt).is_some(), "No plugin for {}", pt);
        }
    }

    #[test]
    fn test_all_amiami_parser_types_have_plugin() {
        let registry = build_registry();
        let amiami_types = [
            "amiami_rakuten_confirm",
            "amiami_rakuten_send",
            "amiami_confirm",
            "amiami_send",
            "amiami_cancel",
        ];
        for pt in &amiami_types {
            assert!(find_plugin(&registry, pt).is_some(), "No plugin for {}", pt);
        }
    }

    #[test]
    fn test_all_surugaya_parser_types_have_plugin() {
        let registry = build_registry();
        let surugaya_types = ["surugaya_confirm", "surugaya_send"];
        for pt in &surugaya_types {
            assert!(find_plugin(&registry, pt).is_some(), "No plugin for {}", pt);
        }
    }

    #[test]
    fn test_all_surugaya_mp_parser_types_have_plugin() {
        let registry = build_registry();
        let surugaya_mp_types = ["surugaya_mp_confirm", "surugaya_mp_send"];
        for pt in &surugaya_mp_types {
            assert!(find_plugin(&registry, pt).is_some(), "No plugin for {}", pt);
        }
    }
}
