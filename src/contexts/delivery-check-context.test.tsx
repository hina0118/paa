import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { DeliveryCheckProvider } from './delivery-check-provider';
import { useDeliveryCheck } from './use-delivery-check';
import { mockInvoke, mockListen } from '@/test/setup';
import { ReactNode } from 'react';
import {
  BATCH_PROGRESS_EVENT,
  TASK_NAMES,
  type BatchProgress,
} from './batch-progress-types';

const { toastSuccessMock, toastErrorMock, notifyMock, isAppWindowVisibleMock } =
  vi.hoisted(() => ({
    toastSuccessMock: vi.fn(),
    toastErrorMock: vi.fn(),
    notifyMock: vi
      .fn<
        Parameters<(title: string, body: string) => Promise<void>>,
        Promise<void>
      >()
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

describe('DeliveryCheckContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    isAppWindowVisibleMock.mockResolvedValue(true);
    notifyMock.mockResolvedValue(undefined);
    mockInvoke.mockResolvedValue(undefined);
    mockListen.mockResolvedValue(() => {});
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <DeliveryCheckProvider>{children}</DeliveryCheckProvider>
  );

  it('provides initial state', () => {
    const { result } = renderHook(() => useDeliveryCheck(), { wrapper });

    expect(result.current.isChecking).toBe(false);
    expect(result.current.progress).toBeNull();
  });

  it('provides all required context values', () => {
    const { result } = renderHook(() => useDeliveryCheck(), { wrapper });

    expect(result.current).toHaveProperty('isChecking');
    expect(result.current).toHaveProperty('progress');
    expect(result.current).toHaveProperty('startDeliveryCheck');
    expect(result.current).toHaveProperty('cancelDeliveryCheck');
    expect(typeof result.current.startDeliveryCheck).toBe('function');
    expect(typeof result.current.cancelDeliveryCheck).toBe('function');
  });

  it('throws error when used outside provider', () => {
    const consoleErrorSpy = vi
      .spyOn(console, 'error')
      .mockImplementation(() => {});

    try {
      expect(() => {
        renderHook(() => useDeliveryCheck());
      }).toThrow(
        'useDeliveryCheck must be used within a DeliveryCheckProvider'
      );
    } finally {
      consoleErrorSpy.mockRestore();
    }
  });

  it('startDeliveryCheck invokes start_delivery_check and sets isChecking to true', async () => {
    const { result } = renderHook(() => useDeliveryCheck(), { wrapper });

    await act(async () => {
      await result.current.startDeliveryCheck();
    });

    expect(mockInvoke).toHaveBeenCalledWith('start_delivery_check');
    expect(result.current.isChecking).toBe(true);
  });

  it('startDeliveryCheck resets isChecking to false on invoke failure', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('start failed'));

    const { result } = renderHook(() => useDeliveryCheck(), { wrapper });

    await act(async () => {
      try {
        await result.current.startDeliveryCheck();
      } catch {
        // エラーは期待される
      }
    });

    expect(result.current.isChecking).toBe(false);
  });

  it('cancelDeliveryCheck invokes cancel_delivery_check', async () => {
    const { result } = renderHook(() => useDeliveryCheck(), { wrapper });

    await act(async () => {
      await result.current.cancelDeliveryCheck();
    });

    expect(mockInvoke).toHaveBeenCalledWith('cancel_delivery_check');
  });

  it('cancelDeliveryCheck logs error and rethrows on failure', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    mockInvoke.mockRejectedValueOnce(new Error('cancel failed'));

    const { result } = renderHook(() => useDeliveryCheck(), { wrapper });

    await expect(
      act(async () => {
        await result.current.cancelDeliveryCheck();
      })
    ).rejects.toThrow('cancel failed');

    expect(consoleSpy).toHaveBeenCalledWith(
      'Failed to cancel delivery check:',
      expect.any(Error)
    );
    consoleSpy.mockRestore();
  });

  it('sets isChecking to false when batch-progress completes', async () => {
    let progressCallback:
      | ((e: { payload: BatchProgress }) => Promise<void>)
      | null = null;
    mockListen.mockImplementation((event: string, cb: (e: unknown) => void) => {
      if (event === BATCH_PROGRESS_EVENT) {
        progressCallback = cb as (e: {
          payload: BatchProgress;
        }) => Promise<void>;
      }
      return Promise.resolve(() => {});
    });

    const { result } = renderHook(() => useDeliveryCheck(), { wrapper });

    await waitFor(() => expect(progressCallback).not.toBeNull());

    // 配送チェック開始
    await act(async () => {
      await result.current.startDeliveryCheck();
    });
    expect(result.current.isChecking).toBe(true);

    // 完了イベント受信
    await act(async () => {
      await progressCallback?.({
        payload: {
          task_name: TASK_NAMES.DELIVERY_CHECK,
          batch_number: 1,
          batch_size: 10,
          total_items: 10,
          processed_count: 10,
          success_count: 8,
          failed_count: 2,
          progress_percent: 100,
          status_message: 'Done',
          is_complete: true,
        },
      });
    });

    await waitFor(() => {
      expect(result.current.isChecking).toBe(false);
    });
  });

  it('ignores batch-progress events for other tasks', async () => {
    let progressCallback:
      | ((e: { payload: BatchProgress }) => Promise<void>)
      | null = null;
    mockListen.mockImplementation((event: string, cb: (e: unknown) => void) => {
      if (event === BATCH_PROGRESS_EVENT) {
        progressCallback = cb as (e: {
          payload: BatchProgress;
        }) => Promise<void>;
      }
      return Promise.resolve(() => {});
    });

    const { result } = renderHook(() => useDeliveryCheck(), { wrapper });

    await waitFor(() => expect(progressCallback).not.toBeNull());

    await act(async () => {
      await result.current.startDeliveryCheck();
    });
    expect(result.current.isChecking).toBe(true);

    // 別タスクの完了イベント
    await act(async () => {
      await progressCallback?.({
        payload: {
          task_name: TASK_NAMES.GMAIL_SYNC,
          batch_number: 1,
          batch_size: 50,
          total_items: 100,
          processed_count: 100,
          success_count: 100,
          failed_count: 0,
          progress_percent: 100,
          status_message: 'Done',
          is_complete: true,
        },
      });
    });

    // isChecking は変わらない
    expect(result.current.isChecking).toBe(true);
  });

  it('shows toastSuccess on completion when window is visible', async () => {
    let progressCallback:
      | ((e: { payload: BatchProgress }) => Promise<void>)
      | null = null;
    mockListen.mockImplementation((event: string, cb: (e: unknown) => void) => {
      if (event === BATCH_PROGRESS_EVENT) {
        progressCallback = cb as (e: {
          payload: BatchProgress;
        }) => Promise<void>;
      }
      return Promise.resolve(() => {});
    });

    isAppWindowVisibleMock.mockResolvedValue(true);

    renderHook(() => useDeliveryCheck(), { wrapper });
    await waitFor(() => expect(progressCallback).not.toBeNull());

    await act(async () => {
      await progressCallback?.({
        payload: {
          task_name: TASK_NAMES.DELIVERY_CHECK,
          batch_number: 1,
          batch_size: 10,
          total_items: 10,
          processed_count: 10,
          success_count: 9,
          failed_count: 1,
          progress_percent: 100,
          status_message: 'Done',
          is_complete: true,
        },
      });
    });

    expect(toastSuccessMock).toHaveBeenCalledWith(
      '配送状況確認が完了しました',
      '成功: 9件、失敗: 1件'
    );
  });

  it('shows toastError on completion with error when window is visible', async () => {
    let progressCallback:
      | ((e: { payload: BatchProgress }) => Promise<void>)
      | null = null;
    mockListen.mockImplementation((event: string, cb: (e: unknown) => void) => {
      if (event === BATCH_PROGRESS_EVENT) {
        progressCallback = cb as (e: {
          payload: BatchProgress;
        }) => Promise<void>;
      }
      return Promise.resolve(() => {});
    });

    isAppWindowVisibleMock.mockResolvedValue(true);

    renderHook(() => useDeliveryCheck(), { wrapper });
    await waitFor(() => expect(progressCallback).not.toBeNull());

    await act(async () => {
      await progressCallback?.({
        payload: {
          task_name: TASK_NAMES.DELIVERY_CHECK,
          batch_number: 1,
          batch_size: 10,
          total_items: 10,
          processed_count: 10,
          success_count: 0,
          failed_count: 1,
          progress_percent: 100,
          status_message: 'Error',
          is_complete: true,
          error: 'delivery check error',
        },
      });
    });

    expect(toastErrorMock).toHaveBeenCalledWith(
      '配送状況確認に失敗しました',
      'delivery check error'
    );
  });

  it('sends notification on completion when window is not visible', async () => {
    let progressCallback:
      | ((e: { payload: BatchProgress }) => Promise<void>)
      | null = null;
    mockListen.mockImplementation((event: string, cb: (e: unknown) => void) => {
      if (event === BATCH_PROGRESS_EVENT) {
        progressCallback = cb as (e: {
          payload: BatchProgress;
        }) => Promise<void>;
      }
      return Promise.resolve(() => {});
    });

    isAppWindowVisibleMock.mockResolvedValue(false);

    renderHook(() => useDeliveryCheck(), { wrapper });
    await waitFor(() => expect(progressCallback).not.toBeNull());

    await act(async () => {
      await progressCallback?.({
        payload: {
          task_name: TASK_NAMES.DELIVERY_CHECK,
          batch_number: 1,
          batch_size: 10,
          total_items: 10,
          processed_count: 10,
          success_count: 5,
          failed_count: 0,
          progress_percent: 100,
          status_message: 'Done',
          is_complete: true,
        },
      });
    });

    expect(notifyMock).toHaveBeenCalledWith(
      '配送状況確認完了',
      '成功: 5件、失敗: 0件'
    );
  });
});
