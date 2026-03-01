export function buildTrackingUrl(
  carrier: string | null,
  trackingNumber: string | null
): string | null {
  if (!carrier || !trackingNumber) return null;
  const num = encodeURIComponent(trackingNumber.trim());
  if (carrier.includes('佐川')) {
    return `https://k2k.sagawa-exp.co.jp/p/web/okurijosearch.do?okurijoNo=${num}`;
  }
  if (
    carrier.includes('日本郵便') ||
    carrier.includes('ゆうパケット') ||
    carrier.includes('ゆうパック')
  ) {
    return `https://trackings.post.japanpost.jp/services/srv/search/?requestNo=${num}`;
  }
  if (carrier.includes('ヤマト') || carrier.includes('クロネコ')) {
    return `https://jizen.kuronekoyamato.co.jp/jizen/servlet/crjz.b.NQ0010?id=${num}`;
  }
  return null;
}
