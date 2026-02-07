import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  formatError,
  toastSuccess,
  toastError,
  toastWarning,
  toastInfo,
} from './toast';

const mockSuccess = vi.fn();
const mockError = vi.fn();
const mockWarning = vi.fn();
const mockInfo = vi.fn();

vi.mock('sonner', () => ({
  toast: Object.assign(vi.fn(), {
    success: (...args: unknown[]) => mockSuccess(...args),
    error: (...args: unknown[]) => mockError(...args),
    warning: (...args: unknown[]) => mockWarning(...args),
    info: (...args: unknown[]) => mockInfo(...args),
  }),
}));

describe('toast utility', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('formatError', () => {
    it('returns message for Error instance', () => {
      expect(formatError(new Error('test error'))).toBe('test error');
    });

    it('returns stringified value for non-Error', () => {
      expect(formatError('string error')).toBe('string error');
      expect(formatError(123)).toBe('123');
    });
  });

  describe('toastSuccess', () => {
    it('calls sonner toast.success with message', () => {
      toastSuccess('成功しました');
      expect(mockSuccess).toHaveBeenCalledWith('成功しました', undefined);
    });

    it('calls sonner toast.success with description', () => {
      toastSuccess('成功', '詳細メッセージ');
      expect(mockSuccess).toHaveBeenCalledWith('成功', {
        description: '詳細メッセージ',
      });
    });
  });

  describe('toastError', () => {
    it('calls sonner toast.error with message', () => {
      toastError('エラーが発生しました');
      expect(mockError).toHaveBeenCalledWith('エラーが発生しました', undefined);
    });
  });

  describe('toastWarning', () => {
    it('calls sonner toast.warning with message', () => {
      toastWarning('警告です');
      expect(mockWarning).toHaveBeenCalledWith('警告です', undefined);
    });
  });

  describe('toastInfo', () => {
    it('calls sonner toast.info with message', () => {
      toastInfo('情報です');
      expect(mockInfo).toHaveBeenCalledWith('情報です', undefined);
    });
  });
});
