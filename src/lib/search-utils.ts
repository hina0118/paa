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
 * items_fts において item_name と brand のみを検索対象とする FTS5 クエリを生成する。
 * 各トークンが item_name または brand のいずれかに含まれるものをヒットさせる。
 */
export function buildFts5ItemBrandQuery(userInput: string): string {
  const trimmed = userInput.trim();
  if (!trimmed) return '';

  const tokens = trimmed.split(/\s+/).filter(Boolean);
  if (tokens.length === 0) return '';

  const termParts = tokens.map((t) => {
    const escaped = t.replace(/"/g, '""');
    const quoted = `"${escaped}"`;
    return `(item_name:${quoted} OR brand:${quoted})`;
  });

  return termParts.join(' AND ');
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
