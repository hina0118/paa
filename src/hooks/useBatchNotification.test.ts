import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useBatchNotification } from './useBatchNotification';
import type { BatchProgress } from '@/contexts/batch-progress-types';

const { toastSuccessMock, toastErrorMock, notifyMock, isAppWindowVisibleMock } =
  vi.hoisted(() => ({
    toastSuccessMock: vi.fn(),
    toastErrorMock: vi.fn(),
    notifyMock: vi
      .fn<[string, string], Promise<void>>()
      .mockResolvedValue(undefined),
    isAppWindowVisibleMock: vi
      .fn<[], Promise<boolean>>()
      .mockResolvedValue(true),
  }));

vi.mock('@/lib/toast', () => ({
  toastSuccess: (...args: unknown[]) => toastSuccessMock(...args),
  toastError: (...args: unknown[]) => toastErrorMock(...args),
}));

vi.mock('@/lib/utils', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/utils')>();
  return {
    ...actual,
    notify: (title: string, body: string) => notifyMock(title, body),
    isAppWindowVisible: () => isAppWindowVisibleMock(),
  };
});

function makeBatchProgress(
  overrides: Partial<BatchProgress> = {}
): BatchProgress {
  return {
    task_name: 'テスト',
    batch_number: 1,
    batch_size: 50,
    total_items: 100,
    processed_count: 100,
    success_count: 95,
    failed_count: 5,
    progress_percent: 100,
    status_message: 'Complete',
    is_complete: true,
    ...overrides,
  };
}

const buildMessage = (p: BatchProgress) =>
  `成功: ${p.success_count}件、失敗: ${p.failed_count}件`;

describe('useBatchNotification', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    isAppWindowVisibleMock.mockResolvedValue(true);
    notifyMock.mockResolvedValue(undefined);
  });

  it('calls toastSuccess when window is visible and no error', async () => {
    const { result } = renderHook(() =>
      useBatchNotification('メールパース', buildMessage, 'email parse')
    );

    await act(async () => {
      await result.current(
        makeBatchProgress({ success_count: 10, failed_count: 2 })
      );
    });

    expect(toastSuccessMock).toHaveBeenCalledWith(
      'メールパースが完了しました',
      '成功: 10件、失敗: 2件'
    );
    expect(notifyMock).not.toHaveBeenCalled();
  });

  it('calls toastError when window is visible and data has error', async () => {
    const { result } = renderHook(() =>
      useBatchNotification('Gmail同期', buildMessage, 'Gmail sync')
    );

    await act(async () => {
      await result.current(makeBatchProgress({ error: 'something broke' }));
    });

    expect(toastErrorMock).toHaveBeenCalledWith(
      'Gmail同期に失敗しました',
      'something broke'
    );
    expect(toastSuccessMock).not.toHaveBeenCalled();
    expect(notifyMock).not.toHaveBeenCalled();
  });

  it('calls notify with short success title when window is not visible and no error', async () => {
    isAppWindowVisibleMock.mockResolvedValue(false);

    const { result } = renderHook(() =>
      useBatchNotification('メールパース', buildMessage, 'email parse')
    );

    await act(async () => {
      await result.current(
        makeBatchProgress({ success_count: 5, failed_count: 1 })
      );
    });

    expect(notifyMock).toHaveBeenCalledWith(
      'メールパース完了',
      '成功: 5件、失敗: 1件'
    );
    expect(toastSuccessMock).not.toHaveBeenCalled();
  });

  it('calls notify with short failure title when window is not visible and data has error', async () => {
    isAppWindowVisibleMock.mockResolvedValue(false);

    const { result } = renderHook(() =>
      useBatchNotification('Gmail同期', buildMessage, 'Gmail sync')
    );

    await act(async () => {
      await result.current(makeBatchProgress({ error: 'network error' }));
    });

    expect(notifyMock).toHaveBeenCalledWith('Gmail同期失敗', 'network error');
    expect(toastErrorMock).not.toHaveBeenCalled();
  });

  it('logs console.error when notify throws on success path', async () => {
    isAppWindowVisibleMock.mockResolvedValue(false);
    notifyMock.mockRejectedValue(new Error('notify failed'));
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const { result } = renderHook(() =>
      useBatchNotification('メールパース', buildMessage, 'email parse')
    );

    await act(async () => {
      await result.current(makeBatchProgress());
    });

    expect(consoleSpy).toHaveBeenCalledWith(
      'Failed to send email parse completion notification:',
      expect.any(Error)
    );
    consoleSpy.mockRestore();
  });

  it('logs console.error when notify throws on error path', async () => {
    isAppWindowVisibleMock.mockResolvedValue(false);
    notifyMock.mockRejectedValue(new Error('notify failed'));
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const { result } = renderHook(() =>
      useBatchNotification('Gmail同期', buildMessage, 'Gmail sync')
    );

    await act(async () => {
      await result.current(makeBatchProgress({ error: 'sync error' }));
    });

    expect(consoleSpy).toHaveBeenCalledWith(
      'Failed to send Gmail sync failure notification:',
      expect.any(Error)
    );
    consoleSpy.mockRestore();
  });
});
