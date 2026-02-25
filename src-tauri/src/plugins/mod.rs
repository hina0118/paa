//! VendorPlugin トレイト定義
//!
//! 店舗ごとのメールパース処理をプラグインとして抽象化するトレイト。
//! 新しい店舗を追加する場合は `VendorPlugin` を実装し、`registry.rs` に登録するだけでよい。
//!
//! # 設計方針
//! - `dispatch()` がパース + 保存を一括処理し、呼び出し元（`email_parse_task.rs`）をシンプルに保つ
//! - `DispatchError::ParseFailed` は「次のパーサーを試す」、`DispatchError::SaveFailed` は「このメールをリトライ」
//! - `alternate_domains()` はプラグイン側で管理（DMM の mail/mono 二重チェック等）

pub mod registry;

mod dmm;
mod hobbysearch;

pub use registry::{build_registry, find_plugin};

use async_trait::async_trait;
use chrono::DateTime;
use std::path::PathBuf;
use std::sync::Arc;

use crate::parsers::{EmailParser, OrderInfo};
use crate::repository::OrderRepository;

// ─────────────────────────────────────────────────────────────────────────────
// DispatchOutcome / DispatchError
// ─────────────────────────────────────────────────────────────────────────────

/// ディスパッチ成功時の結果種別
///
/// `email_parse_task.rs` が `EmailParseOutput` を組み立てるために使用する。
pub enum DispatchOutcome {
    /// 通常注文を保存した（confirm / send など）
    OrderSaved(OrderInfo),
    /// キャンセルを適用した
    CancelApplied { order_number: String },
    /// 注文番号変更を適用した
    OrderNumberChanged { new_order_number: String },
    /// まとめ完了を適用した
    ConsolidationApplied { new_order_number: String },
    /// 複数注文を保存した（split_complete 等）。先頭の `OrderInfo` を代表として保持する。
    MultiOrderSaved(Vec<OrderInfo>),
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
    /// - `order_repo`: 注文リポジトリ
    /// - `image_save_ctx`: 画像保存用コンテキスト（`None` の場合は画像登録をスキップ）
    ///
    /// # エラー
    /// - `DispatchError::ParseFailed` → 呼び出し元は次のパーサーを試す
    /// - `DispatchError::SaveFailed` → 呼び出し元はこのメールをリトライ対象にする
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
}

// ─────────────────────────────────────────────────────────────────────────────
// プラグイン共通ヘルパー（pub(crate) で各プラグインから参照）
// ─────────────────────────────────────────────────────────────────────────────

/// from_address から shop_domain（ドメイン文字列）を抽出する
pub(crate) fn derive_shop_domain(from_address: Option<&str>) -> Option<String> {
    use crate::logic::email_parser::extract_domain;
    use crate::logic::sync_logic::extract_email_address;
    from_address
        .and_then(|addr| extract_email_address(addr))
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
