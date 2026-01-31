import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { SyncProvider, useSync } from './sync-context';
import { mockInvoke, mockListen } from '@/test/setup';
import { ReactNode } from 'react';

const mockSyncMetadata = {
  sync_status: 'idle' as const,
  total_synced_count: 0,
  batch_size: 50,
};

const _mockSyncProgress = {
  batch_number: 1,
  batch_size: 50,
  total_synced: 50,
  newly_saved: 45,
  status_message: 'Batch 1 complete: 45 new emails',
  is_complete: false,
};

describe('SyncContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // デフォルトではidleステータスを返す
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve(mockSyncMetadata);
      }
      return Promise.resolve(undefined);
    });
    mockListen.mockResolvedValue(() => {});
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <SyncProvider>{children}</SyncProvider>
  );

  it('provides initial sync state', async () => {
    const { result } = renderHook(() => useSync(), { wrapper });

    await waitFor(() => {
      expect(result.current.isSyncing).toBe(false);
    });
  });

  it('initializes with metadata from backend', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve({
          sync_status: 'idle' as const,
          total_synced_count: 100,
          batch_size: 50,
        });
      }
      return Promise.resolve(undefined);
    });

    const { result } = renderHook(() => useSync(), { wrapper });

    await waitFor(
      () => {
        expect(result.current.metadata).toBeDefined();
        expect(result.current.metadata?.total_synced_count).toBe(100);
      },
      { timeout: 3000 }
    );
  });

  it('handles stuck syncing status on initialization', async () => {
    let callCount = 0;
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        callCount++;
        if (callCount === 1) {
          // 初回: syncingステータス
          return Promise.resolve({
            sync_status: 'syncing' as const,
            total_synced_count: 0,
            batch_size: 50,
          });
        }
        // 2回目以降: idleステータス
        return Promise.resolve({
          sync_status: 'idle' as const,
          total_synced_count: 0,
          batch_size: 50,
        });
      }
      if (cmd === 'reset_sync_status') {
        return Promise.resolve(undefined);
      }
      return Promise.resolve(undefined);
    });

    renderHook(() => useSync(), { wrapper });

    await waitFor(
      () => {
        expect(mockInvoke).toHaveBeenCalledWith('reset_sync_status');
      },
      { timeout: 3000 }
    );
  });

  it('handles reset_sync_status failure when stuck', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    let callCount = 0;
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        callCount++;
        if (callCount === 1) {
          return Promise.resolve({
            sync_status: 'syncing' as const,
            total_synced_count: 0,
            batch_size: 50,
          });
        }
        return Promise.resolve({
          sync_status: 'idle' as const,
          total_synced_count: 0,
          batch_size: 50,
        });
      }
      if (cmd === 'reset_sync_status') {
        return Promise.reject(new Error('Reset failed'));
      }
      return Promise.resolve(undefined);
    });

    renderHook(() => useSync(), { wrapper });

    await waitFor(
      () => {
        expect(consoleSpy).toHaveBeenCalledWith(
          'Failed to reset sync status:',
          expect.any(Error)
        );
      },
      { timeout: 3000 }
    );

    consoleSpy.mockRestore();
  });

  it('handles sync-progress event without is_complete (no refresh)', async () => {
    let progressCallback:
      | ((e: { payload: { is_complete: boolean } }) => void)
      | null = null;
    mockListen.mockImplementation((event: string, cb: (e: unknown) => void) => {
      if (event === 'sync-progress') {
        progressCallback = cb as (e: {
          payload: { is_complete: boolean };
        }) => void;
      }
      return Promise.resolve(() => {});
    });

    let getSyncStatusCallCount = 0;
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        getSyncStatusCallCount++;
        return Promise.resolve({
          sync_status: 'idle' as const,
          total_synced_count: 100,
          batch_size: 50,
        });
      }
      return Promise.resolve(undefined);
    });

    renderHook(() => useSync(), { wrapper });

    await waitFor(() => expect(progressCallback).not.toBeNull());

    const countBefore = getSyncStatusCallCount;
    await act(async () => {
      progressCallback?.({
        payload: {
          batch_number: 1,
          batch_size: 50,
          total_synced: 50,
          newly_saved: 45,
          status_message: 'In progress',
          is_complete: false,
        },
      });
    });

    // is_complete=false なので refreshStatus (get_sync_status) は呼ばれない
    expect(getSyncStatusCallCount).toBe(countBefore);
  });

  it('handles sync-progress event with is_complete', async () => {
    let progressCallback:
      | ((e: { payload: { is_complete: boolean } }) => void)
      | null = null;
    mockListen.mockImplementation((event: string, cb: (e: unknown) => void) => {
      if (event === 'sync-progress') {
        progressCallback = cb as (e: {
          payload: { is_complete: boolean };
        }) => void;
      }
      return Promise.resolve(() => {});
    });

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve({
          sync_status: 'idle' as const,
          total_synced_count: 100,
          batch_size: 50,
        });
      }
      return Promise.resolve(undefined);
    });

    const { result } = renderHook(() => useSync(), { wrapper });

    await waitFor(() => expect(progressCallback).not.toBeNull());

    await act(async () => {
      progressCallback?.({
        payload: {
          batch_number: 1,
          batch_size: 50,
          total_synced: 50,
          newly_saved: 45,
          status_message: 'Complete',
          is_complete: true,
        },
      });
    });

    expect(mockInvoke).toHaveBeenCalledWith('get_sync_status');
    await waitFor(() => {
      expect(result.current.isSyncing).toBe(false);
    });
  });

  it('starts sync successfully', async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { result } = renderHook(() => useSync(), { wrapper });

    await act(async () => {
      await result.current.startSync();
    });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('start_sync');
      expect(result.current.isSyncing).toBe(true);
    });
  });

  it('cancels sync successfully', async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { result } = renderHook(() => useSync(), { wrapper });

    // まず同期を開始
    await act(async () => {
      await result.current.startSync();
    });

    // 同期をキャンセル
    await act(async () => {
      await result.current.cancelSync();
    });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('cancel_sync');
    });
  });

  it('refreshes status', async () => {
    const updatedMetadata = {
      sync_status: 'idle',
      total_synced_count: 200,
      batch_size: 50,
    };

    mockInvoke.mockResolvedValue(updatedMetadata);

    const { result } = renderHook(() => useSync(), { wrapper });

    await act(async () => {
      await result.current.refreshStatus();
    });

    await waitFor(() => {
      expect(result.current.metadata?.total_synced_count).toBe(200);
    });
  });

  it('updates batch size', async () => {
    const { result } = renderHook(() => useSync(), { wrapper });

    await act(async () => {
      await result.current.updateBatchSize(100);
    });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('update_batch_size', {
        batchSize: 100,
      });
    });
  });

  it('handles sync error', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve(mockSyncMetadata);
      }
      if (cmd === 'start_sync') {
        return Promise.reject(new Error('Sync failed'));
      }
      return Promise.resolve(undefined);
    });

    const { result } = renderHook(() => useSync(), { wrapper });

    await act(async () => {
      try {
        await result.current.startSync();
      } catch {
        // エラーは期待される
      }
    });

    // エラー発生時、isSyncingはfalseに戻る（startSync内のcatch節でsetIsSyncing(false)が呼ばれる）
    expect(result.current.isSyncing).toBe(false);
  });

  it('throws error when used outside provider', () => {
    const originalError = console.error;
    console.error = () => {};

    expect(() => {
      renderHook(() => useSync());
    }).toThrow('useSync must be used within SyncProvider');

    console.error = originalError;
  });

  it('provides all required context values', async () => {
    const { result } = renderHook(() => useSync(), { wrapper });

    await waitFor(() => {
      expect(result.current).toHaveProperty('isSyncing');
      expect(result.current).toHaveProperty('progress');
      expect(result.current).toHaveProperty('metadata');
      expect(result.current).toHaveProperty('startSync');
      expect(result.current).toHaveProperty('cancelSync');
      expect(result.current).toHaveProperty('refreshStatus');
      expect(result.current).toHaveProperty('updateBatchSize');
    });

    expect(typeof result.current.startSync).toBe('function');
    expect(typeof result.current.cancelSync).toBe('function');
    expect(typeof result.current.refreshStatus).toBe('function');
    expect(typeof result.current.updateBatchSize).toBe('function');
  });

  it('handles refresh status error gracefully', async () => {
    mockInvoke.mockRejectedValue(new Error('Failed to fetch'));

    const { result } = renderHook(() => useSync(), { wrapper });

    await act(async () => {
      await result.current.refreshStatus();
    });

    // エラーが発生しても例外を投げない（コンソールエラーのみ）
    expect(result.current.metadata).toBeDefined();
  });

  it('sets isSyncing to true during sync', async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { result } = renderHook(() => useSync(), { wrapper });

    await act(async () => {
      await result.current.startSync();
    });

    await waitFor(() => {
      expect(result.current.isSyncing).toBe(true);
    });
  });

  it('updates max iterations', async () => {
    const { result } = renderHook(() => useSync(), { wrapper });

    await act(async () => {
      await result.current.updateMaxIterations(200);
    });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('update_max_iterations', {
        maxIterations: 200,
      });
    });
  });

  it('handles update max iterations error', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve(mockSyncMetadata);
      }
      if (cmd === 'update_max_iterations') {
        return Promise.reject(new Error('Update failed'));
      }
      return Promise.resolve(undefined);
    });

    const { result } = renderHook(() => useSync(), { wrapper });

    await act(async () => {
      try {
        await result.current.updateMaxIterations(200);
      } catch {
        // エラーは期待される
      }
    });

    expect(consoleSpy).toHaveBeenCalledWith(
      'Failed to update max iterations:',
      expect.any(Error)
    );
    consoleSpy.mockRestore();
  });

  it('handles cancel sync error and rethrows', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve(mockSyncMetadata);
      }
      if (cmd === 'cancel_sync') {
        return Promise.reject(new Error('Cancel failed'));
      }
      return Promise.resolve(undefined);
    });

    const { result } = renderHook(() => useSync(), { wrapper });

    await act(async () => {
      await result.current.startSync();
    });

    await expect(
      act(async () => {
        await result.current.cancelSync();
      })
    ).rejects.toThrow('Cancel failed');
  });

  it('updates metadata after successful operations', async () => {
    let metadataState = {
      sync_status: 'idle' as const,
      total_synced_count: 50,
      batch_size: 50,
    };

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    mockInvoke.mockImplementation((cmd: string, args?: any) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve(metadataState);
      }
      if (cmd === 'update_batch_size') {
        // バッチサイズ更新をシミュレート
        metadataState = {
          ...metadataState,
          batch_size: args?.batchSize || 100,
        };
        return Promise.resolve(undefined);
      }
      return Promise.resolve(undefined);
    });

    const { result } = renderHook(() => useSync(), { wrapper });

    await waitFor(
      () => {
        expect(result.current.metadata?.total_synced_count).toBe(50);
      },
      { timeout: 3000 }
    );

    await act(async () => {
      await result.current.updateBatchSize(100);
    });

    await waitFor(
      () => {
        expect(result.current.metadata?.batch_size).toBe(100);
      },
      { timeout: 3000 }
    );
  });
});
