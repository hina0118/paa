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
 * ISO日付文字列を ja-JP 形式でフォーマット。
 * SQLite の "YYYY-MM-DD HH:MM:SS" 形式は WebKit で Invalid Date になることがあるため、
 * スペースを "T" に置換して ISO8601 形式に正規化してからパースする。
 */
export function formatDate(s: string | null | undefined): string {
  if (!s) return '-';
  try {
    const normalized =
      s.includes(' ') && !s.includes('T') ? s.replace(' ', 'T') : s;
    const d = new Date(normalized);
    return isNaN(d.getTime()) ? s : d.toLocaleDateString('ja-JP');
  } catch {
    return s;
  }
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
