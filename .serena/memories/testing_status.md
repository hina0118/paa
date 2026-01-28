# テスト環境とカバレッジ状況

最終更新: 2026-01-18

## テスト環境

### フロントエンド (React + TypeScript)

- **テストフレームワーク**: Vitest 4.0.17
- **テスティングライブラリ**: React Testing Library
- **DOM環境**: jsdom
- **カバレッジツール**: @vitest/coverage-v8
- **設定ファイル**: `vitest.config.ts`

### バックエンド (Rust)

- **テストフレームワーク**: Cargo test (標準)
- **カバレッジツール**: cargo-llvm-cov
- **データベーステスト**: sqlx + sqlite::memory

## 現在のカバレッジ状況

### フロントエンド: 93.60% ✅ (目標85%達成)

- **Statements**: 93.65%
- **Branches**: 80.35%
- **Functions**: 96.96%
- **Lines**: 93.60%
- **テスト数**: 122件

#### 100%カバレッジ達成ファイル

- `src/components/screens/dashboard.tsx` ✅
- `src/components/screens/settings.tsx` ✅
- `src/components/screens/sync.tsx` ✅
- `src/components/ui/button.tsx` ✅
- `src/components/ui/card.tsx` ✅
- `src/components/ui/checkbox.tsx` ✅
- `src/components/ui/input.tsx` ✅
- `src/components/ui/progress.tsx` ✅
- `src/contexts/navigation-context.tsx` ✅
- `src/lib/utils.ts` ✅

#### 高カバレッジ

- `src/contexts/sync-context.tsx`: 85.45%

### バックエンド: 63.03% 🔄 (前回57.13%から+5.90%向上)

- **行カバレッジ**: 63.03% (4845行中1791行未カバー)
- **関数カバレッジ**: 61.54% (520関数中200関数未カバー)
- **リージョンカバレッジ**: 66.57%
- **テスト数**: 206件 (183 + 15 + 8)

#### ファイル別カバレッジ

- `src-tauri/src/gmail.rs`: 70.83% (前回66.94%から+3.89%)
- `src-tauri/src/logic/sync_logic.rs`: 97.25% ✅ 新規追加
- `src-tauri/src/logic/email_parser.rs`: 96.03% ✅ 新規追加
- `src-tauri/src/repository.rs`: 88.94% ✅ 新規追加
- `src-tauri/src/parsers/hobbysearch_common.rs`: 99.59% ✅
- `src-tauri/src/parsers/hobbysearch_confirm.rs`: 93.27%
- `src-tauri/src/parsers/hobbysearch_send.rs`: 93.91%
- `src-tauri/src/lib.rs`: 12.82% (主にTauriセットアップコード)
- `src-tauri/src/main.rs`: 0.00% (エントリーポイント)

#### Issue #35 対応による改善点

1. **GmailClientTrait**: Gmail API操作を抽象化、mockall対応
2. **EmailRepository / ShopSettingsRepository**: DB操作をリポジトリパターンで抽象化
3. **logic層**: ビジネスロジックを純粋関数として切り出し
   - `sync_logic.rs`: クエリビルド、メールアドレス抽出、メッセージフィルタリング
   - `email_parser.rs`: パーサー候補取得、ドメイン抽出
4. **統合テスト**: `parser_integration_tests.rs` 追加（8テスト）

## テストコマンド

### フロントエンド

```bash
# ウォッチモード
npm run test:frontend

# 1回実行
npm run test:frontend:run

# カバレッジ測定
npm run test:frontend:coverage

# UIモード
npm run test:frontend:ui
```

### バックエンド

```bash
# テスト実行
cd src-tauri && cargo test

# カバレッジ測定
cd src-tauri && cargo llvm-cov --all-features --workspace

# HTMLレポート生成
cd src-tauri && cargo llvm-cov --all-features --workspace --html
```

### 全体

```bash
npm run test:all
```

## テストファイル構造

### フロントエンド

```
src/
├── components/
│   ├── screens/
│   │   ├── dashboard.test.tsx
│   │   ├── settings.test.tsx
│   │   └── sync.test.tsx
│   └── ui/
│       ├── button.test.tsx
│       ├── card.test.tsx
│       ├── checkbox.test.tsx
│       ├── input.test.tsx
│       └── progress.test.tsx
├── contexts/
│   ├── navigation-context.test.tsx
│   └── sync-context.test.tsx
├── lib/
│   └── utils.test.ts
└── test/
    └── setup.ts  # テスト環境設定・モック
```

### バックエンド

```
src-tauri/
├── src/
│   ├── gmail.rs  # テストモジュール内包 (#[cfg(test)])
│   └── lib.rs    # テストモジュール内包
└── tests/
    └── command_tests.rs  # 統合テスト
```

## テストベストプラクティス

### フロントエンド

1. **AAA パターン** (Arrange-Act-Assert)
2. **ユーザー中心のクエリ**: `getByRole` > `getByLabelText` > `getByTestId`
3. **非同期操作**: `findBy*` クエリを使用
4. **Tauri APIモック**: `src/test/setup.ts` で自動モック化

### バックエンド

1. **インメモリDB**: `sqlite::memory:` でテスト高速化
2. **トランザクション**: テストごとに独立したDB状態
3. **エラーケース**: 境界値・エラーパスも網羅
4. **非同期テスト**: `#[tokio::test]` を使用

## 改善履歴

### 2026-01-17

- フロントエンド: 61.94% → 93.60% (+31.66%)
- バックエンド: 36.13% → 57.13% (+21.00%)
- テスト総数: 33件 → 189件 (+156件)

## 今後の課題

### バックエンド

1. **モックフレームワーク導入**
   - `mockito` または `wiremock` を検討
   - OAuth認証フローのモックテスト

2. **統合テスト拡充**
   - `sync_gmail_incremental` の分岐網羅
   - エラーハンドリングの網羅

3. **カバレッジ目標**
   - 短期: 70% (現実的な目標)
   - 中期: 80%
   - 長期: 85% (モックテスト完備後)

### フロントエンド

1. **sync-context.tsx** のカバレッジ向上 (85.45% → 90%+)
2. **統合テスト** の追加 (画面間遷移など)

## 関連ドキュメント

- `TESTING_FRONTEND.md` - フロントエンドテストガイド
- `COVERAGE_STATUS.md` - カバレッジレポート
- `src-tauri/TESTING.md` - バックエンドテストガイド
