# Issue #49 対応計画

**Issue**: [#49 商品一覧検索: LIKE 先頭ワイルドカードによるパフォーマンス改善](https://github.com/hina0118/paa/issues/49)

## 目的

商品一覧の検索で `%${search}%` のような先頭ワイルドカードを使用しているため、SQLite がインデックスを利用できず、データ量増加時に遅延する可能性がある問題を解消する。

## 現状分析

### 対象コード

**ファイル**: `src/lib/orders-queries.ts`

**現在の検索条件** (L29-34):

```ts
if (search.trim()) {
  conditions.push(
    '(i.item_name LIKE ? OR i.brand LIKE ? OR o.order_number LIKE ? OR o.shop_domain LIKE ? OR o.shop_name LIKE ?)'
  );
  const pattern = `%${search.trim()}%`;
  args.push(pattern, pattern, pattern, pattern, pattern);
}
```

- 5 カラムすべてに `%検索語%` を適用
- 先頭ワイルドカードのため B-tree インデックスが効かず、フルスキャンになる

### 検索対象フィールドの分類

| フィールド   | テーブル | インデックス                        | DB 機能                   |
| ------------ | -------- | ----------------------------------- | ------------------------- |
| item_name    | items    | idx_items_item_name                 | **items_fts** (FTS5) あり |
| brand        | items    | idx_items_brand                     | **items_fts** (FTS5) あり |
| order_number | orders   | idx_orders_order_number_shop_domain | なし                      |
| shop_domain  | orders   | idx_orders_shop_domain              | なし                      |
| shop_name    | orders   | なし                                | なし                      |

### 既存の items_fts (FTS5)

`001_init.sql` L67-91 で定義済み:

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(
    item_name,
    item_name_normalized,
    brand,
    category,
    content=items,
    content_rowid=id,
    tokenize='unicode61 remove_diacritics 2'
);
```

- `item_name`, `item_name_normalized`, `brand`, `category` を対象
- トリガーで items との同期を維持

### 呼び出し元

- `src/components/screens/orders.tsx` (L77): `loadOrderItems(db, { search: searchDebounced, ... })`
- デバウンス 300ms で検索語を送信

## 対応方針

**ハイブリッド方式**:

1. **items 系 (item_name, brand)**: FTS5 の `MATCH` を使用（全文検索、インデックス有効）
2. **orders 系 (order_number, shop_domain, shop_name)**: 前方一致 `検索語%` に変更し、既存インデックスを利用

### 方針選定理由

| 案                          | メリット                                                  | デメリット                                                                     |
| --------------------------- | --------------------------------------------------------- | ------------------------------------------------------------------------------ |
| **1. FTS5 のみ**            | 全文検索で高性能                                          | order_number / shop_domain / shop_name は items_fts に含まれない               |
| **2. 後方一致のみ**         | 実装が単純                                                | 「語の途中」検索ができず UX 低下（例: 「ンダム」で「ガンダム」にヒットしない） |
| **3. トークン化検索**       | 柔軟                                                      | 実装コストが高い                                                               |
| **4. ハイブリッド（採用）** | FTS5 で items 高速化、orders は前方一致でインデックス利用 | orders は前方一致に限定（order_number などは実用上支障少ない）                 |

### UX の変化

- **items (商品名・ブランド)**: FTS5 により「語の途中」も含む検索が可能（例: 「ガンダム」→「RGガンダム」など）
- **orders (注文番号・ショップ)**: 前方一致のみ（例: 「1999」→「1999.co.jp」、「ORD」→「ORD-123」）
- 注文番号・ショップ検索は前方一致が一般的なため、実用上の影響は小さいと想定

## 対応計画

### Phase 1: FTS5 検索の導入（items 系）

#### 1.1 FTS5 検索ヘルパーの追加

**ファイル**: `src/lib/orders-queries.ts` または `src/lib/search-utils.ts`

**内容**:

- FTS5 の `MATCH` 用に検索語をエスケープする関数
- FTS5 予約語（`"`, `-`, 語句接頭辞など）のエスケープ
- 空文字・空白のみの場合は検索条件を追加しない

**参考**: SQLite FTS5 では `"` で囲むとフレーズ検索。単純な AND 検索の場合は `term1 term2` で可。ユーザー入力の `"` や `-` はエスケープが必要。

#### 1.2 loadOrderItems の検索条件をハイブリッドに変更

**ファイル**: `src/lib/orders-queries.ts`

**変更内容**:

```ts
// 変更前
conditions.push(
  '(i.item_name LIKE ? OR i.brand LIKE ? OR o.order_number LIKE ? OR o.shop_domain LIKE ? OR o.shop_name LIKE ?)'
);
args.push(pattern, pattern, pattern, pattern, pattern);

// 変更後（概念）
// items: items_fts と JOIN し MATCH で検索
// orders: 前方一致 LIKE 'term%'
conditions.push(
  `(
    i.id IN (SELECT rowid FROM items_fts WHERE items_fts MATCH ?)
    OR o.order_number LIKE ?
    OR o.shop_domain LIKE ?
    OR o.shop_name LIKE ?
  )`
);
args.push(ftsQuery, prefixPattern, prefixPattern, prefixPattern);
```

**具体的な SQL 構成**:

- `items_fts MATCH ?`: FTS5 クエリはトークン単位でマッチ（部分文字列もトークンに含まれればヒット）
- `o.order_number LIKE ?`: `term%` で前方一致
- `o.shop_domain LIKE ?`, `o.shop_name LIKE ?`: 同様に `term%`

**注意**: FTS5 は `content=items` の external content のため、`items` と `items_fts` の rowid は一致。`i.id IN (SELECT rowid FROM items_fts WHERE ...)` で正しく JOIN 可能。

#### 1.3 FTS5 クエリのエスケープ

**エスケープルール**:

- ダブルクォート `"` → `""` にエスケープ
- 先頭の `-` は除外（NOT 演算子として解釈されるため）
- 空・空白のみの場合は検索条件を追加しない

**トークン検索**: ユーザーが「RG ガンダム」と入力した場合、`RG ガンダム` または `"RG ガンダム"` で FTS5 に渡す。unicode61 トークナイザーではスペース区切りでトークン化される。

### Phase 2: テストの更新

#### 2.1 ユニットテストの修正

**ファイル**: `src/lib/orders-queries.test.ts`

**変更**:

- `applies search filter` (L32-37): `%商品%` → FTS5 用クエリと `商品%` の両方が含まれることを検証
- モック DB の `select` 呼び出し引数を新仕様に合わせる

**追加**:

- FTS5 エスケープ関数の単体テスト（新規ファイルまたは既存テスト内）
- 空文字・空白のみの検索で条件が追加されないことのテスト

### Phase 3: E2E テストの確認（任意）

**ファイル**: `tests/e2e-tauri/*.spec.ts` または `tests/e2e/*.spec.ts`

- 商品一覧画面で検索を実行する E2E があれば、実施して動作確認
- 既存の orders 系 E2E があれば、検索周りの回帰がないか確認

## タスク一覧

| #   | タスク                                        | ファイル                                             | 優先度 |
| --- | --------------------------------------------- | ---------------------------------------------------- | ------ |
| 1   | FTS5 クエリ用エスケープ関数の実装             | `src/lib/search-utils.ts` または `orders-queries.ts` | 高     |
| 2   | loadOrderItems の検索条件をハイブリッドに変更 | `src/lib/orders-queries.ts`                          | 高     |
| 3   | 検索フィルタのユニットテスト修正・追加        | `src/lib/orders-queries.test.ts`                     | 高     |
| 4   | 手動・E2E での動作確認                        | -                                                    | 中     |

## リスク・注意点

1. **FTS5 トークナイザー**: unicode61 は日本語の分かち書きをしないため、「ガンダム」で「RGガンダム」がヒットするかはトークン化に依存。要確認。
2. **マイグレーション**: リリース前のため、必要に応じて 001_init.sql にまとめて追記する。新規マイグレーションファイルは不要。
3. **sql.js / E2E**: `docs/plans/2025-01-30-db-mock-for-e2e.md` では FTS5 を除外するスキーマも検討されている。E2E で FTS5 非対応の場合は、フォールバック（従来の LIKE）を検討。
4. **XSS/SQL インジェクション**: ユーザー入力をバインドパラメータで渡すため、現状の設計で問題なし。FTS5 クエリ文字列のエスケープのみ注意。

## 関連

- 元 PR: [#48](https://github.com/hina0118/paa/pull/48)
- 影響ファイル: `src/lib/orders-queries.ts`
- スキーマ: `src-tauri/migrations/001_init.sql` (items_fts)
