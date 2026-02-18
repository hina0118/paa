const numberFormatter = new Intl.NumberFormat('ja-JP');

export function formatNumber(num: number): string {
  return numberFormatter.format(num);
}

/**
 * 文字数（キャラクターカウント）を「〜 文字」の形式で整形します。
 *
 * NOTE: 引数はバイト数ではなく「文字数」です。将来的な誤用を避けるため、
 *       バイト数を扱う場合は別の関数を実装してください。
 */
export function formatCharacters(charCount: number): string {
  if (charCount === 0) return '0 文字';
  return `${formatNumber(Math.round(charCount))} 文字`;
}

/**
 * @deprecated 引数はバイト数ではなく「文字数」です。新しいコードでは formatCharacters を使用してください。
 */
export function formatBytes(charCount: number): string {
  return formatCharacters(charCount);
}
export function formatCurrency(amount: number): string {
  return `¥${formatNumber(amount)}`;
}

export function calculatePercentage(part: number, total: number): string {
  if (total === 0) return '0';
  return ((part / total) * 100).toFixed(1);
}
