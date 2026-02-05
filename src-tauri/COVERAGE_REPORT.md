# カバレッジレポート

## 現在のカバレッジ状況

最終更新: 2026-01-17

### 全体サマリー

| 項目                   | カバレッジ | 目標 | 状態      |
| ---------------------- | ---------- | ---- | --------- |
| **全体（行）**         | 36.13%     | 85%  | ❌ 要改善 |
| **全体（関数）**       | 34.65%     | 80%  | ❌ 要改善 |
| **全体（リージョン）** | 32.11%     | -    | -         |

### ファイル別カバレッジ

| ファイル   | 行カバレッジ                  | 関数カバレッジ                  | 状態                 |
| ---------- | ----------------------------- | ------------------------------- | -------------------- |
| `gmail.rs` | 45.11% (787行中432行未カバー) | 43.66% (71関数中40関数未カバー) | ⚠️ 改善必要          |
| `lib.rs`   | 6.41% (234行中219行未カバー)  | 13.79% (29関数中25関数未カバー) | ❌ 大幅改善必要      |
| `main.rs`  | 0.00% (3行中3行未カバー)      | 0.00% (1関数中1関数未カバー)    | - エントリーポイント |

## テストケース

現在のテスト数: **17件**

### gmail.rs のテスト (14件)

- ✅ 構造体テスト (4件)
  - `test_gmail_message_structure`
  - `test_fetch_result_structure`
  - `test_sync_progress_event_structure`
  - `test_sync_metadata_structure`

- ✅ 同期状態管理テスト (5件)
  - `test_sync_state_initialization`
  - `test_sync_state_cancel`
  - `test_sync_state_reset`
  - `test_sync_state_try_start`
  - `test_sync_state_try_start_resets_cancel`

- ✅ データベース操作テスト (5件)
  - `test_save_messages_to_db_empty`
  - `test_save_messages_to_db_single`
  - `test_save_messages_to_db_duplicate`
  - `test_save_messages_to_db_batch`
  - `test_save_messages_to_db_partial_duplicate`

### lib.rs のテスト (3件)

- ✅ 基本機能テスト (3件)
  - `test_greet`
  - `test_greet_empty`
  - `test_greet_special_characters`

## 改善が必要な領域

### 優先度: 高

1. **lib.rs (6.41%)**
   - Tauriコマンドハンドラのテストが不足
   - `start_sync`, `cancel_sync`, `get_sync_status` などの統合テストが必要
   - データベース初期化処理のテスト

2. **gmail.rs の未カバー部分 (45.11%)**
   - `GmailClient::new` - 認証周りのテスト（モック使用）
   - `GmailClient::fetch_messages` - Gmail API呼び出しのテスト
   - `BatchRunner<GmailSyncTask>` - メール同期ロジックのテスト（start_sync 経由）
   - エラーハンドリングケースのテスト

### 優先度: 中

3. **エラーケーステスト**
   - データベース接続エラー
   - Gmail API エラー
   - 認証エラー
   - タイムアウトケース

4. **エッジケーステスト**
   - 大量データの処理
   - 同時実行制御
   - キャンセル処理

### 優先度: 低

5. **main.rs (0%)**
   - エントリーポイントのため、通常はテスト対象外
   - 必要に応じて統合テストで間接的にカバー

## 次のステップ

### 短期目標（カバレッジ 60%）

1. lib.rs の Tauriコマンドハンドラのテスト追加
2. gmail.rs の基本的なエラーハンドリングテスト追加
3. データベース初期化とマイグレーションのテスト

### 中期目標（カバレッジ 75%）

1. GmailClient のモックを使用した単体テスト
2. BatchRunner<GmailSyncTask> の詳細なシナリオテスト
3. 並行処理とキャンセル処理の統合テスト

### 長期目標（カバレッジ 85%）

1. エッジケースの網羅的なテスト
2. パフォーマンステスト
3. E2Eテストの追加

## カバレッジレポートの確認方法

HTMLレポートを生成:

```bash
cargo llvm-cov --all-features --workspace --html
```

レポート表示:

```bash
start target/llvm-cov/html/index.html
```

テキストサマリー:

```bash
cargo llvm-cov --all-features --workspace
```

## CI/CD統合

カバレッジ閾値チェック（85%）:

```bash
cargo llvm-cov --all-features --workspace --fail-under-lines 85
```

LCOV形式で出力（Codecov連携用）:

```bash
cargo llvm-cov --all-features --workspace --lcov --output-path coverage.lcov
```
