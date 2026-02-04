/**
 * E2E テスト用 DB モック（Tauri 非稼働時のフォールバック）
 *
 * ブラウザのみで E2E 実行する際、@tauri-apps/plugin-sql は利用できない。
 * このモックは Tables / Orders など DB 依存 UI がエラーにならず表示されるようにする。
 * 将来の sql.js 統合時にはこのモックを置き換える。
 *
 * @see docs/plans/2025-01-30-db-mock-for-e2e.md
 */

import { sanitizeTableName } from './table-utils';

type SchemaColumn = {
  cid: number;
  name: string;
  type: string;
  notnull: number;
  dflt_value: unknown;
  pk: number;
};

/** PRAGMA table_info 用の最小スキーマ（id のみ） */
const MINIMAL_SCHEMA: SchemaColumn[] = [
  { cid: 0, name: 'id', type: 'INTEGER', notnull: 0, dflt_value: null, pk: 1 },
];

/** images 用スキーマ（getColumnLabel 分岐カバー用） */
const IMAGES_SCHEMA: SchemaColumn[] = [
  { cid: 0, name: 'id', type: 'INTEGER', notnull: 0, dflt_value: null, pk: 1 },
  {
    cid: 1,
    name: 'item_id',
    type: 'INTEGER',
    notnull: 1,
    dflt_value: null,
    pk: 0,
  },
  {
    cid: 2,
    name: 'item_name_normalized',
    type: 'TEXT',
    notnull: 0,
    dflt_value: null,
    pk: 0,
  },
  {
    cid: 3,
    name: 'file_name',
    type: 'TEXT',
    notnull: 0,
    dflt_value: null,
    pk: 0,
  },
  {
    cid: 4,
    name: 'created_at',
    type: 'TEXT',
    notnull: 0,
    dflt_value: null,
    pk: 0,
  },
];

/** orders 用スキーマ（Tables 画面の orders テーブル表示用） */
const ORDERS_SCHEMA: SchemaColumn[] = [
  { cid: 0, name: 'id', type: 'INTEGER', notnull: 0, dflt_value: null, pk: 1 },
  {
    cid: 1,
    name: 'shop_domain',
    type: 'TEXT',
    notnull: 0,
    dflt_value: null,
    pk: 0,
  },
  {
    cid: 2,
    name: 'shop_name',
    type: 'TEXT',
    notnull: 0,
    dflt_value: null,
    pk: 0,
  },
  {
    cid: 3,
    name: 'order_number',
    type: 'TEXT',
    notnull: 0,
    dflt_value: null,
    pk: 0,
  },
  {
    cid: 4,
    name: 'order_date',
    type: 'DATETIME',
    notnull: 0,
    dflt_value: null,
    pk: 0,
  },
  {
    cid: 5,
    name: 'created_at',
    type: 'TEXT',
    notnull: 1,
    dflt_value: null,
    pk: 0,
  },
  {
    cid: 6,
    name: 'updated_at',
    type: 'TEXT',
    notnull: 1,
    dflt_value: null,
    pk: 0,
  },
];

/** shop_settings 用スキーマ（E2E セルクリックテスト用） */
const SHOP_SETTINGS_SCHEMA: SchemaColumn[] = [
  { cid: 0, name: 'id', type: 'INTEGER', notnull: 0, dflt_value: null, pk: 1 },
  {
    cid: 1,
    name: 'shop_name',
    type: 'TEXT',
    notnull: 1,
    dflt_value: null,
    pk: 0,
  },
  {
    cid: 2,
    name: 'sender_address',
    type: 'TEXT',
    notnull: 1,
    dflt_value: null,
    pk: 0,
  },
  {
    cid: 3,
    name: 'parser_type',
    type: 'TEXT',
    notnull: 1,
    dflt_value: null,
    pk: 0,
  },
  {
    cid: 4,
    name: 'is_enabled',
    type: 'INTEGER',
    notnull: 0,
    dflt_value: null,
    pk: 0,
  },
  {
    cid: 5,
    name: 'subject_filters',
    type: 'TEXT',
    notnull: 0,
    dflt_value: null,
    pk: 0,
  },
  {
    cid: 6,
    name: 'created_at',
    type: 'TEXT',
    notnull: 0,
    dflt_value: null,
    pk: 0,
  },
  {
    cid: 7,
    name: 'updated_at',
    type: 'TEXT',
    notnull: 0,
    dflt_value: null,
    pk: 0,
  },
];

/** E2E 用シード: shop_settings（ページネーションテスト用に複数行） */
const SHOP_SETTINGS_ROWS = Array.from({ length: 55 }, (_, i) => ({
  id: i + 1,
  shop_name: `ショップ${i + 1}`,
  sender_address: `shop${i + 1}@example.com`,
  parser_type: 'hobbysearch_send',
  is_enabled: 1,
  subject_filters: null,
  created_at: '2024-01-01',
  updated_at: '2024-01-01',
}));

/** E2E 用シード: orders（Tables 画面の orders テーブル表示用） */
const MOCK_ORDERS_ROWS = [
  {
    id: 1,
    shop_domain: 'example.com',
    order_number: 'ORD-E2E-001',
    order_date: '2024-01-15',
    created_at: '2024-01-15',
    updated_at: '2024-01-15',
  },
];

/** E2E 用シード: orders + items + images + product_master（Orders ドロワー用） */
const MOCK_ORDER_ITEMS = [
  {
    id: 1,
    orderId: 1,
    itemName: 'E2Eテスト商品',
    itemNameNormalized: 'e2eテスト商品',
    price: 1500,
    quantity: 1,
    category: null,
    brand: null,
    createdAt: '2024-01-15',
    shopName: 'Example Shop',
    shopDomain: 'example.com',
    orderNumber: 'ORD-E2E-001',
    orderDate: '2024-01-15',
    fileName: null,
    deliveryStatus: 'delivered',
    // product_master からの情報
    maker: 'テストメーカー',
    series: 'テストシリーズ',
    productName: 'E2Eテスト商品（解析済）',
    scale: '1/7',
    isReissue: 0,
  },
];

export type E2EMockDb = {
  select: <T>(sql: string, args?: unknown[]) => Promise<T[]>;
  close: () => Promise<void>;
};

/** parse_skipped 用スキーマ */
const PARSE_SKIPPED_SCHEMA: SchemaColumn[] = [
  {
    cid: 0,
    name: 'email_id',
    type: 'INTEGER',
    notnull: 1,
    dflt_value: null,
    pk: 1,
  },
  {
    cid: 1,
    name: 'error_message',
    type: 'TEXT',
    notnull: 0,
    dflt_value: null,
    pk: 0,
  },
  {
    cid: 2,
    name: 'created_at',
    type: 'DATETIME',
    notnull: 1,
    dflt_value: null,
    pk: 0,
  },
];

function getSchemaForTable(tableName: string): SchemaColumn[] {
  if (tableName === 'images') return IMAGES_SCHEMA;
  if (tableName === 'orders') return ORDERS_SCHEMA;
  if (tableName === 'shop_settings') return SHOP_SETTINGS_SCHEMA;
  if (tableName === 'parse_skipped') return PARSE_SKIPPED_SCHEMA;
  return MINIMAL_SCHEMA;
}

export function createE2EMockDb(): E2EMockDb {
  return {
    select: async <T>(sql: string, args?: unknown[]): Promise<T[]> => {
      const normalized = sql.replace(/\s+/g, ' ').trim();

      if (normalized.startsWith('PRAGMA table_info')) {
        const match = normalized.match(/PRAGMA table_info\((\w+)\)/);
        const tableName = match?.[1];
        if (tableName) {
          try {
            sanitizeTableName(tableName);
            return getSchemaForTable(tableName) as unknown as T[];
          } catch {
            return [];
          }
        }
        return [];
      }

      if (normalized.includes('COUNT(*)')) {
        const tableMatch = normalized.match(/FROM\s+(\w+)/i);
        const tableName = tableMatch?.[1];
        let count = 0;
        if (tableName === 'shop_settings') count = SHOP_SETTINGS_ROWS.length;
        else if (tableName === 'orders') count = 1;
        else if (tableName === 'parse_skipped') count = 0;
        return [{ count }] as unknown as T[];
      }

      if (normalized === 'SELECT 1') {
        return [{}] as unknown as T[];
      }

      if (
        normalized.includes('SELECT *') &&
        normalized.includes('FROM shop_settings')
      ) {
        const arr = args ?? [];
        const limit = (arr[arr.length - 2] as number | undefined) ?? 50;
        const offset = (arr[arr.length - 1] as number | undefined) ?? 0;
        const slice = SHOP_SETTINGS_ROWS.slice(
          offset,
          limit != null ? offset + limit : undefined
        );
        return slice as unknown as T[];
      }

      if (
        normalized.includes('SELECT *') &&
        normalized.includes('FROM orders') &&
        (normalized.includes('LIMIT') || normalized.includes('OFFSET'))
      ) {
        const arr = args ?? [];
        const limit = (arr[arr.length - 2] as number | undefined) ?? 50;
        const offset = (arr[arr.length - 1] as number | undefined) ?? 0;
        const slice = MOCK_ORDERS_ROWS.slice(
          offset,
          limit != null ? offset + limit : undefined
        );
        return slice as unknown as T[];
      }

      if (
        normalized.includes('FROM items') &&
        normalized.includes('JOIN orders') &&
        normalized.includes('LEFT JOIN images')
      ) {
        return MOCK_ORDER_ITEMS as unknown as T[];
      }

      if (normalized.includes('COALESCE(shop_name, shop_domain)')) {
        return [{ shop_display: 'example.com' }] as unknown as T[];
      }

      if (
        normalized.includes("strftime('%Y'") &&
        normalized.includes('order_date')
      ) {
        return [{ yr: '2024' }] as unknown as T[];
      }

      return [];
    },
    close: async () => {},
  };
}
