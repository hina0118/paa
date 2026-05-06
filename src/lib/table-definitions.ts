import { invoke } from '@tauri-apps/api/core';

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
    pending = invoke<TableDefinition[]>('get_table_definitions').then(
      (defs) => {
        cache = defs;
        return defs;
      }
    );
  }
  return pending;
}

export function tableNameToScreenId(name: string): string {
  return `table-${name.replace(/_/g, '-')}`;
}

export function screenIdToTableName(screenId: string): string {
  return screenId.slice(6).replace(/-/g, '_');
}
