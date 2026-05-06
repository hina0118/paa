/**
 * テーブル名の検証ユーティリティ（SQL インジェクション防止）
 *
 * テーブル定義の正規リストは table-definitions.ts で一元管理。
 * database.ts と e2e-mock-db.ts の両方から参照するため、
 * 循環依存を避けるために別モジュールとして分離。
 */

import { VALID_TABLE_NAMES, type TableName } from './table-definitions';

export const VALID_TABLES = VALID_TABLE_NAMES;
export type ValidTableName = TableName;

export function isValidTableName(name: string): name is ValidTableName {
  return (VALID_TABLES as readonly string[]).includes(name);
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
