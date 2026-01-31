import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * アプリ全体で使用するタイムゾーン（日本標準時）。
 * バックエンドは chrono_tz::Asia::Tokyo、DB は UTC。規約: README §4。
 */
const JST = 'Asia/Tokyo';

/**
 * タイムゾーン未指定の日付文字列を UTC としてパースする。
 * SQLite などバックエンドは UTC で保存しているため、Z やオフセットがない場合は UTC として解釈する。
 *
 * タイムゾーン判定: 日付のみ（"2024-01-01"）では "01-01" が誤って [+-]XX:XX とマッチするため、
 * 時刻部分（T と :）が含まれる場合のみ正規表現で判定する。
 */
function parseAsUtcIfNeeded(s: string): Date {
  let normalized =
    s.includes(' ') && !s.includes('T') ? s.replace(' ', 'T') : s;
  const hasTimePart = normalized.includes('T') && normalized.includes(':');
  const hasTimezone = hasTimePart && /Z|[+-]\d{2}:?\d{2}$/.test(normalized);
  if (!hasTimezone) {
    normalized += 'Z';
  }
  return new Date(normalized);
}

/**
 * ISO日付文字列を ja-JP 形式でフォーマット（日付のみ、JST）。
 * SQLite の "YYYY-MM-DD HH:MM:SS" 形式は WebKit で Invalid Date になることがあるため、
 * スペースを "T" に置換して ISO8601 形式に正規化してからパースする。
 * バックエンドは UTC で保存しているため、タイムゾーン未指定の場合は UTC として解釈する。
 */
export function formatDate(s: string | null | undefined): string {
  if (!s) return '-';
  try {
    const d = parseAsUtcIfNeeded(s);
    return isNaN(d.getTime())
      ? s
      : d.toLocaleDateString('ja-JP', { timeZone: JST });
  } catch {
    return s;
  }
}

/**
 * 日時文字列を ja-JP 形式でフォーマット（日付+時刻、JST）。
 * バックエンドの UTC 日時を JST で表示するために使用する。
 */
export function formatDateTime(s: string | null | undefined): string {
  if (!s) return '-';
  try {
    const d = parseAsUtcIfNeeded(s);
    return isNaN(d.getTime())
      ? s
      : d.toLocaleString('ja-JP', { timeZone: JST });
  } catch {
    return s;
  }
}

/**
 * Parses numeric filter input; returns undefined for empty/invalid (e.g. "-", "e", "1e5").
 * Prevents NaN from being passed to queries.
 * Rejects scientific notation since parseInt("1e5", 10) returns 1 (surprising).
 */
export function parseNumericFilter(
  val: string | undefined
): number | undefined {
  if (val == null || val === '') return undefined;
  if (/e/i.test(val)) return undefined;
  const parsed = parseInt(val, 10);
  return Number.isFinite(parsed) ? parsed : undefined;
}

const priceFormatter = new Intl.NumberFormat('ja-JP');

/** 価格を円表示でフォーマット */
export function formatPrice(price: number): string {
  return priceFormatter.format(price) + '円';
}

/**
 * Send a desktop notification
 * @param title - Notification title
 * @param body - Notification body text
 * @returns Promise that resolves when notification is sent
 */
export async function notify(title: string, body: string): Promise<void> {
  let permissionGranted = await isPermissionGranted();

  if (!permissionGranted) {
    const permission = await requestPermission();
    permissionGranted = permission === 'granted';
  }

  if (permissionGranted) {
    await sendNotification({ title, body });
  } else {
    console.warn('Notification permission not granted');
  }
}
