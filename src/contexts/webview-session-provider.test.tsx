import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { ReactNode } from 'react';
import { mockInvoke, mockListen } from '@/test/setup';
import { AmazonSessionProvider } from './amazon-session-provider';
import { SurugayaSessionProvider } from './surugaya-session-provider';
import { useAmazonSession } from './use-amazon-session';
import { useSurugayaSession } from './use-surugaya-session';
import { TASK_NAMES } from './batch-progress-types';

const { toastSuccessMock, toastErrorMock } = vi.hoisted(() => ({
  toastSuccessMock: vi.fn(),
  toastErrorMock: vi.fn(),
}));

vi.mock('@/lib/toast', () => ({
  toastSuccess: (...args: unknown[]) => toastSuccessMock(...args),
  toastError: (...args: unknown[]) => toastErrorMock(...args),
}));

type Listener = (e: { payload: unknown }) => void;

function captureListeners() {
  const listeners: Record<string, Listener> = {};
  mockListen.mockImplementation((event: string, cb: Listener) => {
    listeners[event] = cb;
    return Promise.resolve(() => {});
  });
  return listeners;
}

describe('WebviewSessionProvider', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(undefined);
    mockListen.mockResolvedValue(() => {});
  });

  const amazonWrapper = ({ children }: { children: ReactNode }) => (
    <AmazonSessionProvider>{children}</AmazonSessionProvider>
  );
  const surugayaWrapper = ({ children }: { children: ReactNode }) => (
    <SurugayaSessionProvider>{children}</SurugayaSessionProvider>
  );

  it('invokes the shop-specific commands', async () => {
    const { result } = renderHook(() => useAmazonSession(), {
      wrapper: amazonWrapper,
    });

    await act(async () => {
      await result.current.openLoginWindow();
      await result.current.startFetch();
      await result.current.startRefetchAll();
      await result.current.cancelFetch();
    });

    expect(mockInvoke).toHaveBeenCalledWith('open_amazon_login_window');
    expect(mockInvoke).toHaveBeenCalledWith('start_amazon_order_fetch', {
      forceRefetch: false,
    });
    expect(mockInvoke).toHaveBeenCalledWith('start_amazon_order_fetch', {
      forceRefetch: true,
    });
    expect(mockInvoke).toHaveBeenCalledWith('cancel_amazon_order_fetch');
  });

  it('resets isFetching when start fails', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('window not open'));
    const { result } = renderHook(() => useSurugayaSession(), {
      wrapper: surugayaWrapper,
    });

    await act(async () => {
      await expect(result.current.startFetch()).rejects.toThrow(
        'window not open'
      );
    });
    expect(result.current.isFetching).toBe(false);
    expect(mockInvoke).toHaveBeenCalledWith('start_surugaya_mypage_fetch', {
      forceRefetch: false,
    });
  });

  it('tracks progress and completion events', async () => {
    const listeners = captureListeners();
    const { result } = renderHook(() => useSurugayaSession(), {
      wrapper: surugayaWrapper,
    });
    await waitFor(() =>
      expect(listeners['surugaya:fetch_complete']).toBeDefined()
    );

    await act(async () => {
      await result.current.startFetch();
    });
    expect(result.current.isFetching).toBe(true);

    act(() => {
      listeners['surugaya:fetch_progress']({
        payload: { current: 1, total: 4, url: 'https://example.com/1' },
      });
    });
    expect(result.current.progress?.task_name).toBe(
      TASK_NAMES.SURUGAYA_MYPAGE_FETCH
    );
    expect(result.current.progress?.processed_count).toBe(1);
    expect(result.current.progress?.progress_percent).toBe(25);

    act(() => {
      listeners['surugaya:fetch_complete']({
        payload: { cancelled: false, error: null },
      });
    });
    expect(result.current.isFetching).toBe(false);
    expect(result.current.progress?.is_complete).toBe(true);
    expect(toastSuccessMock).toHaveBeenCalledWith(
      '駿河屋マイページ取得が完了しました'
    );
  });

  it('shows an error toast on failure and no toast on cancel', async () => {
    const listeners = captureListeners();
    renderHook(() => useAmazonSession(), { wrapper: amazonWrapper });
    await waitFor(() =>
      expect(listeners['amazon:fetch_complete']).toBeDefined()
    );

    act(() => {
      listeners['amazon:fetch_complete']({
        payload: { cancelled: false, error: 'boom' },
      });
    });
    expect(toastErrorMock).toHaveBeenCalledWith(
      'Amazon注文詳細取得に失敗しました',
      'boom'
    );

    act(() => {
      listeners['amazon:fetch_complete']({
        payload: { cancelled: true, error: null },
      });
    });
    expect(toastSuccessMock).not.toHaveBeenCalled();
  });
});
