/**
 * FTS5 クエリ用エスケープ。
 * 各トークンを引用符で囲み、AND で結合する。
 * 予約語（OR, AND, - など）を無害化する。
 */
export function escapeFts5Query(userInput: string): string {
  const trimmed = userInput.trim();
  if (!trimmed) return '';

  const tokens = trimmed.split(/\s+/).filter(Boolean);
  if (tokens.length === 0) return '';

  const escaped = tokens
    .map((t) => {
      const escaped = t.replace(/"/g, '""');
      return `"${escaped}"`;
    })
    .join(' AND ');

  return escaped;
}

/**
 * items_fts の item_name と brand 列のみを検索対象とする FTS5 クエリを生成。
 * 列指定なしの MATCH だと category / item_name_normalized もヒットするため、
 * 仕様（item_name, brand のみ）に合わせて列を明示する。
 */
export function buildFts5ItemBrandQuery(userInput: string): string {
  const escaped = escapeFts5Query(userInput);
  if (!escaped) return '';
  return `(item_name:(${escaped}) OR brand:(${escaped}))`;
}

/**
 * LIKE 前方一致用パターンのエスケープ。
 * % と _ をエスケープし、末尾に % を付与しない（呼び出し側で prefix 用に付与する）。
 * SQLite の ESCAPE '\' と併用する。
 */
export function escapeLikePrefix(userInput: string): string {
  const trimmed = userInput.trim();
  if (!trimmed) return '';

  return trimmed
    .replace(/\\/g, '\\\\')
    .replace(/%/g, '\\%')
    .replace(/_/g, '\\_');
}
