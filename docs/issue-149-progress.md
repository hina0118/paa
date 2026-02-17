# Issue #149: lib.rsの巨大コマンドハンドラーをモジュール分割

## 概要

`src-tauri/src/lib.rs`（2,160行、50+コマンド）を `commands/` モジュールに分割し、lib.rsを薄いオーケストレーターに変換する。

## 対応状況

| Step | モジュール                  | 状態 | 備考                                                     |
| ---- | --------------------------- | ---- | -------------------------------------------------------- |
| 0    | ベースライン確認            | done | cargo test 全通過                                        |
| 1    | ディレクトリ作成 + mod.rs   | done | `src-tauri/src/commands/mod.rs`                          |
| 2a   | `commands/log.rs`           | done | LogEntry, LOG_BUFFER, init/add/get_logs + テスト8件      |
| 2b   | `commands/stats.rs`         | done | greet, seed_e2e_db, get_db_filename, stats系 + テスト4件 |
| 2c   | `commands/window.rs`        | done | get/save_window_settings + validate + テスト5件          |
| 2d   | `commands/shop_settings.rs` | done | CRUD 4コマンド                                           |
| 2e   | `commands/metadata.rs`      | done | export/import/restore                                    |
| 2f   | `commands/overrides.rs`     | done | 12コマンド                                               |
| 2g   | `commands/api_keys.rs`      | done | Gemini/Gmail/SerpApi 9コマンド                           |
| 2h   | `commands/image_search.rs`  | done | search_product_images, save_image_from_url               |
| 2i   | `commands/config.rs`        | done | Gemini設定 3コマンド + テスト2件                         |
| 2j   | `commands/sync.rs`          | done | Gmail同期 10コマンド + validators + テスト5件            |
| 2k   | `commands/parse.rs`         | done | メール解析 6コマンド + テスト8件                         |
| 2l   | `commands/product_parse.rs` | done | 商品名解析 1コマンド + state + deprecated struct         |
| 3    | lib.rs整理 + 最終テスト     | done | cargo test + cargo clippy 全通過                         |

## 結果

### 行数の変化

| ファイル | Before  | After            |
| -------- | ------- | ---------------- |
| `lib.rs` | 2,160行 | 558行 (**-74%**) |

### 新規作成ファイル

| ファイル                    | 行数      | コマンド数     |
| --------------------------- | --------- | -------------- |
| `commands/mod.rs`           | 25        | - (re-exports) |
| `commands/log.rs`           | 194       | 1 + helpers    |
| `commands/stats.rs`         | 114       | 8              |
| `commands/window.rs`        | 98        | 2              |
| `commands/shop_settings.rs` | 52        | 4              |
| `commands/metadata.rs`      | 32        | 3              |
| `commands/overrides.rs`     | 157       | 12             |
| `commands/api_keys.rs`      | 138       | 9              |
| `commands/image_search.rs`  | 79        | 2              |
| `commands/config.rs`        | 84        | 3              |
| `commands/sync.rs`          | 243       | 10             |
| `commands/parse.rs`         | 373       | 6              |
| `commands/product_parse.rs` | 86        | 1              |
| **合計**                    | **1,675** | **61**         |

### 追加変更

- `batch_commands.rs`: `crate::ProductNameParseState` → `crate::commands::ProductNameParseState` に3箇所修正

### テスト結果

- `cargo test`: 全テスト通過（lib.rs内テスト + 各モジュール内テスト）
- `cargo clippy`: 警告なし

## 詳細ログ

### Step 0: ベースライン確認

- `cargo test` 全通過確認済み

### Step 1: ディレクトリ + mod.rs 作成

- `src-tauri/src/commands/` ディレクトリ作成
- `mod.rs` に12モジュールの宣言とre-exportを記述

### Step 2a-2l: 全モジュール作成

- lib.rsから各責務ごとにコマンドハンドラーを抽出
- テストも対応するモジュールに移動
- バリデーション関数はそれを使用するコマンドと同じモジュールに配置

### Step 3: lib.rs整理 + 最終テスト

- lib.rsからコマンドハンドラー・ヘルパー・テストを全て削除
- `pub mod commands;` を追加
- `run()` 内の参照を `commands::` プレフィックスに更新
- `invoke_handler` のコマンド登録を `commands::` パスに変更
- `batch_commands.rs` の `ProductNameParseState` 参照パスを修正
- lib.rsに残したもの: モジュール宣言、`is_sqlite_version_supported()`、`run()`、image_utilsテスト
- `cargo test` 全通過、`cargo clippy` 警告なし
