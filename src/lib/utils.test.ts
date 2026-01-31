import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  cn,
  notify,
  formatDate,
  formatDateTime,
  formatPrice,
  parseNumericFilter,
} from './utils';

// Tauri Notification APIをモック
const mockIsPermissionGranted = vi.fn();
const mockRequestPermission = vi.fn();
const mockSendNotification = vi.fn();

vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: () => mockIsPermissionGranted(),
  requestPermission: () => mockRequestPermission(),
  sendNotification: (options: unknown) => mockSendNotification(options),
}));

describe('cn utility', () => {
  it('merges class names correctly', () => {
    const result = cn('text-red-500', 'bg-blue-500');
    expect(result).toBe('text-red-500 bg-blue-500');
  });

  it('handles conditional classes', () => {
    const isActive = true;
    const isHidden = false;
    const result = cn(
      'base-class',
      isActive && 'conditional-class',
      isHidden && 'hidden-class'
    );
    expect(result).toBe('base-class conditional-class');
  });

  it('merges Tailwind classes correctly', () => {
    // Tailwind merge should resolve conflicting classes
    const result = cn('px-2 py-1', 'px-4');
    expect(result).toBe('py-1 px-4');
  });

  it('handles undefined and null', () => {
    const result = cn('base', undefined, null, 'other');
    expect(result).toBe('base other');
  });

  it('handles empty input', () => {
    const result = cn();
    expect(result).toBe('');
  });

  it('handles arrays of classes', () => {
    const result = cn(['class1', 'class2'], 'class3');
    expect(result).toBe('class1 class2 class3');
  });

  it('handles objects with conditional classes', () => {
    const result = cn({
      'base-class': true,
      hidden: false,
      active: true,
    });
    expect(result).toContain('base-class');
    expect(result).toContain('active');
    expect(result).not.toContain('hidden');
  });
});

describe('notify utility', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('sends notification when permission is already granted', async () => {
    mockIsPermissionGranted.mockResolvedValue(true);

    await notify('Test Title', 'Test Body');

    expect(mockIsPermissionGranted).toHaveBeenCalled();
    expect(mockRequestPermission).not.toHaveBeenCalled();
    expect(mockSendNotification).toHaveBeenCalledWith({
      title: 'Test Title',
      body: 'Test Body',
    });
  });

  it('requests permission and sends notification when permission is granted', async () => {
    mockIsPermissionGranted.mockResolvedValue(false);
    mockRequestPermission.mockResolvedValue('granted');

    await notify('Test Title', 'Test Body');

    expect(mockIsPermissionGranted).toHaveBeenCalled();
    expect(mockRequestPermission).toHaveBeenCalled();
    expect(mockSendNotification).toHaveBeenCalledWith({
      title: 'Test Title',
      body: 'Test Body',
    });
  });

  it('does not send notification when permission is denied', async () => {
    const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

    mockIsPermissionGranted.mockResolvedValue(false);
    mockRequestPermission.mockResolvedValue('denied');

    await notify('Test Title', 'Test Body');

    expect(mockIsPermissionGranted).toHaveBeenCalled();
    expect(mockRequestPermission).toHaveBeenCalled();
    expect(mockSendNotification).not.toHaveBeenCalled();
    expect(consoleSpy).toHaveBeenCalledWith(
      'Notification permission not granted'
    );

    consoleSpy.mockRestore();
  });

  it('handles Japanese characters in notification', async () => {
    mockIsPermissionGranted.mockResolvedValue(true);

    await notify('同期完了', 'メールの同期が完了しました');

    expect(mockSendNotification).toHaveBeenCalledWith({
      title: '同期完了',
      body: 'メールの同期が完了しました',
    });
  });

  it('handles empty title and body', async () => {
    mockIsPermissionGranted.mockResolvedValue(true);

    await notify('', '');

    expect(mockSendNotification).toHaveBeenCalledWith({
      title: '',
      body: '',
    });
  });

  it('handles long notification content', async () => {
    mockIsPermissionGranted.mockResolvedValue(true);

    const longTitle = 'A'.repeat(100);
    const longBody = 'B'.repeat(500);

    await notify(longTitle, longBody);

    expect(mockSendNotification).toHaveBeenCalledWith({
      title: longTitle,
      body: longBody,
    });
  });
});

describe('formatDate', () => {
  it('formats ISO date string to ja-JP', () => {
    expect(formatDate('2024-01-15T00:00:00')).toMatch(
      /\d{4}\/\d{1,2}\/\d{1,2}/
    );
  });

  it('formats SQLite datetime format (YYYY-MM-DD HH:MM:SS) by normalizing to ISO8601', () => {
    expect(formatDate('2024-01-15 12:30:45')).toMatch(
      /\d{4}\/\d{1,2}\/\d{1,2}/
    );
  });

  it('returns "-" for null', () => {
    expect(formatDate(null)).toBe('-');
  });

  it('returns "-" for undefined', () => {
    expect(formatDate(undefined)).toBe('-');
  });

  it('returns "-" for empty string', () => {
    expect(formatDate('')).toBe('-');
  });

  it('returns original string for invalid date', () => {
    expect(formatDate('invalid-date')).toBe('invalid-date');
  });
});

describe('formatDateTime', () => {
  it('formats UTC ISO string to JST', () => {
    // 2024-01-15 00:00:00 UTC = 2024-01-15 09:00:00 JST
    expect(formatDateTime('2024-01-15T00:00:00Z')).toMatch(
      /\d{4}\/\d{1,2}\/\d{1,2}/
    );
  });

  it('treats SQLite datetime (no timezone) as UTC and converts to JST', () => {
    // 2024-01-15 00:00:00 UTC = 2024/1/15 9:00:00 JST
    const result = formatDateTime('2024-01-15 00:00:00');
    expect(result).toContain('2024/1/15');
    expect(result).toContain('9:00'); // JST = UTC+9
  });

  it('formats SQLite datetime format', () => {
    expect(formatDateTime('2024-01-15 12:30:45')).toMatch(
      /\d{4}\/\d{1,2}\/\d{1,2}/
    );
  });

  it('returns "-" for null', () => {
    expect(formatDateTime(null)).toBe('-');
  });

  it('returns "-" for undefined', () => {
    expect(formatDateTime(undefined)).toBe('-');
  });
});

describe('formatPrice', () => {
  it('formats price with yen', () => {
    expect(formatPrice(1000)).toBe('1,000円');
  });

  it('formats zero', () => {
    expect(formatPrice(0)).toBe('0円');
  });
});

describe('parseNumericFilter', () => {
  it('returns undefined for empty string', () => {
    expect(parseNumericFilter('')).toBeUndefined();
  });

  it('returns undefined for null and undefined', () => {
    expect(parseNumericFilter(null as unknown as string)).toBeUndefined();
    expect(parseNumericFilter(undefined)).toBeUndefined();
  });

  it('returns undefined for invalid inputs like "-" and "e"', () => {
    expect(parseNumericFilter('-')).toBeUndefined();
    expect(parseNumericFilter('e')).toBeUndefined();
  });

  it('returns undefined for scientific notation "1e5"', () => {
    expect(parseNumericFilter('1e5')).toBeUndefined();
  });

  it('returns undefined for non-numeric strings', () => {
    expect(parseNumericFilter('abc')).toBeUndefined();
    expect(parseNumericFilter('  ')).toBeUndefined();
  });

  it('parses valid integers', () => {
    expect(parseNumericFilter('100')).toBe(100);
    expect(parseNumericFilter('0')).toBe(0);
    expect(parseNumericFilter('5000')).toBe(5000);
  });

  it('truncates floating point to integer', () => {
    expect(parseNumericFilter('1.5')).toBe(1);
    expect(parseNumericFilter('99.99')).toBe(99);
  });

  it('parses negative numbers', () => {
    expect(parseNumericFilter('-10')).toBe(-10);
  });

  it('handles large numbers', () => {
    expect(parseNumericFilter('999999999')).toBe(999999999);
  });

  it('ignores leading/trailing spaces for valid numbers', () => {
    expect(parseNumericFilter('  42  ')).toBe(42);
  });
});
