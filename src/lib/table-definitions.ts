import { invoke } from '@tauri-apps/api/core';

// E2E (non-Tauri) fallback: must stay in sync with table_definitions.rs
const STATIC_TABLE_LABELS: Record<string, string> = {
  emails: 'メール',
  orders: '注文',
  items: '商品アイテム',
  images: '画像',
  deliveries: '配送情報',
  htmls: 'HTML本文',
  order_emails: '注文-メール',
  order_htmls: '注文-HTML',
  shop_settings: '店舗設定',
  product_master: '商品マスタ',
  item_overrides: 'アイテム上書き',
  order_overrides: '注文上書き',
  excluded_items: '除外アイテム',
  excluded_orders: '除外注文',
  tracking_check_logs: '配送確認ログ',
  news_clips: 'ニュースクリップ',
  item_exclusion_patterns: '除外キーワードパターン',
};

export const VALID_TABLE_NAMES = [
  'emails',
  'orders',
  'items',
  'images',
  'deliveries',
  'htmls',
  'order_emails',
  'order_htmls',
  'shop_settings',
  'product_master',
  'item_overrides',
  'order_overrides',
  'excluded_items',
  'excluded_orders',
  'tracking_check_logs',
  'news_clips',
  'item_exclusion_patterns',
] as const;

export type TableName = (typeof VALID_TABLE_NAMES)[number];

export type TableDefinition = { name: string; label: string };

let cache: TableDefinition[] | null = null;
let pending: Promise<TableDefinition[]> | null = null;

export async function getTableDefinitions(): Promise<TableDefinition[]> {
  if (cache) return cache;
  if (!pending) {
    pending = invoke<TableDefinition[]>('get_table_definitions')
      .then((defs) => {
        cache = defs;
        return defs;
      })
      .catch(() => {
        const fallback = VALID_TABLE_NAMES.map((name) => ({
          name,
          label: STATIC_TABLE_LABELS[name] ?? name,
        }));
        cache = fallback;
        return fallback;
      });
  }
  return pending;
}

export function tableNameToScreenId(name: string): string {
  return `table-${name.replace(/_/g, '-')}`;
}

export function screenIdToTableName(screenId: string): string {
  return screenId.slice(6).replace(/-/g, '_');
}
