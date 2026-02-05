/**
 * テーブル名の検証ユーティリティ（SQL インジェクション防止）
 *
 * database.ts と e2e-mock-db.ts の両方から参照するため、
 * 循環依存を避けるために別モジュールとして分離。
 */

export const VALID_TABLES = [
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
] as const;

export type ValidTableName = (typeof VALID_TABLES)[number];

export function isValidTableName(name: string): name is ValidTableName {
  return VALID_TABLES.includes(name as ValidTableName);
}

export function sanitizeTableName(tableName: string): string {
  if (!isValidTableName(tableName)) {
    throw new Error(
      `Table "${tableName}" is not allowed. ` +
        `Allowed tables are: ${VALID_TABLES.join(', ')}. ` +
        `This may indicate a configuration issue or a bug in the calling code.`
    );
  }
  return tableName;
}
