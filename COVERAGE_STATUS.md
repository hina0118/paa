# プロジェクト全体カバレッジ状況レポート

最終更新: 2026-01-17

## 📊 全体サマリー

### プロジェクト目標: **85%**

| 領域 | カバレッジ | 目標 | 状態 | テスト数 |
|------|-----------|------|------|---------|
| **バックエンド（Rust）** | 36.13% | 85% | ❌ 大幅改善必要 | 17件 |
| **フロントエンド（React）** | 100% | 85% | ✅ 目標達成 | 16件 |
| **全体（推定）** | ~40-45% | 85% | ❌ 改善必要 | 33件 |

---

## 🦀 バックエンド（Rust）カバレッジ詳細

### 全体: 36.13%

| 指標 | カバレッジ | 詳細 |
|------|-----------|------|
| **行カバレッジ** | 36.13% | 1024行中654行未カバー |
| **関数カバレッジ** | 34.65% | 101関数中66関数未カバー |
| **リージョンカバレッジ** | 32.11% | 1305リージョン中886未カバー |

### ファイル別詳細

#### 1. gmail.rs - **45.11%** ⚠️

| 指標 | カバレッジ | 詳細 |
|------|-----------|------|
| 行 | 45.11% | 787行中432行未カバー |
| 関数 | 43.66% | 71関数中40関数未カバー |
| リージョン | 36.90% | 1076リージョン中679未カバー |

**カバー済み:**
- ✅ GmailMessage, FetchResult, SyncProgressEvent構造体
- ✅ SyncState（同期状態管理）
- ✅ save_messages_to_db（データベース保存）

**未カバー（優先度: 高）:**
- ❌ GmailClient::new（認証処理）
- ❌ GmailClient::fetch_messages（Gmail API呼び出し）
- ❌ GmailClient::get_message（メッセージ取得）
- ❌ sync_gmail_incremental（増分同期ロジック）
- ❌ エラーハンドリング各種

#### 2. lib.rs - **6.41%** ❌

| 指標 | カバレッジ | 詳細 |
|------|-----------|------|
| 行 | 6.41% | 234行中219行未カバー |
| 関数 | 13.79% | 29関数中25関数未カバー |
| リージョン | 9.73% | 226リージョン中204未カバー |

**カバー済み:**
- ✅ greet関数（3パターン）

**未カバー（優先度: 高）:**
- ❌ start_sync（同期開始コマンド）
- ❌ cancel_sync（同期キャンセルコマンド）
- ❌ get_sync_status（同期状態取得コマンド）
- ❌ reset_sync_status（同期状態リセット）
- ❌ update_batch_size（バッチサイズ更新）
- ❌ fetch_gmail_emails（メール取得コマンド）
- ❌ run関数（アプリ初期化）

#### 3. main.rs - **0.00%**

エントリーポイントのため、通常はテスト対象外。

---

## ⚛️ フロントエンド（React）カバレッジ詳細

### 全体: 100% ✅

| 指標 | カバレッジ |
|------|-----------|
| **Statements** | 100% |
| **Branches** | 100% |
| **Functions** | 100% |
| **Lines** | 100% |

### ファイル別詳細

#### 1. components/ui/button.tsx - **100%** ✅

**テスト数: 9件**
- ✅ 基本レンダリング
- ✅ クリックイベント
- ✅ バリアント（default, destructive, outline, secondary, ghost, link）
- ✅ サイズ（sm, default, lg, icon）
- ✅ disabled状態
- ✅ カスタムクラス

#### 2. lib/utils.ts - **100%** ✅

**テスト数: 7件**
- ✅ クラス名のマージ
- ✅ 条件付きクラス
- ✅ Tailwindクラスの競合解決
- ✅ undefined/null処理
- ✅ 空入力処理
- ✅ 配列処理
- ✅ オブジェクト処理

### 未テスト（今後追加予定）

**優先度: 高**
1. 画面コンポーネント
   - Dashboard（ダッシュボード）
   - Settings（設定画面）
   - Sync（同期画面）

2. データ関連コンポーネント
   - EmailList（メール一覧）
   - DataTable（データテーブル）
   - Columns（カラム定義）

**優先度: 中**
3. Context
   - NavigationContext（ナビゲーション状態）
   - SyncContext（同期状態）

4. その他UIコンポーネント
   - Card, Checkbox, DropdownMenu, Input, Progress, Table

---

## 📈 改善計画

### 短期目標（カバレッジ 60%）

**バックエンド（Rust）**
1. lib.rs の Tauriコマンドハンドラのテスト
   - start_sync, cancel_sync, get_sync_status
   - 推定カバレッジ向上: +15%

2. gmail.rs の基本的なエラーハンドリング
   - データベースエラー、認証エラー
   - 推定カバレッジ向上: +5%

**フロントエンド（React）**
- 現在100%達成済み ✅
- 新規コンポーネントのテストを継続

### 中期目標（カバレッジ 75%）

**バックエンド（Rust）**
1. GmailClient のモックテスト
   - new, fetch_messages, get_message
   - 推定カバレッジ向上: +15%

2. sync_gmail_incremental の詳細シナリオ
   - 正常系、エラー系、エッジケース
   - 推定カバレッジ向上: +10%

### 長期目標（カバレッジ 85%）

**バックエンド（Rust）**
1. エッジケースの網羅的なテスト
2. パフォーマンステスト
3. 統合テスト（E2E）

**フロントエンド（React）**
1. 全コンポーネントのテスト
2. Context統合テスト
3. E2Eテスト

---

## 🎯 次のステップ

### 今すぐ実施すべきこと

1. **lib.rs のTauriコマンドテスト作成**
   ```rust
   // tests/command_tests.rs に追加
   - test_start_sync
   - test_cancel_sync
   - test_get_sync_status
   ```

2. **gmail.rs のエラーハンドリングテスト**
   ```rust
   // gmail.rs の tests モジュールに追加
   - test_save_messages_db_error
   - test_invalid_timestamp_handling
   ```

3. **フロントエンドの主要画面テスト**
   ```tsx
   // src/components/screens/ に追加
   - dashboard.test.tsx
   - sync.test.tsx
   - settings.test.tsx
   ```

---

## 📋 カバレッジコマンド一覧

### バックエンド（Rust）
```bash
# テキスト出力
cd src-tauri && cargo llvm-cov --all-features --workspace

# HTMLレポート生成
cd src-tauri && cargo llvm-cov --all-features --workspace --html

# 閾値チェック（85%）
cd src-tauri && cargo llvm-cov --all-features --workspace --fail-under-lines 85
```

### フロントエンド（React）
```bash
# テキスト出力
npm run test:frontend:coverage

# HTMLレポート生成（自動）
npm run test:frontend:coverage
# -> coverage/index.html
```

### 全体
```bash
# 全テスト実行
npm run test:all

# 個別実行
npm run test                    # バックエンドのみ
npm run test:frontend:run       # フロントエンドのみ
```

---

## 📊 進捗トラッキング

| 日付 | バックエンド | フロントエンド | 全体推定 | 備考 |
|------|------------|--------------|---------|------|
| 2026-01-17 | 36.13% | 100% | ~45% | 初期環境構築完了 |
| - | 目標: 60% | 維持: 100% | ~70% | 短期目標 |
| - | 目標: 75% | 維持: 100% | ~82% | 中期目標 |
| - | 目標: 85% | 維持: 100% | ~90% | 長期目標達成 |

---

## 🔗 関連ドキュメント

- [バックエンドテストガイド](src-tauri/TESTING.md)
- [フロントエンドテストガイド](TESTING_FRONTEND.md)
- [バックエンドカバレッジレポート](src-tauri/COVERAGE_REPORT.md)
