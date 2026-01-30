# カバレッジ目標復旧計画（Issue #51 関連）

## 背景

PR #48 で Orders 画面を追加した際、カバレッジ閾値を一時的に引き下げた。以下を元の目標値に復旧する。

| 領域                     | 目標 | 現状（PR #48 時点）                                    |
| ------------------------ | ---- | ------------------------------------------------------ |
| フロントエンド（Vitest） | 85%  | lines 76%, functions 57%, branches 61%, statements 73% |
| E2E（Playwright 関数）   | 25%  | 21%                                                    |

## 対応方針：原因分析 → 対策実施

### Phase 1: 原因分析（必須）

閾値に合わせてテストを足す前に、**低下の原因を特定**する。

#### 1.1 フロントエンド（Vitest）

```bash
npm run test:frontend:coverage
```

**分析項目:**

- **新規追加ファイルの影響**
  - `src/components/orders/` 配下（order-item-card, order-item-row, order-item-drawer, status-badge）
  - `src/components/screens/orders.tsx`
  - `src/lib/orders-queries.ts`
  - `src/hooks/useImageUrl.ts`
- **カバレッジ不足の内訳**
  - ファイル別の lines/functions/branches/statements
  - 未カバーとなっている主なコードパス
- **既存ファイルの変化**
  - PR #48 で削除した EmailList 関連により、相対的に未カバー率が上昇していないか

#### 1.2 E2E（Playwright）

```bash
npm run test:e2e
# 実行後、coverage-e2e/coverage-data.json を確認
```

**分析項目:**

- **総関数数・カバー関数数の変化**
  - Orders 追加により総関数数が増えたか
  - カバーされている関数の割合の変化
- **未カバー領域**
  - Orders 画面のどの部分が E2E で触れられていないか
  - 他画面でカバーが落ちていないか

#### 1.3 分析結果のまとめ

- カバレッジ低下の主因（新規コード vs 既存コードの変化）
- 優先してテスト追加すべきファイル・フロー
- 目標達成に必要なテストの見積もり

### Phase 2: 対策実施

Phase 1 の結果に基づき、以下を実施する。

1. **フロントエンド**
   - 不足しているユニットテストの追加（主に `orders` 関連）
   - 必要に応じて既存コンポーネントのテスト拡充

2. **E2E**
   - Orders 画面の主要フロー（検索・フィルタ・ソート・詳細ドロワー）のテスト追加
   - 他画面で不足しているフローのテスト追加

3. **検証**
   - ローカルで `npm run test:frontend:coverage` と `npm run test:e2e` を実行
   - 閾値を満たすことを確認してから CI にマージ

## Phase 1 分析結果（2025-01-30 実施）

### フロントエンド

- **主因**: Orders 関連の新規ファイルが未テスト（order-item-card, order-item-row, order-item-drawer, status-badge, orders-queries, useImageUrl）
- **対策**: 上記コンポーネント・モジュールのユニットテストを追加済み

### E2E

- **主因**: Orders 画面の総関数数増加に対し、E2E で実行されるフローが限定的
- **対策**: 検索入力・リスト切替・ソート変更などの E2E テストを追加済み

## 関連

- Issue #51: E2E カバレッジ目標 25% への復旧
- PR #48: Issue #18 商品一覧画面の実装
