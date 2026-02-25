# プラグインシステム — 新規店舗の追加手順

## 概要

メール解析プラグインは `src-tauri/src/plugins/` に集約されており、
新規店舗の追加は **`registry.rs` の 1 箇所** と **新しいプラグインファイルの作成** のみで完結する。
`shop_settings` テーブルへのデフォルト設定投入やトランザクション管理はフレームワーク側が自動で行う。

## ファイル構成

```
src-tauri/src/plugins/
├── mod.rs          VendorPlugin トレイト定義・ensure_default_settings()
├── registry.rs     build_registry() — 全プラグインをここで登録
├── dmm.rs          DmmPlugin 実装例
└── hobbysearch.rs  HobbySearchPlugin 実装例
```

## 新規店舗を追加する手順

### Step 1 — プラグインファイルを作成する

`src-tauri/src/plugins/<shop>.rs` を新規作成し、`VendorPlugin` トレイトを実装する。

```rust
// src-tauri/src/plugins/myshop.rs

use std::{path::PathBuf, sync::Arc};
use crate::repository::SqliteOrderRepository;
use super::{DefaultShopSetting, DispatchError, DispatchOutcome, VendorPlugin};

pub struct MyShopPlugin;

#[async_trait::async_trait]
impl VendorPlugin for MyShopPlugin {
    // --------------------------------------------------------
    // メタデータ
    // --------------------------------------------------------

    /// shop_settings.shop_name に対応する表示名
    fn shop_name(&self) -> &str {
        "マイショップ"
    }

    /// このプラグインが処理できる parser_type の一覧
    fn parser_types(&self) -> &[&str] {
        &["myshop_confirm", "myshop_cancel", "myshop_send"]
    }

    /// 複数プラグインが同一 parser_type を持つ場合に使用する優先度（高いほど優先）
    fn priority(&self) -> i32 {
        10
    }

    // --------------------------------------------------------
    // デフォルト shop_settings 行
    // --------------------------------------------------------

    /// アプリ起動時に ensure_default_settings() が INSERT OR IGNORE で投入する設定一覧。
    /// sender_address + parser_type の組み合わせが UNIQUE キーなので重複は自動で無視される。
    fn default_shop_settings(&self) -> Vec<DefaultShopSetting> {
        vec![
            DefaultShopSetting {
                shop_name: "マイショップ".to_string(),
                sender_address: "order@myshop.example.com".to_string(),
                parser_type: "myshop_confirm".to_string(),
                subject_filters: Some(vec!["【マイショップ】注文確認".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "マイショップ".to_string(),
                sender_address: "order@myshop.example.com".to_string(),
                parser_type: "myshop_cancel".to_string(),
                subject_filters: Some(vec!["【マイショップ】キャンセル完了".to_string()]),
            },
            DefaultShopSetting {
                shop_name: "マイショップ".to_string(),
                sender_address: "order@myshop.example.com".to_string(),
                parser_type: "myshop_send".to_string(),
                subject_filters: Some(vec!["【マイショップ】発送完了".to_string()]),
            },
        ]
    }

    // --------------------------------------------------------
    // メール解析・保存
    // --------------------------------------------------------

    /// メール本文を解析し、DB への保存までをトランザクション内で完結させる。
    /// `tx` は呼び出し元 (EmailParseTask) が begin/commit/rollback を管理するため、
    /// このメソッド内では commit/rollback を呼んではならない。
    async fn dispatch(
        &self,
        parser_type: &str,
        email_id: i64,
        from_address: Option<&str>,
        shop_name: &str,
        internal_date: Option<i64>,
        body: &str,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        _image_save_ctx: &Option<(Arc<sqlx::SqlitePool>, std::path::PathBuf)>,
    ) -> Result<DispatchOutcome, DispatchError> {
        match parser_type {
            "myshop_confirm" => {
                // 本文を解析して OrderInfo を生成し、SqliteOrderRepository の _in_tx ヘルパーで保存
                // 解析失敗 → Err(DispatchError::ParseFailed("...".to_string()))
                // 保存失敗 → Err(DispatchError::SaveFailed("...".to_string()))
                todo!("implement myshop_confirm parser")
            }
            _ => Err(DispatchError::ParseFailed(format!(
                "unsupported parser_type: {parser_type}"
            ))),
        }
    }
}
```

> **注意**: `dispatch()` 内では `SqliteOrderRepository::save_order_in_tx(tx, ...)` など
> `_in_tx` サフィックスの静的メソッドを使用する。`tx.commit()` / `tx.rollback()` は
> `EmailParseTask::process_batch()` が管理するため、プラグイン内では呼ばない。

### Step 2 — registry.rs にエントリを追加する

```rust
// src-tauri/src/plugins/registry.rs

use super::dmm::DmmPlugin;
use super::hobbysearch::HobbySearchPlugin;
use super::myshop::MyShopPlugin;   // ← 追加
use super::VendorPlugin;

pub fn build_registry() -> Vec<Box<dyn VendorPlugin>> {
    vec![
        Box::new(DmmPlugin),
        Box::new(HobbySearchPlugin),
        Box::new(MyShopPlugin),   // ← 追加
    ]
}
```

### Step 3 — mod.rs でサブモジュールを宣言する

```rust
// src-tauri/src/plugins/mod.rs（既存の pub mod 宣言の近くに追加）

pub mod myshop;   // ← 追加
```

### Step 4 — ビルド・テストを実行する

```bash
cd src-tauri
cargo check
cargo test
```

`ensure_default_settings()` は起動時に自動実行されるため、
`001_init.sql` の変更は **不要**。

---

## 仕組みの概要

### デフォルト設定の自動投入

`lib.rs` の起動シーケンス内で以下が呼ばれる:

```
build_registry()
  → 各プラグインの default_shop_settings()
  → ShopSettingsRepository::insert_if_not_exists()  (INSERT OR IGNORE)
```

- 既存行は `INSERT OR IGNORE` により無視されるため冪等
- 既存ユーザーが手動変更した `is_enabled` / `subject_filters` は上書きされない
  (`INSERT OR IGNORE` は行が存在する場合何もしない)

### トランザクション管理

`EmailParseTask::process_batch()` が各メールの処理ごとに:

1. `pool.begin()` でトランザクションを開始
2. `plugin.dispatch(..., &mut tx, ...)` を呼び出す
3. `Ok` → `tx.commit()`
4. `Err(ParseFailed)` / `Err(SaveFailed)` → `tx` を drop（自動ロールバック）

プラグインは `tx` を受け取り、DB 操作を全て同一トランザクション内で行う。

### _in_tx ヘルパー

`SqliteOrderRepository` には `pub(crate)` な静的メソッドが用意されている:

| ヘルパー | 用途 |
|---|---|
| `save_order_in_tx(tx, ...)` | 注文の新規保存・更新 |
| `apply_cancel_in_tx(tx, ...)` | キャンセル処理 |
| `apply_order_number_change_in_tx(tx, ...)` | 注文番号変更 |
| `apply_consolidation_in_tx(tx, ...)` | 注文まとめ |
| `apply_split_first_order_in_tx(tx, ...)` | 注文分割 |
| `apply_send_and_replace_items_in_tx(tx, ...)` | 発送・商品差替え |
| `apply_change_items_in_tx(tx, ...)` | 商品変更 |

詳細なシグネチャは `src-tauri/src/repository/order.rs` を参照。
