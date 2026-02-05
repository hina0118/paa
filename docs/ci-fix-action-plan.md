# CI 成功のための対応計画

**作成日**: 2026-02-05  
**対応完了**: 2026-02-05

---

## 概要

CI が通らない項目を洗い出し、対応計画を立てました。  
**Phase 1（E2E）と Phase 2（Coverage）の対応を完了しました。**

---

## CI ワークフロー一覧と現状

| ワークフロー | ジョブ            | 現状        | 閾値/条件                                    |
| ------------ | ----------------- | ----------- | -------------------------------------------- |
| **Lint**     | lint-frontend     | ✅ 通過     | ESLint, Prettier                             |
|              | lint-rust         | ✅ 通過     | Clippy, cargo fmt                            |
| **Test**     | test-frontend     | ✅ 通過     | Vitest 360件                                 |
|              | test-rust         | ✅ 通過     | 331件                                        |
| **Coverage** | coverage-frontend | ❌ **失敗** | 85% (lines, functions, branches, statements) |
|              | coverage-rust     | 要確認      | 65% (行カバレッジ)                           |
| **E2E**      | e2e-tests         | ❌ **失敗** | 全テスト成功 + 関数カバレッジ 25%            |

---

## 1. フロントエンドカバレッジ (coverage-frontend) — 失敗

### 現状

```
lines:     83.04% (閾値 85%) ← 約2%不足
functions: 79.06% (閾値 85%) ← 約6%不足
statements: 81.98% (閾値 85%) ← 約3%不足
branches:  78.71% (閾値 85%) ← 約6%不足
```

### 低カバレッジの主なファイル

| ファイル                    | Stmts  | Branch | Funcs  | Lines  | 備考               |
| --------------------------- | ------ | ------ | ------ | ------ | ------------------ |
| **settings.tsx**            | 55.33% | 46.55% | 30.76% | 58.04% | 最大の足を引っ張り |
| **api-keys.tsx**            | 64.51% | 67.67% | 57.69% | 67.56% |                    |
| **database.ts**             | 70.66% | 75%    | 69.23% | 70.66% |                    |
| **batch-progress-types.ts** | 66.66% | 100%   | 0%     | 66.66% | 型定義、関数0      |
| **parse-provider.tsx**      | 84.28% | 83.33% | 85.71% | 84.05% | 境界付近           |
| **sync-provider.tsx**       | 85.93% | 60%    | 92.3%  | 85.71% | 境界付近           |

### 対応方針

**優先度 高（settings.tsx）**

- カバレッジ 55% → 85% 以上に引き上げ
- 未カバー行: 27-246, 300-503 付近
- バッチサイズ保存、パースバッチサイズ、Gemini 設定などのテスト追加

**優先度 高（api-keys.tsx）**

- カバレッジ 64% → 85% 以上
- 未カバー行: 217-236, 241-255 付近
- API キー保存・削除のエッジケースをテスト

**優先度 中（database.ts）**

- カバレッジ 70% → 85% 以上
- 未カバー: 95, 231-237, 248 付近
- エラーハンドリングや初期化のテスト追加

**優先度 低（閾値調整案）**

- 短期対応として、閾値を現状に合わせて一時的に引き下げる選択肢もある（lines: 83, functions: 79 など）
- 中長期ではテスト追加で 85% を維持する方針を推奨

---

## 2. E2E テスト — 11件失敗

### 失敗原因の分析

**根本原因: サイドバー・ナビゲーションの仕様変更**

- 旧: `Sync` と `Parse` が別画面
- 新: `Batch` に統合（Sync + Parse を 1 画面に集約）
- サイドバー項目: Dashboard, Orders, **Batch**, Logs, Shop Settings, API Keys, Settings

**Tables セクションのテーブル名不一致**

- E2E が期待: Sync Metadata, Window Settings, Parse Metadata
- 実際のテーブル: Emails, Orders, Items, Images, Deliveries, HTMLs, Order-Emails, Order-HTMLs, Shop Settings, **Product Master**
- Sync Metadata / Window Settings / Parse Metadata は存在しない

### 失敗しているテスト一覧

| テスト                                  | 原因                                                        |
| --------------------------------------- | ----------------------------------------------------------- |
| ナビゲーション › サイドバーが表示される | `Sync`, `Parse` ボタンが存在しない                          |
| ナビゲーション › Sync画面に遷移できる   | `Sync` ボタンが存在しない                                   |
| ナビゲーション › Parse画面に遷移できる  | `Parse` ボタンが存在しない                                  |
| Parse画面 › 全4件                       | `Parse` ボタンが存在しない（Batch に統合済み）              |
| Sync画面 › 全3件                        | `Sync` ボタンが存在しない（Batch に統合済み）               |
| Tables画面 › 複数のテーブルに遷移できる | Sync Metadata, Window Settings, Parse Metadata が存在しない |

### 対応方針

**Phase 1: ナビゲーション・Sync/Parse の修正**

1. **navigation.spec.ts**
   - `Sync` → `Batch` に変更
   - `Parse` を削除し、`Batch` 経由のテストに統合
   - サイドバー確認: `Sync`, `Parse` の代わりに `Batch` を期待

2. **sync.spec.ts**
   - 画面遷移: `navigateToScreen(page, 'Sync')` → `navigateToScreen(page, 'Batch')`
   - タイトル: `Gmail同期` は Batch 画面内のセクション見出しとして存在
   - 必要に応じて、Batch 画面内の「Gmail同期」セクションを特定するセレクタに変更

3. **parse.spec.ts**
   - 画面遷移: `navigateToScreen(page, 'Parse')` → `navigateToScreen(page, 'Batch')`
   - パース関連の UI は Batch 画面内に存在するため、その要素を対象にテストを書き直す

**Phase 2: Tables の修正**

4. **tables.spec.ts**
   - 期待するテーブル一覧を実際のものに合わせる:
     - `Sync Metadata`, `Window Settings`, `Parse Metadata` を削除
     - `Product Master` を追加
   - 修正後のリスト: Orders, Items, Images, Deliveries, HTMLs, Order-Emails, Order-HTMLs, Shop Settings, Product Master

---

## 3. Rust カバレッジ (coverage-rust) — 要確認

### 現状

- 閾値: 65%（行カバレッジ）
- 過去の COVERAGE_STATUS.md: 67.44% で通過
- `.llvm-cov.toml` で `src/gmail/` を除外済み（keyring 依存のため CI でスキップ）

### 対応

- CI の coverage-rust ジョブの結果を確認
- 65% 未満の場合は、`.llvm-cov.toml` の除外設定やテスト追加を検討

---

## 4. E2E カバレッジ (25%)

### 現状

- `coverage-reporter.ts` で関数カバレッジ 25% 未達時に CI 失敗
- E2E が 11 件失敗しているため、カバレッジ計測も不完全な可能性

### 対応

- 上記 E2E 修正で全テスト通過させたうえで、E2E カバレッジを再計測
- 25% 未達の場合は、不足している画面・操作の E2E を追加

---

## 対応順序の推奨

### Phase 1: E2E 修正（即時・必須）

| 順  | 対応内容                                                                  |
| --- | ------------------------------------------------------------------------- |
| 1   | **navigation.spec.ts** — Sync/Parse を Batch に変更、サイドバー項目を更新 |
| 2   | **sync.spec.ts** — Batch 画面へ遷移するように変更                         |
| 3   | **parse.spec.ts** — Batch 画面へ遷移するように変更                        |
| 4   | **tables.spec.ts** — テーブル一覧を実装に合わせて修正                     |

### Phase 2: フロントエンドカバレッジ

| 順  | 対応内容                                        |
| --- | ----------------------------------------------- |
| 5   | **settings.tsx** — テスト追加（85% 以上を目標） |
| 6   | **api-keys.tsx** — テスト追加                   |
| 7   | **database.ts** — 必要に応じてテスト追加        |

### Phase 3: 確認

| 順  | 対応内容                              |
| --- | ------------------------------------- |
| 8   | CI 全体を再実行し、全ジョブ通過を確認 |
| 9   | E2E カバレッジ 25% を確認             |

---

## 技術メモ

### サイドバー構成（現行）

```tsx
// src/components/layout/sidebar.tsx
navigationItems: Dashboard, Orders, Batch, Logs, Shop Settings, API Keys, Settings
tableItems: Emails, Orders, Items, Images, Deliveries, HTMLs, Order-Emails, Order-HTMLs, Shop Settings, Product Master
```

### Batch 画面の構成

- Gmail同期セクション（同期を開始、同期日時をリセット など）
- パースセクション（パースを開始、削除して実行 など）
- 商品名パースセクション

### expectSidebarVisible の期待

- `page.getByRole('complementary')` — `<aside>` は complementary として認識
- `page.getByText('PAA')` — サイドバーヘッダー
- ナビゲーションボタン: Dashboard, Orders, **Batch**, Logs, Shop Settings, Settings
