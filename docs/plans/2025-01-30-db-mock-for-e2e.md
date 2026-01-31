# DB モック化計画（E2E テスト用）

## 背景・目的

現在の E2E テスト（Playwright）は `npm run dev`（Vite 単体）で実行され、Tauri バックエンドが起動しない。そのため `@tauri-apps/plugin-sql` が利用できず、以下の機能が動作しない。

- **Tables 画面**: スキーマ取得・データ取得がエラーになり、更新・ページネーション・セルクリックのテストが不可
- **Orders 画面**: 商品データが空のまま。ドロワー開閉・カード/行表示のテストが不十分
- **Dashboard / Sync / Parse / Logs 等**: 各種 `invoke` 呼び出しも Tauri 依存（本計画の対象外）

**目的**: フロントエンドの DB アクセス部分だけをモック化し、Tables・Orders など DB 依存 UI の E2E テストを実行可能にする。

---

## 現状の構成

```
┌─────────────────────────────────────────────────────────┐
│ useDatabase() → DatabaseManager.getInstance().getDatabase()
│                              │
│                              ▼
│              @tauri-apps/plugin-sql (Database.load)
│              @tauri-apps/api/path (appDataDir, join)
│                              │
│                              ▼
│              Tauri IPC → Rust バックエンド (sqlx + SQLite)
└─────────────────────────────────────────────────────────┘
```

**DB を利用する主な場所**:

| ファイル            | 用途                                               |
| ------------------- | -------------------------------------------------- |
| `App.tsx`           | 初回 `SELECT 1` で接続確認                         |
| `orders.tsx`        | `loadOrderItems`, `getOrderItemFilterOptions`      |
| `table-viewer.tsx`  | `PRAGMA table_info`, `SELECT COUNT(*)`, `SELECT *` |
| `orders-queries.ts` | JOIN クエリ、DISTINCT 等                           |

**DB インターフェース**:

```ts
interface DbLike {
  select<T>(sql: string, args?: unknown[]): Promise<T[]>;
  close?(): Promise<void>; // DatabaseManager の cleanup で使用
}
```

---

## 方針比較

### 方針 A: sql.js によるブラウザ内 SQLite（推奨）

| 項目       | 内容                                                 |
| ---------- | ---------------------------------------------------- |
| 概要       | Tauri 未検出時に sql.js で in-memory SQLite を使用   |
| メリット   | 実際の SQL を実行。既存クエリをそのまま利用可能。    |
| デメリット | バンドル増加、FTS5/トリガー等で互換性に制限の可能性  |
| 検出       | `typeof window !== 'undefined' && !window.__TAURI__` |

### 方針 B: パターンベースのモック

| 項目       | 内容                                             |
| ---------- | ------------------------------------------------ |
| 概要       | SQL 文字列やクエリ種別で分岐し、固定データを返す |
| メリット   | 軽量、依存なし                                   |
| デメリット | クエリごとにモックをメンテ。変更に弱い           |

### 方針 C: DI（依存性注入）

| 項目       | 内容                                                |
| ---------- | --------------------------------------------------- |
| 概要       | `getDb` を Context で注入し、E2E 時のみモックを渡す |
| メリット   | アプリ本体は変更少なめ                              |
| デメリット | Playwright から Context を差し替える方法が複雑      |

---

## 推奨: 方針 A（sql.js）の実装計画

### 1. 依存関係

```json
{
  "devDependencies": {
    "sql.js": "^1.10.0"
  }
}
```

- sql.js は WebAssembly ベースの SQLite。通常ビルドに FTS5 は含まれていない場合がある。
- FTS5 非対応の場合は、スキーマから FTS5 とトリガーを除外した簡易版を使用。

### 2. 検出ロジック

```ts
// Tauri 環境かどうか
function isTauriEnv(): boolean {
  return typeof window !== 'undefined' && !!window.__TAURI__;
}
```

- `window.__TAURI__` は Tauri アプリの WebView 内でのみ存在する。

### 3. DatabaseManager の分岐

```ts
// database.ts の getDatabase() 内
if (isTauriEnv()) {
  // 既存: appDataDir + Database.load (tauri-plugin-sql)
} else {
  // 新規: sql.js で in-memory DB を初期化
  return this.initSqlJsDb();
}
```

### 4. スキーマの扱い

**001_init.sql の制約**:

- `CREATE VIRTUAL TABLE ... USING fts5(...)` → sql.js で未対応の可能性が高い → **省略**
- `CREATE TRIGGER` → 基本的にサポート → **そのまま実行**（FTS 関連トリガーは FTS5 と合わせて省略）

**E2E 用の簡易スキーマ**:

- `emails`, `orders`, `items`, `images`, `deliveries`, `order_emails`, `htmls`, `order_htmls` のテーブル定義
- `shop_settings`, `sync_metadata`, `window_settings`, `parse_metadata` のテーブル定義
- `items_fts` および関連トリガーは **作成しない**
- 初期データ: `sync_metadata`, `window_settings`, `parse_metadata`, `shop_settings` の INSERT を 001_init から流用

### 5. シードデータ（Orders テスト用）

```sql
INSERT INTO orders (shop_domain, order_number, order_date)
  VALUES ('example.com', 'ORD-001', '2024-01-15');
INSERT INTO items (order_id, item_name, price, quantity)
  VALUES (1, 'テスト商品', 1000, 1);
INSERT INTO images (item_id, file_name)
  VALUES (1, 'test.png');
INSERT INTO deliveries (order_id, delivery_status)
  VALUES (1, 'delivered');
```

- 最低限 1 件の注文・商品・画像・配送情報を用意し、Orders 画面のドロワー表示を検証可能にする。

### 6. 初回ロード

- sql.js は WASM を動的ロードするため、初回のみやや遅い。
- `initSqlJs()` を 1 回だけ実行し、その結果をシングルトンで保持。

### 7. 影響範囲

| ファイル                       | 変更内容                                           |
| ------------------------------ | -------------------------------------------------- |
| `src/lib/database.ts`          | `isTauriEnv()` 分岐、`initSqlJsDb()` 追加          |
| `src/lib/e2e-schema.ts` (新規) | E2E 用スキーマ + シード SQL                        |
| `package.json`                 | `sql.js` を devDependencies に追加                 |
| `vite.config.ts`               | 必要なら sql.js の WASM を public にコピーする設定 |

---

## 代替案: 方針 B（パターンモック）の概要

sql.js を導入したくない場合の簡易案。

```ts
// mock-db.ts
export function createMockDb(): DbLike {
  return {
    select: async (sql: string, args?: unknown[]) => {
      if (sql.includes('PRAGMA table_info')) return mockSchemaRows;
      if (sql.includes('COUNT(*)')) return [{ count: 1 }];
      if (sql.includes('FROM orders') && sql.includes('JOIN'))
        return mockOrderItems;
      if (sql.includes('SELECT 1')) return [{}];
      return [];
    },
    close: async () => {},
  };
}
```

- クエリの種類が増えるたびに分岐を追加する必要がある。
- 本格的な E2E には不向きだが、プロトタイプや段階的導入には使える。

---

## リスク・注意点

1. **本番バンドルへの混入**
   - sql.js は条件分岐で E2E 時のみロードする想定。本番（Tauri）ビルドでは `tauri build` で tree-shaking が働くか確認が必要。

2. **スキーマ乖離**
   - 001_init.sql と E2E 用スキーマの差分をドキュメント化し、意図的に簡略化している部分を明示する。

3. **パフォーマンス**
   - E2E はテスト開始時の 1 回だけ sql.js を初期化するため、体感影響は小さい想定。

---

## 実装ステップ

1. **Phase 1**: `isTauriEnv()` と分岐骨格の追加 ✅（2025-01-30 完了）
   - `src/lib/e2e-mock-db.ts` でパターンベースの最小モックを実装
   - shop_settings・orders/items のシードで Tables セルクリック・Orders ドロワーをテスト可能に
2. **Phase 2**: E2E 用スキーマの作成（FTS5 除外）※将来 sql.js 導入時
3. **Phase 3**: sql.js の統合と `initSqlJsDb()` 実装
4. ~~**Phase 4**: Orders 用シードデータの投入~~ ※Phase 1 でモックに含めた
5. ~~**Phase 5**: E2E テストの拡充~~ ※Phase 1 で実施済み

---

## 参考

- [sql.js ドキュメント](https://sql.js.org/)
- [@tauri-apps/plugin-sql](https://v2.tauri.app/plugin/sql/)
- playwright.config.ts のコメント（Phase 2 で Tauri 連携を検討）
