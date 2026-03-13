import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { ReactNode } from 'react';
import { FullParsePipelineProvider } from './full-parse-pipeline-provider';
import { useFullParsePipeline } from './use-full-parse-pipeline';
import { mockInvoke, mockListen } from '@/test/setup';
import type { PipelineStep } from './full-parse-pipeline-context-value';

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

const wrapper = ({ children }: { children: ReactNode }) => (
  <FullParsePipelineProvider>{children}</FullParsePipelineProvider>
);

describe('FullParsePipelineContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    isAppWindowVisibleMock.mockResolvedValue(true);
    notifyMock.mockResolvedValue(undefined);
    mockInvoke.mockResolvedValue(undefined);
    mockListen.mockResolvedValue(() => {});
  });

  it('provides initial state', () => {
    const { result } = renderHook(() => useFullParsePipeline(), { wrapper });
    expect(result.current.isRunning).toBe(false);
    expect(result.current.currentStep).toBeNull();
    expect(typeof result.current.startPipeline).toBe('function');
  });

  it('throws when used outside provider', () => {
    const originalError = console.error;
    console.error = () => {};
    expect(() => renderHook(() => useFullParsePipeline())).toThrow(
      'useFullParsePipeline must be used within FullParsePipelineProvider'
    );
    console.error = originalError;
  });

  it('calls start_full_parse_pipeline on startPipeline', async () => {
    const { result } = renderHook(() => useFullParsePipeline(), { wrapper });

    await act(async () => {
      await result.current.startPipeline();
    });

    expect(mockInvoke).toHaveBeenCalledWith('start_full_parse_pipeline');
  });

  it('sets isRunning=true when startPipeline is called', async () => {
    // invoke が解決するまで保留させてステート確認
    let resolveInvoke!: () => void;
    mockInvoke.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          resolveInvoke = resolve;
        })
    );

    const { result } = renderHook(() => useFullParsePipeline(), { wrapper });

    act(() => {
      result.current.startPipeline();
    });

    expect(result.current.isRunning).toBe(true);

    // 後処理
    act(() => resolveInvoke());
  });

  it('resets isRunning=false and shows error toast on startPipeline failure', async () => {
    mockInvoke.mockRejectedValue(new Error('backend error'));

    const { result } = renderHook(() => useFullParsePipeline(), { wrapper });

    await act(async () => {
      try {
        await result.current.startPipeline();
      } catch {
        // エラーは期待される
      }
    });

    expect(result.current.isRunning).toBe(false);
    expect(toastErrorMock).toHaveBeenCalledWith(
      '一括パースの開始に失敗しました',
      'backend error'
    );
  });

  describe('full-parse:step_started イベント', () => {
    const setupStepListener = () => {
      const callbacks: ((e: { payload: PipelineStep }) => void)[] = [];
      mockListen.mockImplementation(
        (event: string, cb: (e: unknown) => void) => {
          if (event === 'full-parse:step_started') {
            callbacks.push(cb as (e: { payload: PipelineStep }) => void);
          }
          return Promise.resolve(() => {});
        }
      );
      return {
        hasCallbacks: () => callbacks.length > 0,
        fire: (step: PipelineStep) => {
          for (const cb of callbacks) cb({ payload: step });
        },
      };
    };

    it('updates currentStep on step_started event', async () => {
      const { hasCallbacks, fire } = setupStepListener();
      const { result } = renderHook(() => useFullParsePipeline(), { wrapper });

      await waitFor(() => expect(hasCallbacks()).toBe(true));

      act(() => fire('parse'));
      expect(result.current.currentStep).toBe('parse');
      expect(result.current.isRunning).toBe(true);

      act(() => fire('surugaya'));
      expect(result.current.currentStep).toBe('surugaya');

      act(() => fire('product_parse'));
      expect(result.current.currentStep).toBe('product_parse');

      act(() => fire('delivery_check'));
      expect(result.current.currentStep).toBe('delivery_check');
    });
  });

  describe('full-parse:complete イベント', () => {
    const setupCompleteListener = () => {
      const callbacks: (() => void)[] = [];
      mockListen.mockImplementation(
        (event: string, cb: (e: unknown) => void) => {
          if (event === 'full-parse:complete') {
            callbacks.push(cb as () => void);
          }
          return Promise.resolve(() => {});
        }
      );
      return {
        hasCallbacks: () => callbacks.length > 0,
        fire: () => {
          for (const cb of callbacks) cb();
        },
      };
    };

    it('resets isRunning and currentStep on complete', async () => {
      const { hasCallbacks, fire } = setupCompleteListener();
      const { result } = renderHook(() => useFullParsePipeline(), { wrapper });

      await waitFor(() => expect(hasCallbacks()).toBe(true));

      // まず実行中状態にする
      await act(async () => {
        await result.current.startPipeline();
      });

      await act(async () => {
        fire();
      });

      expect(result.current.isRunning).toBe(false);
      expect(result.current.currentStep).toBeNull();
    });

    it('shows toastSuccess on complete when window is visible', async () => {
      const { hasCallbacks, fire } = setupCompleteListener();
      isAppWindowVisibleMock.mockResolvedValue(true);

      renderHook(() => useFullParsePipeline(), { wrapper });
      await waitFor(() => expect(hasCallbacks()).toBe(true));

      await act(async () => {
        fire();
      });

      expect(toastSuccessMock).toHaveBeenCalledWith('一括パースが完了しました');
    });

    it('sends OS notification on complete when window is not visible', async () => {
      const { hasCallbacks, fire } = setupCompleteListener();
      isAppWindowVisibleMock.mockResolvedValue(false);

      renderHook(() => useFullParsePipeline(), { wrapper });
      await waitFor(() => expect(hasCallbacks()).toBe(true));

      await act(async () => {
        fire();
      });

      expect(notifyMock).toHaveBeenCalledWith(
        '一括パース完了',
        '全ステップが完了しました'
      );
    });
  });
});
