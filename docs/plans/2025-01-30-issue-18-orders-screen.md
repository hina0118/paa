# Issue #18 商品一覧画面（Orders）実装計画

> **For Claude:** 実装時は superpowers:executing-plans を使用してタスク単位で進めること。

**Goal:** メニューの Orders 画面を、受信トレイサンプル（EmailList）から、大量の注文データを快適に閲覧・管理できる商品一覧画面へ完全に置き換える。

**Architecture:** items テーブルを軸に orders / images / deliveries を JOIN した一覧を表示。仮想スクロールで 5,000 件以上のデータでも軽快に動作。カード型とリスト型の表示切替、検索・フィルタ・ソートを実装。

**Tech Stack:** React, TanStack Virtual, @tauri-apps/plugin-sql, convertFileSrc（画像表示）, Tailwind CSS, shadcn/ui

---

## 現状サマリ

| 項目            | 状態                                                             |
| --------------- | ---------------------------------------------------------------- |
| Orders 画面     | `App.tsx` で `EmailList`（受信トレイサンプル）を表示             |
| データ          | モック `emailData`（`@/lib/data`）を使用、DB は未参照            |
| DB スキーマ     | orders, items, images, deliveries が揃っている（001_init.sql）   |
| 型定義          | `Item`, `Order`, `ItemImage`, `Delivery` が `@/lib/types` に存在 |
| Tables メニュー | `OrdersTable`（汎用 TableViewer）は別途存在、Orders 画面とは別物 |

---

## データモデル（商品一覧表示用）

商品一覧の 1 行 = 1 item。以下の JOIN で取得:

```sql
SELECT
  i.id, i.order_id, i.item_name, i.item_name_normalized, i.price, i.quantity,
  i.category, i.brand, i.created_at,
  o.shop_domain, o.order_number, o.order_date,
  img.file_name,
  d.delivery_status
FROM items i
JOIN orders o ON i.order_id = o.id
LEFT JOIN images img ON img.item_id = i.id
LEFT JOIN deliveries d ON d.order_id = o.id
```

注文に deliveries が複数ある場合は、最新 1 件を採用（`ORDER BY d.updated_at DESC LIMIT 1` はサブクエリで対応）。

---

## タスク一覧

### Task 1: 依存パッケージの追加

**目的:** 仮想スクロールと画像遅延読み込みに必要なパッケージを追加する。

**Files:**

- Modify: `package.json`

**Step 1: パッケージ追加**

```bash
npm install @tanstack/react-virtual
```

**Step 2: 動作確認**

```bash
npm run build
```

Expected: ビルド成功

**Step 3: コミット**

```bash
git add package.json package-lock.json
git commit -m "chore: add @tanstack/react-virtual for Orders screen (issue #18)"
```

---

### Task 2: 商品一覧用型・データ取得 API の追加

**目的:** 商品一覧表示に必要な型と、DB から items 一覧を取得する関数を追加する。

**Files:**

- Modify: `src/lib/types.ts`
- Create: `src/lib/orders-queries.ts`

**Step 1: 型の追加**

`src/lib/types.ts` に追加:

```typescript
/** 商品一覧 1 件分（items + order + image + delivery） */
export type OrderItemRow = {
  id: number;
  orderId: number;
  itemName: string;
  itemNameNormalized: string | null;
  price: number;
  quantity: number;
  category: string | null;
  brand: string | null;
  createdAt: string;
  shopDomain: string | null;
  orderNumber: string | null;
  orderDate: string | null;
  fileName: string | null;
  deliveryStatus: DeliveryStatus | null;
};
```

**Step 2: クエリ関数の作成**

Create `src/lib/orders-queries.ts`:

```typescript
import { sanitizeTableName } from '@/lib/database';
import type { OrderItemRow } from '@/lib/types';

type LoadParams = {
  search?: string;
  shopDomain?: string;
  year?: number;
  priceMin?: number;
  priceMax?: number;
  sortBy?: 'order_date' | 'price';
  sortOrder?: 'asc' | 'desc';
};

export async function loadOrderItems(
  db: { select: <T>(sql: string, args?: unknown[]) => Promise<T[]> },
  params: LoadParams = {}
): Promise<OrderItemRow[]> {
  const {
    search = '',
    shopDomain,
    year,
    priceMin,
    priceMax,
    sortBy = 'order_date',
    sortOrder = 'desc',
  } = params;

  const conditions: string[] = ['1=1'];
  const args: unknown[] = [];

  if (search.trim()) {
    conditions.push(
      '(i.item_name LIKE ? OR i.brand LIKE ? OR o.order_number LIKE ? OR o.shop_domain LIKE ?)'
    );
    const pattern = `%${search.trim()}%`;
    args.push(pattern, pattern, pattern, pattern);
  }
  if (shopDomain) {
    conditions.push('o.shop_domain = ?');
    args.push(shopDomain);
  }
  if (year) {
    conditions.push("strftime('%Y', o.order_date) = ?");
    args.push(String(year));
  }
  if (priceMin != null) {
    conditions.push('i.price >= ?');
    args.push(priceMin);
  }
  if (priceMax != null) {
    conditions.push('i.price <= ?');
    args.push(priceMax);
  }

  const orderCol =
    sortBy === 'price' ? 'i.price' : 'COALESCE(o.order_date, o.created_at)';
  const orderDir = sortOrder.toUpperCase();

  const sql = `
    SELECT
      i.id,
      i.order_id AS orderId,
      i.item_name AS itemName,
      i.item_name_normalized AS itemNameNormalized,
      i.price,
      i.quantity,
      i.category,
      i.brand,
      i.created_at AS createdAt,
      o.shop_domain AS shopDomain,
      o.order_number AS orderNumber,
      o.order_date AS orderDate,
      img.file_name AS fileName,
      (SELECT d2.delivery_status FROM deliveries d2
       WHERE d2.order_id = o.id
       ORDER BY d2.updated_at DESC LIMIT 1) AS deliveryStatus
    FROM items i
    JOIN orders o ON i.order_id = o.id
    LEFT JOIN images img ON img.item_id = i.id
    WHERE ${conditions.join(' AND ')}
    ORDER BY ${orderCol} ${orderDir}
  `;

  const rows = await db.select<OrderItemRow[]>(sql, args);
  return rows;
}

export async function getOrderItemFilterOptions(db: {
  select: <T>(sql: string, args?: unknown[]) => Promise<T[]>;
}): Promise<{ shopDomains: string[]; years: number[] }> {
  const [shops, years] = await Promise.all([
    db.select<{ shop_domain: string }[]>(
      'SELECT DISTINCT shop_domain FROM orders WHERE shop_domain IS NOT NULL ORDER BY shop_domain'
    ),
    db.select<{ yr: string }[]>(
      "SELECT DISTINCT strftime('%Y', order_date) AS yr FROM orders WHERE order_date IS NOT NULL AND trim(strftime('%Y', order_date)) != '' ORDER BY yr DESC"
    ),
  ]);
  return {
    shopDomains: shops.map((r) => r.shop_domain),
    years: years.map((r) => parseInt(r.yr, 10)).filter((n) => !isNaN(n)),
  };
}
```

**Step 3: コミット**

```bash
git add src/lib/types.ts src/lib/orders-queries.ts
git commit -m "feat(orders): add OrderItemRow type and loadOrderItems query (issue #18)"
```

---

### Task 3: Orders 画面コンポーネントの骨格作成

**目的:** EmailList の代わりに表示する Orders 画面の基本レイアウトを作成する。

**Files:**

- Create: `src/components/screens/orders.tsx`
- Modify: `src/App.tsx`

**Step 1: Orders 画面の作成**

Create `src/components/screens/orders.tsx`:

```tsx
import { ShoppingCart } from 'lucide-react';

export function Orders() {
  return (
    <div className="container mx-auto py-10 px-6">
      <div className="mb-8 space-y-2">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-primary/10">
            <ShoppingCart className="h-6 w-6 text-primary" />
          </div>
          <div>
            <h1 className="text-3xl font-bold tracking-tight">商品一覧</h1>
            <p className="text-sm text-muted-foreground mt-1">
              注文商品を閲覧・管理
            </p>
          </div>
        </div>
      </div>
      <div className="text-muted-foreground py-12 text-center">
        読み込み中...
      </div>
    </div>
  );
}
```

**Step 2: App.tsx で Orders に差し替え**

`src/App.tsx` の `case 'orders':` を変更:

```tsx
case 'orders':
  return <Orders />;
```

`EmailList` の import を削除し、`Orders` を import する。

**Step 3: 動作確認**

```bash
npm run tauri dev
```

Expected: Orders メニューをクリックすると「商品一覧」タイトルが表示される。

**Step 4: コミット**

```bash
git add src/components/screens/orders.tsx src/App.tsx
git commit -m "feat(orders): add Orders screen skeleton, replace EmailList (issue #18)"
```

---

### Task 4: 検索・フィルタバーの実装

**目的:** 商品名・ショップ名・注文番号での検索、ショップ別・購入年別・価格帯別のフィルタを追加する。

**Files:**

- Modify: `src/components/screens/orders.tsx`
- Modify: `src/components/ui/input.tsx`（既存を利用）
- Modify: `src/components/ui/dropdown-menu.tsx` または Select コンポーネント追加

**手順:**

1. 検索入力欄（デバウンス付き）を追加
2. ショップ別・購入年別・価格帯別のドロップダウンフィルタを追加
3. `loadOrderItems` と `getOrderItemFilterOptions` を呼び出し
4. フィルタ変更時に一覧を再取得

（Select コンポーネントがなければ shadcn/ui の Select を追加するか、既存の DropdownMenu で代用）

**Step: コミット**

```bash
git add src/components/screens/orders.tsx
git commit -m "feat(orders): add search and filter bar (issue #18)"
```

---

### Task 5: 仮想スクロール付きカードグリッドの実装

**目的:** 仮想スクロールを導入し、カード型グリッドで商品を表示する。

**Files:**

- Modify: `src/components/screens/orders.tsx`
- Create: `src/components/orders/order-item-card.tsx`（カード UI）
- Create: `src/components/orders/status-badge.tsx`（ステータスバッジ）

**手順:**

1. `StatusBadge`: deliveryStatus を「予約済み」「発送待ち」「発送済み」「配送中」「到着済み」「キャンセル」などにマッピングして色分け表示
2. `OrderItemCard`: 画像（プレースホルダ or convertFileSrc）、商品名、価格、ショップ、日付、StatusBadge を表示
3. `@tanstack/react-virtual` の `useVirtualizer` でグリッド仮想スクロールを実装
4. レスポンシブ: 幅に応じて 2〜4 列

**Step: コミット**

```bash
git add src/components/screens/orders.tsx src/components/orders/
git commit -m "feat(orders): add virtual scroll card grid (issue #18)"
```

---

### Task 6: 画像の遅延読み込みと convertFileSrc 対応

**目的:** 画像を `app_data_dir/images/` から `convertFileSrc` で表示し、遅延読み込みで軽量化する。

**Files:**

- Modify: `src/components/orders/order-item-card.tsx`
- Modify: `tauri.conf.json`（asset プロトコル設定・CSP）- Issue #47 Phase 2 と同様

**手順:**

1. `convertFileSrc` で `appDataDir/images/{fileName}` を URL 化
2. `loading="lazy"` で画像遅延読み込み
3. `object-fit: cover` でアスペクト比を維持
4. 画像がない場合はプレースホルダ（例: 商品アイコン）を表示

**参考:** Issue #47 の Task 3（asset プロトコル）が未実施の場合は同時に対応。

**Step: コミット**

```bash
git add src/components/orders/order-item-card.tsx tauri.conf.json
git commit -m "feat(orders): lazy load images with convertFileSrc (issue #18)"
```

---

### Task 7: カード型 / リスト型の切り替え

**目的:** 表示モードをカード型とリスト型で切り替えられるようにする。

**Files:**

- Modify: `src/components/screens/orders.tsx`
- Create: `src/components/orders/order-item-row.tsx`（リスト型 1 行）

**手順:**

1. トグルボタン（カード / リスト）を追加
2. リスト型では 1 行に商品名・価格・ショップ・日付・ステータスを横並び
3. 仮想スクロールは両モードで共通利用

**Step: コミット**

```bash
git add src/components/screens/orders.tsx src/components/orders/
git commit -m "feat(orders): add card/list view toggle (issue #18)"
```

---

### Task 8: 並び替え機能

**目的:** 購入日順・価格順のソートを実装する。

**Files:**

- Modify: `src/components/screens/orders.tsx`
- Modify: `src/lib/orders-queries.ts`（既に sortBy / sortOrder 対応済み）

**手順:**

1. ソート用のドロップダウンまたはボタンを追加（「購入日が新しい順」「価格が高い順」など）
2. `loadOrderItems` の `sortBy`, `sortOrder` を更新して再取得

**Step: コミット**

```bash
git add src/components/screens/orders.tsx
git commit -m "feat(orders): add sort by date/price (issue #18)"
```

---

### Task 9: 詳細表示（ドロワー/モーダル）

**目的:** アイテムクリックで右側からドロワーを開き、詳細を表示する。

**Files:**

- Modify: `src/components/screens/orders.tsx`
- Create: `src/components/orders/order-item-drawer.tsx`

**手順:**

1. ドロワーは `npx shadcn@latest add sheet` で Sheet を追加して使用する。または既存の Dialog でモーダル表示でも可。
2. 選択アイテムの詳細（商品名、価格、注文番号、ショップ、配達状況など）を表示
3. 閉じるボタンでドロワーを閉じる

**Step: コミット**

```bash
git add src/components/screens/orders.tsx src/components/orders/
git commit -m "feat(orders): add item detail drawer (issue #18)"
```

---

### Task 10: EmailList 関連の削除

**目的:** 不要になった受信トレイサンプルを削除する。

**Files:**

- Delete: `src/components/emails/email-list.tsx`
- Delete: `src/components/emails/columns.tsx`
- Delete: `src/components/emails/data-table.tsx`
- Modify: `src/App.tsx`（default の fallback を `Orders` に変更）
- Modify: `src/lib/data.ts`（emailData が他で使われていなければ削除）

**手順:**

1. `EmailList` を参照している箇所が `App.tsx` の `default` のみか確認
2. 上記ファイルを削除
3. `App.tsx` の `default:` を `return <Orders />` に変更

**Step: コミット**

```bash
git add -A
git commit -m "chore: remove EmailList sample, use Orders as default (issue #18)"
```

---

### Task 11: テスト・E2E の追加

**目的:** フロント単体テストと E2E で Orders 画面の基本動作を検証する。

**Files:**

- Create: `src/components/screens/orders.test.tsx`
- Modify: `tests/e2e/navigation.spec.ts` など（Orders 画面への遷移を追加）

**手順:**

1. `orders.test.tsx`: モック DB で Orders を表示し、検索・フィルタが動作することを確認
2. E2E: Orders メニュークリック → 商品一覧が表示されることを確認

**Step: コミット**

```bash
git add src/components/screens/orders.test.tsx tests/e2e/
git commit -m "test: add Orders screen unit and E2E tests (issue #18)"
```

---

## ステータスバッジマッピング

| delivery_status  | 表示ラベル | 色（例）   |
| ---------------- | ---------- | ---------- |
| not_shipped      | 発送待ち   | グレー     |
| preparing        | 準備中     | イエロー   |
| shipped          | 発送済み   | ブルー     |
| in_transit       | 配送中     | ブルー     |
| out_for_delivery | 配達中     | ブルー     |
| delivered        | 到着済み   | グリーン   |
| failed           | 配達失敗   | レッド     |
| returned         | 返送       | オレンジ   |
| cancelled        | キャンセル | レッド     |
| null             | -          | 表示しない |

---

## 検証チェックリスト

- [ ] 5,000 件以上のデータでスクロールが軽快に動作する
- [ ] 検索・フィルタで正しく絞り込まれる
- [ ] 購入日・価格でのソートが正しく動作する
- [ ] カード型 / リスト型の切り替えができる
- [ ] アイテムクリックでドロワーが開く
- [ ] 画像がある場合は表示され、ない場合はプレースホルダが表示される
- [ ] EmailList は完全に削除され、Orders がメニューから表示される

---

## 注意事項

- **画像表示:** Issue #47 の Phase 2（asset プロトコル・CSP）が未実施の場合、Task 6 で同時に設定すること。
- **データ不足時:** items が 0 件の場合は「データがありません」メッセージを表示する。
- **パフォーマンス:** 仮想スクロールは必須。react-window の代替として TanStack Virtual を採用。

---

## 実行オプション

**計画は `docs/plans/2025-01-30-issue-18-orders-screen.md` に保存済み。**

**1. Subagent-Driven（このセッション）** — タスクごとにサブエージェントを起動し、タスク間でレビューしながら実装

**2. Parallel Session（別セッション）** — 新規セッションで executing-plans を使用し、チェックポイント付きで一括実行

どちらの方法で進めますか？
