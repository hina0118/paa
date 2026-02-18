export function formatNumber(num: number): string {
  return new Intl.NumberFormat('ja-JP').format(num);
}

export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 文字';
  return `${formatNumber(Math.round(bytes))} 文字`;
}

export function formatCurrency(amount: number): string {
  return `¥${formatNumber(amount)}`;
}

export function calculatePercentage(part: number, total: number): string {
  if (total === 0) return '0';
  return ((part / total) * 100).toFixed(1);
}
