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

### バックエンド: 57.13% 🔄 (目標85%に向けて改善中)
- **行カバレッジ**: 57.13% (1465行中628行未カバー)
- **関数カバレッジ**: 59.46% (148関数中60関数未カバー)
- **リージョンカバレッジ**: 56.14%
- **テスト数**: 67件

#### ファイル別カバレッジ
- `src-tauri/src/gmail.rs`: 66.94% (コア機能)
- `src-tauri/src/lib.rs`: 6.41% (主にTauriセットアップコード)
- `src-tauri/src/main.rs`: 0.00% (エントリーポイント)

#### 主な未カバー領域
1. **OAuth認証フロー** (`GmailClient::new`, `authenticate`)
   - 実際のGmail API通信が必要
   - モックフレームワークでのテストが推奨

2. **Gmail同期ロジック** (`sync_gmail_incremental`)
   - 複雑な非同期処理と状態管理
   - API呼び出しとDB操作の統合

3. **API呼び出し関数** (`fetch_batch`, `fetch_messages`)
   - Gmail API依存
   - wiremock/mockitoでのモックテストが可能

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
