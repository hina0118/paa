# Issue #36 対応予定

## 目的

`lib.rs`内のTauriコマンドに直接記述されているビジネスロジックとDB操作を、テスト可能な形に分離します。

## 現状分析

### 直接SQLが存在する箇所

#### `lib.rs`内のTauriコマンド

1. **Gmail同期関連**
   - `start_sync` (59行目): エラー時のsync_metadata更新
   - `get_sync_status` (82行目): sync_metadata取得
   - `reset_sync_status` (104行目): ステータスリセット
   - `reset_sync_date` (120行目): oldest_fetched_dateリセット
   - `update_batch_size` (139行目): batch_size更新
   - `update_max_iterations` (159行目): max_iterations更新

2. **パース関連**
   - `start_batch_parse` (908行目): batch_size取得
   - `start_batch_parse` (939行目): エラー時のparse_metadata更新
   - `get_parse_status` (962行目): parse_metadata取得
   - `update_parse_batch_size` (986行目): batch_size更新
   - `parse_and_save_email` (837行目): shop_settings取得

3. **ウィンドウ設定関連**
   - `get_window_settings` (180行目): window_settings取得
   - `save_window_settings` (218行目): window_settings保存
   - `setup`内 (648行目): ウィンドウ設定復元

4. **その他**
   - `get_email_stats` (268行目): メール統計取得

#### `parsers/mod.rs`内

1. **`save_order_to_db`関数** (178-381行目)
   - orders, items, deliveries, order_emailsテーブルへの直接SQL

2. **`batch_parse_emails`関数** (384-718行目)
   - parse_metadata更新 (398行目, 485行目, 693行目)
   - テーブルクリア (411-426行目)
   - shop_settings取得 (432行目)
   - メール取得 (497行目)

## 対応計画

### Phase 1: リポジトリ追加

#### 1.1 SyncMetadataRepository追加

**ファイル**: `src-tauri/src/repository.rs`

**メソッド**:

- `get_sync_metadata() -> Result<SyncMetadata, String>`
- `update_batch_size(batch_size: i64) -> Result<(), String>`
- `update_max_iterations(max_iterations: i64) -> Result<(), String>`
- `reset_sync_status() -> Result<(), String>` (syncing -> idle)
- `reset_sync_date() -> Result<(), String>` (oldest_fetched_date = NULL)
- `update_error_status(error_message: &str) -> Result<(), String>` (エラー時の更新)

**実装クラス**: `SqliteSyncMetadataRepository`

**テスト**: 各メソッドのユニットテストを追加

#### 1.2 ParseMetadataRepository追加

**ファイル**: `src-tauri/src/repository.rs`

**メソッド**:

- `get_parse_metadata() -> Result<ParseMetadata, String>`
- `get_batch_size() -> Result<i64, String>`
- `update_batch_size(batch_size: i64) -> Result<(), String>`
- `update_parse_status(status: &str, started_at: Option<String>, completed_at: Option<String>, total_parsed: Option<i64>, error_message: Option<&str>) -> Result<(), String>`
- `reset_parse_status() -> Result<(), String>` (idleに戻す)

**実装クラス**: `SqliteParseMetadataRepository`

**テスト**: 各メソッドのユニットテストを追加

#### 1.3 WindowSettingsRepository追加

**ファイル**: `src-tauri/src/repository.rs`

**メソッド**:

- `get_window_settings() -> Result<WindowSettings, String>`
- `save_window_settings(settings: WindowSettings) -> Result<(), String>`

**型定義**: `WindowSettings`構造体を`lib.rs`から移動

**実装クラス**: `SqliteWindowSettingsRepository`

**テスト**: 各メソッドのユニットテストを追加

#### 1.4 EmailStatsRepository追加

**ファイル**: `src-tauri/src/repository.rs`

**メソッド**:

- `get_email_stats() -> Result<EmailStats, String>`

**実装クラス**: `SqliteEmailStatsRepository`

**テスト**: 各メソッドのユニットテストを追加

#### 1.5 OrderRepository追加

**ファイル**: `src-tauri/src/repository.rs`

**メソッド**:

- `save_order(order_info: &OrderInfo, email_id: Option<i64>, shop_domain: Option<&str>) -> Result<i64, String>`
  - `save_order_to_db`の内容を移行

**実装クラス**: `SqliteOrderRepository`

**テスト**: 各メソッドのユニットテストを追加

#### 1.6 ParseRepository追加（オプション）

**ファイル**: `src-tauri/src/repository.rs`

**メソッド**:

- `get_unparsed_emails(batch_size: usize) -> Result<Vec<EmailRow>, String>`
- `clear_order_tables() -> Result<(), String>` (order_emails, deliveries, items, ordersをクリア)
- `get_total_email_count() -> Result<i64, String>`

**実装クラス**: `SqliteParseRepository`

**テスト**: 各メソッドのユニットテストを追加

### Phase 2: Tauriコマンドのリファクタリング

#### 2.1 Gmail同期関連コマンドの更新

**対象**: `lib.rs`内の以下のコマンド

- `start_sync`: SyncMetadataRepositoryを使用してエラー更新
- `get_sync_status`: SyncMetadataRepositoryを使用
- `reset_sync_status`: SyncMetadataRepositoryを使用
- `reset_sync_date`: SyncMetadataRepositoryを使用
- `update_batch_size`: SyncMetadataRepositoryを使用
- `update_max_iterations`: SyncMetadataRepositoryを使用

**変更内容**:

- 直接SQLを削除
- リポジトリをインスタンス化して使用
- エラーハンドリングはリポジトリ経由

#### 2.2 パース関連コマンドの更新

**対象**: `lib.rs`内の以下のコマンド

- `start_batch_parse`: ParseMetadataRepositoryとParseRepositoryを使用
- `get_parse_status`: ParseMetadataRepositoryを使用
- `update_parse_batch_size`: ParseMetadataRepositoryを使用
- `parse_and_save_email`: OrderRepositoryとShopSettingsRepositoryを使用

**変更内容**:

- 直接SQLを削除
- リポジトリをインスタンス化して使用
- `ShopSettingsRepository`は既に存在するため、それを使用

#### 2.3 ウィンドウ設定関連コマンドの更新

**対象**: `lib.rs`内の以下のコマンド

- `get_window_settings`: WindowSettingsRepositoryを使用
- `save_window_settings`: WindowSettingsRepositoryを使用
- `setup`内のウィンドウ設定復元: WindowSettingsRepositoryを使用

**変更内容**:

- 直接SQLを削除
- リポジトリをインスタンス化して使用
- `WindowSettings`構造体を適切な場所に移動

#### 2.4 その他コマンドの更新

**対象**: `lib.rs`内の以下のコマンド

- `get_email_stats`: EmailStatsRepositoryを使用

**変更内容**:

- 直接SQLを削除
- リポジトリをインスタンス化して使用

### Phase 3: parsers/mod.rsのリファクタリング

#### 3.1 `save_order_to_db`関数の移行

**変更内容**:

- `save_order_to_db`関数を`OrderRepository::save_order`に移行
- `parsers/mod.rs`からは削除し、`repository.rs`に実装
- `parsers/mod.rs`内の呼び出しをリポジトリ経由に変更

#### 3.2 `batch_parse_emails`関数の更新

**変更内容**:

- parse_metadata更新を`ParseMetadataRepository`経由に変更
- テーブルクリアを`ParseRepository::clear_order_tables`経由に変更
- shop_settings取得は既存の`ShopSettingsRepository`を使用
- メール取得を`ParseRepository::get_unparsed_emails`経由に変更

### Phase 4: テスト追加

#### 4.1 リポジトリのユニットテスト

各リポジトリに対して以下を追加:

- 正常系テスト
- エラーケーステスト
- 境界値テスト

#### 4.2 統合テストの確認

既存の統合テストが全て通ることを確認

### Phase 5: 検証

#### 5.1 コードレビュー

- `lib.rs`と`parsers/mod.rs`から直接SQLがなくなっていることを確認
- 全てのリポジトリにユニットテストがあることを確認

#### 5.2 テスト実行

```bash
cd src-tauri && cargo test
```

#### 5.3 静的解析

```bash
cd src-tauri && cargo clippy
npm run lint
```

## 実装順序

1. **Phase 1**: リポジトリ追加（テスト含む）
2. **Phase 2**: Tauriコマンドのリファクタリング
3. **Phase 3**: parsers/mod.rsのリファクタリング
4. **Phase 4**: テスト追加・確認
5. **Phase 5**: 検証

## 完了条件

- [x] `lib.rs`と`parsers/mod.rs`から直接SQLがなくなる ✅
- [x] 全リポジトリにユニットテストがある ✅
- [x] 既存のテストが全て通る ✅ (214件すべて成功)
- [x] `cargo clippy`で警告がない ✅
- [x] `npm run lint`で警告がない ⚠️ (実行ポリシーの問題でスキップ、cargo clippyは成功)

## 注意事項

1. **既存のリポジトリパターンに従う**
   - `EmailRepository`と`ShopSettingsRepository`の実装パターンを参考にする
   - `async_trait`と`mockall`を使用

2. **型の移動**
   - `WindowSettings`構造体を適切な場所に移動
   - `EmailStats`構造体も必要に応じて移動

3. **エラーハンドリング**
   - 既存のエラーメッセージ形式を維持
   - ユーザー向けメッセージは日本語のまま

4. **後方互換性**
   - Tauriコマンドのシグネチャは変更しない
   - フロントエンドへの影響を最小限に

5. **テスト容易性**
   - リポジトリは`mockall`でモック可能にする
   - ビジネスロジックとDB操作を分離

## 見積もり

- **Phase 1**: 約2-3時間（リポジトリ実装とテスト）
- **Phase 2**: 約1-2時間（Tauriコマンド更新）
- **Phase 3**: 約1時間（parsers/mod.rs更新）
- **Phase 4**: 約1時間（テスト追加・確認）
- **Phase 5**: 約30分（検証）

**合計**: 約5-7時間
