import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { ParseProvider, useParse } from './parse-context';
import { mockInvoke, mockListen } from '@/test/setup';
import { ReactNode } from 'react';

const mockParseMetadata = {
  parse_status: 'idle' as const,
  total_parsed_count: 0,
  batch_size: 100,
};

describe('ParseContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_parse_status') {
        return Promise.resolve(mockParseMetadata);
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
    });

    expect(typeof result.current.startParse).toBe('function');
    expect(typeof result.current.cancelParse).toBe('function');
    expect(typeof result.current.refreshStatus).toBe('function');
    expect(typeof result.current.updateBatchSize).toBe('function');
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
});
