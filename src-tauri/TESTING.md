# テストとカバレッジガイド

このドキュメントでは、プロジェクトのテスト実行とカバレッジ計測の方法を説明します。

## テスト実行

### 基本的なテスト実行

```bash
cargo test
```

### 特定のテストのみ実行

```bash
# gmail モジュールのテストのみ
cargo test gmail::tests

# 特定のテスト関数
cargo test test_gmail_message_structure
```

### テスト出力を詳細表示

```bash
cargo test -- --nocapture
```

## カバレッジ計測

このプロジェクトは85%のコードカバレッジを目標としています。

### cargo-llvm-covを使用したカバレッジ計測

このプロジェクトでは`cargo-llvm-cov`を使用してカバレッジを計測します。

#### インストール（初回のみ）

```bash
cargo install cargo-llvm-cov
```

#### 基本的な使用方法

HTMLレポート生成：
```bash
cargo llvm-cov --all-features --workspace --html
```

HTMLレポートは `target/llvm-cov/html/index.html` に生成されます。

テキスト形式でコンソール出力：
```bash
cargo llvm-cov --all-features --workspace
```

LCOV形式（CI/CDで使用）：
```bash
cargo llvm-cov --all-features --workspace --lcov --output-path coverage.lcov
```

JSON形式：
```bash
cargo llvm-cov --all-features --workspace --json --output-path coverage.json
```

#### 簡易スクリプト

PowerShell（Windows）:
```powershell
powershell -ExecutionPolicy Bypass -File coverage.ps1
```

Git Bash:
```bash
bash coverage.sh
```

#### よく使うコマンド

古いカバレッジデータをクリーンアップ：
```bash
cargo llvm-cov clean
```

特定のテストのみでカバレッジ計測：
```bash
cargo llvm-cov --html --test gmail_tests
```

カバレッジ閾値チェック：
```bash
cargo llvm-cov --fail-under-lines 85
```

## テストカバレッジの目標

- **全体目標**: 85%
- **重要機能**: Gmail同期、データベース操作は90%以上を目指す
- **除外対象**:
  - 自動生成コード
  - テストコード自体
  - main.rs（エントリーポイント）

## 現在のテストケース

### gmail.rs のテスト
- `test_gmail_message_structure` - GmailMessage構造体のテスト
- `test_fetch_result_structure` - FetchResult構造体のテスト
- `test_sync_state_*` - 同期状態管理のテスト（7件）
- `test_save_messages_to_db_*` - データベース保存機能のテスト（5件）

### lib.rs のテスト
- `test_greet` - greet関数の基本動作
- `test_greet_empty` - 空文字列の処理
- `test_greet_special_characters` - 特殊文字（日本語）の処理

合計: 17テスト

## CI/CD統合

GitHub ActionsなどのCI環境では、以下のようにカバレッジを計測できます：

```yaml
- name: Install cargo-llvm-cov
  run: cargo install cargo-llvm-cov

- name: Run tests with coverage
  run: cargo llvm-cov --all-features --workspace --lcov --output-path coverage.lcov

- name: Upload coverage to Codecov
  uses: codecov/codecov-action@v3
  with:
    files: coverage.lcov
    fail_ci_if_error: true

- name: Check coverage threshold
  run: cargo llvm-cov --all-features --workspace --fail-under-lines 85
```

## 追加のテストが必要な領域

現在のカバレッジを向上させるため、以下の領域のテストを追加することを推奨します：

1. **GmailClient** - Gmail APIとの通信処理（モック使用）
2. **sync_gmail_incremental** - 増分同期ロジック
3. **エラーハンドリング** - 各種エラーケースの処理
4. **Tauriコマンド** - フロントエンドからの呼び出し（統合テスト）

## トラブルシューティング

### cargo-llvm-cov が見つからない場合

```bash
cargo install cargo-llvm-cov
```

### カバレッジが0%と表示される場合

llvm-tools-previewがインストールされているか確認：

```bash
rustup component add llvm-tools-preview
```

### 古いカバレッジデータが残っている場合

```bash
cargo llvm-cov clean
```
