import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { ParseProvider } from './parse-provider';
import { useParse } from './use-parse';
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
    isAppWindowVisibleMock: vi.fn<[], Promise<boolean>>(() =>
      Promise.resolve(true)
    ),
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

const mockParseMetadata = {
  parse_status: 'idle' as const,
  total_parsed_count: 0,
  batch_size: 100,
};

describe('ParseContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    isAppWindowVisibleMock.mockResolvedValue(true);
    notifyMock.mockResolvedValue(undefined);
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_parse_status') {
        return Promise.resolve(mockParseMetadata);
      }
      if (cmd === 'has_gemini_api_key') {
        return Promise.resolve(false);
      }
      return Promise.resolve(undefined);
    });
    mockListen.mockResolvedValue(() => {});
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <ParseProvider>{children}</ParseProvider>
  );

  it('provides initial parse state', async () => {
    const { result } = renderHook(() => useParse(), { wrapper });

    await waitFor(() => {
      expect(result.current.isParsing).toBe(false);
    });
  });

  it('initializes with metadata from backend', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_parse_status') {
        return Promise.resolve({
          parse_status: 'idle' as const,
          total_parsed_count: 50,
          batch_size: 100,
        });
      }
      if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
      return Promise.resolve(undefined);
    });

    const { result } = renderHook(() => useParse(), { wrapper });

    await waitFor(
      () => {
        expect(result.current.metadata).toBeDefined();
        expect(result.current.metadata?.total_parsed_count).toBe(50);
      },
      { timeout: 3000 }
    );
  });

  it('sets isParsing true when backend returns running status', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_parse_status') {
        return Promise.resolve({
          parse_status: 'running' as const,
          total_parsed_count: 0,
          batch_size: 100,
        });
      }
      return Promise.resolve(undefined);
    });

    const { result } = renderHook(() => useParse(), { wrapper });

    await waitFor(
      () => {
        expect(result.current.isParsing).toBe(true);
        expect(result.current.metadata?.parse_status).toBe('running');
      },
      { timeout: 3000 }
    );
  });

  it('starts parse successfully', async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { result } = renderHook(() => useParse(), { wrapper });

    await act(async () => {
      await result.current.startParse();
    });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('start_batch_parse', {
        batchSize: undefined,
      });
      expect(result.current.isParsing).toBe(true);
    });
  });

  it('starts parse with custom batch size', async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { result } = renderHook(() => useParse(), { wrapper });

    await act(async () => {
      await result.current.startParse(200);
    });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('start_batch_parse', {
        batchSize: 200,
      });
    });
  });

  it('handles parse error', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_parse_status') {
        return Promise.resolve(mockParseMetadata);
      }
      if (cmd === 'start_batch_parse') {
        return Promise.reject(new Error('Parse failed'));
      }
      return Promise.resolve(undefined);
    });

    const { result } = renderHook(() => useParse(), { wrapper });

    await act(async () => {
      try {
        await result.current.startParse();
      } catch {
        // エラーは期待される
      }
    });

    // エラー発生時、isParsingはfalseに戻る
    expect(result.current.isParsing).toBe(false);
  });

  it('cancels parse successfully', async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { result } = renderHook(() => useParse(), { wrapper });

    // まずパースを開始
    await act(async () => {
      await result.current.startParse();
    });

    // パースをキャンセル
    await act(async () => {
      await result.current.cancelParse();
    });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('cancel_parse');
    });
  });

  it('handles cancel parse error', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_parse_status') {
        return Promise.resolve(mockParseMetadata);
      }
      if (cmd === 'cancel_parse') {
        return Promise.reject(new Error('Cancel failed'));
      }
      return Promise.resolve(undefined);
    });

    const { result } = renderHook(() => useParse(), { wrapper });

    await act(async () => {
      try {
        await result.current.cancelParse();
      } catch {
        // エラーは期待される
      }
    });

    expect(consoleSpy).toHaveBeenCalledWith(
      'Failed to cancel parse:',
      expect.any(Error)
    );
    consoleSpy.mockRestore();
  });

  it('updates batch size', async () => {
    const { result } = renderHook(() => useParse(), { wrapper });

    await act(async () => {
      await result.current.updateBatchSize(200);
    });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('update_parse_batch_size', {
        batchSize: 200,
      });
    });
  });

  it('handles update batch size error', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_parse_status') {
        return Promise.resolve(mockParseMetadata);
      }
      if (cmd === 'update_parse_batch_size') {
        return Promise.reject(new Error('Update failed'));
      }
      return Promise.resolve(undefined);
    });

    const { result } = renderHook(() => useParse(), { wrapper });

    await act(async () => {
      try {
        await result.current.updateBatchSize(200);
      } catch {
        // エラーは期待される
      }
    });

    expect(consoleSpy).toHaveBeenCalledWith(
      'Failed to update batch size:',
      expect.any(Error)
    );
    consoleSpy.mockRestore();
  });

  it('refreshes status', async () => {
    const updatedMetadata = {
      parse_status: 'idle' as const,
      total_parsed_count: 100,
      batch_size: 150,
    };

    mockInvoke.mockResolvedValue(updatedMetadata);

    const { result } = renderHook(() => useParse(), { wrapper });

    await act(async () => {
      await result.current.refreshStatus();
    });

    await waitFor(() => {
      expect(result.current.metadata?.total_parsed_count).toBe(100);
    });
  });

  it('handles refresh status error gracefully', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    mockInvoke.mockRejectedValue(new Error('Failed to fetch'));

    const { result } = renderHook(() => useParse(), { wrapper });

    await act(async () => {
      await result.current.refreshStatus();
    });

    // エラーが発生しても例外を投げない
    expect(consoleSpy).toHaveBeenCalledWith(
      'Failed to fetch parse status:',
      expect.any(Error)
    );
    consoleSpy.mockRestore();
  });

  it('throws error when used outside provider', () => {
    const originalError = console.error;
    console.error = () => {};

    expect(() => {
      renderHook(() => useParse());
    }).toThrow('useParse must be used within ParseProvider');

    console.error = originalError;
  });

  it('provides all required context values', async () => {
    const { result } = renderHook(() => useParse(), { wrapper });

    await waitFor(() => {
      expect(result.current).toHaveProperty('isParsing');
      expect(result.current).toHaveProperty('progress');
      expect(result.current).toHaveProperty('metadata');
      expect(result.current).toHaveProperty('startParse');
      expect(result.current).toHaveProperty('cancelParse');
      expect(result.current).toHaveProperty('refreshStatus');
      expect(result.current).toHaveProperty('updateBatchSize');
      expect(result.current).toHaveProperty('geminiApiKeyStatus');
      expect(result.current).toHaveProperty('hasGeminiApiKey');
      expect(result.current).toHaveProperty('refreshGeminiApiKeyStatus');
    });

    expect(typeof result.current.startParse).toBe('function');
    expect(typeof result.current.cancelParse).toBe('function');
    expect(typeof result.current.refreshStatus).toBe('function');
    expect(typeof result.current.updateBatchSize).toBe('function');
    expect(typeof result.current.refreshGeminiApiKeyStatus).toBe('function');
    expect(['checking', 'available', 'unavailable', 'error']).toContain(
      result.current.geminiApiKeyStatus
    );
  });

  it('sets isParsing to true during parse', async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { result } = renderHook(() => useParse(), { wrapper });

    await act(async () => {
      await result.current.startParse();
    });

    await waitFor(() => {
      expect(result.current.isParsing).toBe(true);
    });
  });

  it('updates isParsing based on running status', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_parse_status') {
        return Promise.resolve({
          parse_status: 'running' as const,
          total_parsed_count: 50,
          batch_size: 100,
        });
      }
      return Promise.resolve(undefined);
    });

    const { result } = renderHook(() => useParse(), { wrapper });

    await waitFor(
      () => {
        expect(result.current.isParsing).toBe(true);
      },
      { timeout: 3000 }
    );
  });

  it('handles batch-progress event without is_complete (no refresh)', async () => {
    let progressCallback: ((e: { payload: BatchProgress }) => void) | null =
      null;
    mockListen.mockImplementation((event: string, cb: (e: unknown) => void) => {
      if (event === BATCH_PROGRESS_EVENT) {
        progressCallback = cb as (e: { payload: BatchProgress }) => void;
      }
      return Promise.resolve(() => {});
    });

    let getParseStatusCallCount = 0;
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_parse_status') {
        getParseStatusCallCount++;
        return Promise.resolve({
          parse_status: 'idle' as const,
          total_parsed_count: 100,
          batch_size: 100,
        });
      }
      return Promise.resolve(undefined);
    });

    renderHook(() => useParse(), { wrapper });

    await waitFor(() => expect(progressCallback).not.toBeNull());

    const countBefore = getParseStatusCallCount;
    await act(async () => {
      progressCallback?.({
        payload: {
          task_name: TASK_NAMES.EMAIL_PARSE,
          batch_number: 1,
          batch_size: 100,
          total_items: 100,
          processed_count: 50,
          success_count: 48,
          failed_count: 2,
          progress_percent: 50,
          status_message: 'In progress',
          is_complete: false,
        },
      });
    });

    // is_complete=false なので refreshStatus は呼ばれない
    expect(getParseStatusCallCount).toBe(countBefore);
  });

  it('handles batch-progress event with is_complete', async () => {
    let progressCallback: ((e: { payload: BatchProgress }) => void) | null =
      null;
    mockListen.mockImplementation((event: string, cb: (e: unknown) => void) => {
      if (event === BATCH_PROGRESS_EVENT) {
        progressCallback = cb as (e: { payload: BatchProgress }) => void;
      }
      return Promise.resolve(() => {});
    });

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_parse_status') {
        return Promise.resolve({
          parse_status: 'idle' as const,
          total_parsed_count: 100,
          batch_size: 100,
        });
      }
      return Promise.resolve(undefined);
    });

    renderHook(() => useParse(), { wrapper });

    await waitFor(() => expect(progressCallback).not.toBeNull());

    await act(async () => {
      progressCallback?.({
        payload: {
          task_name: TASK_NAMES.EMAIL_PARSE,
          batch_number: 1,
          batch_size: 100,
          total_items: 100,
          processed_count: 100,
          success_count: 98,
          failed_count: 2,
          progress_percent: 100,
          status_message: 'Done',
          is_complete: true,
        },
      });
    });

    expect(mockInvoke).toHaveBeenCalledWith('get_parse_status');
  });

  it('shows toastSuccess on email parse completion when window is visible', async () => {
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

    renderHook(() => useParse(), { wrapper });
    await waitFor(() => expect(progressCallback).not.toBeNull());

    await act(async () => {
      await progressCallback?.({
        payload: {
          task_name: TASK_NAMES.EMAIL_PARSE,
          batch_number: 1,
          batch_size: 100,
          total_items: 100,
          processed_count: 100,
          success_count: 98,
          failed_count: 2,
          progress_percent: 100,
          status_message: 'Done',
          is_complete: true,
        },
      });
    });

    expect(toastSuccessMock).toHaveBeenCalledWith(
      'メールパースが完了しました',
      '成功: 98件、失敗: 2件'
    );
  });

  it('shows toastError on email parse completion with error when window is visible', async () => {
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

    renderHook(() => useParse(), { wrapper });
    await waitFor(() => expect(progressCallback).not.toBeNull());

    await act(async () => {
      await progressCallback?.({
        payload: {
          task_name: TASK_NAMES.EMAIL_PARSE,
          batch_number: 1,
          batch_size: 100,
          total_items: 100,
          processed_count: 100,
          success_count: 0,
          failed_count: 1,
          progress_percent: 100,
          status_message: 'Done',
          is_complete: true,
          error: 'boom',
        },
      });
    });

    expect(toastErrorMock).toHaveBeenCalledWith(
      'メールパースに失敗しました',
      'boom'
    );
  });

  it('sends notification on email parse completion when window is not visible', async () => {
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

    renderHook(() => useParse(), { wrapper });
    await waitFor(() => expect(progressCallback).not.toBeNull());

    await act(async () => {
      await progressCallback?.({
        payload: {
          task_name: TASK_NAMES.EMAIL_PARSE,
          batch_number: 1,
          batch_size: 100,
          total_items: 100,
          processed_count: 100,
          success_count: 1,
          failed_count: 0,
          progress_percent: 100,
          status_message: 'Done',
          is_complete: true,
        },
      });
    });

    expect(notifyMock).toHaveBeenCalledWith(
      'メールパース完了',
      '成功: 1件、失敗: 0件'
    );
  });

  it('sends failure notification on email parse completion when window is not visible', async () => {
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

    renderHook(() => useParse(), { wrapper });
    await waitFor(() => expect(progressCallback).not.toBeNull());

    await act(async () => {
      await progressCallback?.({
        payload: {
          task_name: TASK_NAMES.EMAIL_PARSE,
          batch_number: 1,
          batch_size: 100,
          total_items: 100,
          processed_count: 100,
          success_count: 0,
          failed_count: 1,
          progress_percent: 100,
          status_message: 'Failed',
          is_complete: true,
          error: 'some error occurred',
        },
      });
    });

    expect(notifyMock).toHaveBeenCalledWith(
      'メールパース失敗',
      'some error occurred'
    );
  });
  it('updates geminiApiKeyStatus to available/unavailable and handles error', async () => {
    // available
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_parse_status') return Promise.resolve(mockParseMetadata);
      if (cmd === 'has_gemini_api_key') return Promise.resolve(true);
      return Promise.resolve(undefined);
    });
    const { result, rerender } = renderHook(() => useParse(), { wrapper });
    await waitFor(() =>
      expect(result.current.geminiApiKeyStatus).toBe('available')
    );
    expect(result.current.hasGeminiApiKey).toBe(true);

    // unavailable
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_parse_status') return Promise.resolve(mockParseMetadata);
      if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
      return Promise.resolve(undefined);
    });
    await act(async () => {
      await result.current.refreshGeminiApiKeyStatus();
    });
    expect(result.current.geminiApiKeyStatus).toBe('unavailable');
    expect(result.current.hasGeminiApiKey).toBe(false);

    // error
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_parse_status') return Promise.resolve(mockParseMetadata);
      if (cmd === 'has_gemini_api_key')
        return Promise.reject(new Error('nope'));
      return Promise.resolve(undefined);
    });
    await act(async () => {
      await result.current.refreshGeminiApiKeyStatus();
    });
    expect(result.current.geminiApiKeyStatus).toBe('error');
    expect(result.current.hasGeminiApiKey).toBe(false);

    rerender();
  });
});
