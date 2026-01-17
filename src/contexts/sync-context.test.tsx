import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act, waitFor } from '@testing-library/react'
import { SyncProvider, useSync } from './sync-context'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ReactNode } from 'react'

// Tauri APIのモック
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}))

const mockSyncMetadata = {
  sync_status: 'idle' as const,
  total_synced_count: 0,
  batch_size: 50,
}

const mockSyncProgress = {
  batch_number: 1,
  batch_size: 50,
  total_synced: 50,
  newly_saved: 45,
  status_message: 'Batch 1 complete: 45 new emails',
  is_complete: false,
}

describe('SyncContext', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // デフォルトではidleステータスを返す
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve(mockSyncMetadata)
      }
      return Promise.resolve(undefined)
    })
    vi.mocked(listen).mockResolvedValue(() => {})
  })

  const wrapper = ({ children }: { children: ReactNode }) => (
    <SyncProvider>{children}</SyncProvider>
  )

  it('provides initial sync state', async () => {
    const { result } = renderHook(() => useSync(), { wrapper })

    await waitFor(() => {
      expect(result.current.isSyncing).toBe(false)
    })
  })

  it('initializes with metadata from backend', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve({
          sync_status: 'idle' as const,
          total_synced_count: 100,
          batch_size: 50,
        })
      }
      return Promise.resolve(undefined)
    })

    const { result } = renderHook(() => useSync(), { wrapper })

    await waitFor(() => {
      expect(result.current.metadata).toBeDefined()
      expect(result.current.metadata?.total_synced_count).toBe(100)
    }, { timeout: 3000 })
  })

  it('handles stuck syncing status on initialization', async () => {
    let callCount = 0
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        callCount++
        if (callCount === 1) {
          // 初回: syncingステータス
          return Promise.resolve({ sync_status: 'syncing' as const, total_synced_count: 0, batch_size: 50 })
        }
        // 2回目以降: idleステータス
        return Promise.resolve({ sync_status: 'idle' as const, total_synced_count: 0, batch_size: 50 })
      }
      if (cmd === 'reset_sync_status') {
        return Promise.resolve(undefined)
      }
      return Promise.resolve(undefined)
    })

    renderHook(() => useSync(), { wrapper })

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('reset_sync_status')
    }, { timeout: 3000 })
  })

  it('starts sync successfully', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined)

    const { result } = renderHook(() => useSync(), { wrapper })

    await act(async () => {
      await result.current.startSync()
    })

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('start_sync')
      expect(result.current.isSyncing).toBe(true)
    })
  })

  it('cancels sync successfully', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined)

    const { result } = renderHook(() => useSync(), { wrapper })

    // まず同期を開始
    await act(async () => {
      await result.current.startSync()
    })

    // 同期をキャンセル
    await act(async () => {
      await result.current.cancelSync()
    })

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('cancel_sync')
    })
  })

  it('refreshes status', async () => {
    const updatedMetadata = {
      sync_status: 'idle',
      total_synced_count: 200,
      batch_size: 50,
    }

    vi.mocked(invoke).mockResolvedValue(updatedMetadata)

    const { result } = renderHook(() => useSync(), { wrapper })

    await act(async () => {
      await result.current.refreshStatus()
    })

    await waitFor(() => {
      expect(result.current.metadata?.total_synced_count).toBe(200)
    })
  })

  it('updates batch size', async () => {
    const { result } = renderHook(() => useSync(), { wrapper })

    await act(async () => {
      await result.current.updateBatchSize(100)
    })

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('update_batch_size', { batchSize: 100 })
    })
  })

  it('handles sync error', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve(mockSyncMetadata)
      }
      if (cmd === 'start_sync') {
        return Promise.reject(new Error('Sync failed'))
      }
      return Promise.resolve(undefined)
    })

    const { result } = renderHook(() => useSync(), { wrapper })

    await act(async () => {
      try {
        await result.current.startSync()
      } catch (error) {
        // エラーは期待される
      }
    })

    // エラー発生時、isSyncingはfalseに戻る（startSync内のcatch節でsetIsSyncing(false)が呼ばれる）
    expect(result.current.isSyncing).toBe(false)
  })

  it('throws error when used outside provider', () => {
    const originalError = console.error
    console.error = () => {}

    expect(() => {
      renderHook(() => useSync())
    }).toThrow('useSync must be used within SyncProvider')

    console.error = originalError
  })

  it('provides all required context values', async () => {
    const { result } = renderHook(() => useSync(), { wrapper })

    await waitFor(() => {
      expect(result.current).toHaveProperty('isSyncing')
      expect(result.current).toHaveProperty('progress')
      expect(result.current).toHaveProperty('metadata')
      expect(result.current).toHaveProperty('startSync')
      expect(result.current).toHaveProperty('cancelSync')
      expect(result.current).toHaveProperty('refreshStatus')
      expect(result.current).toHaveProperty('updateBatchSize')
    })

    expect(typeof result.current.startSync).toBe('function')
    expect(typeof result.current.cancelSync).toBe('function')
    expect(typeof result.current.refreshStatus).toBe('function')
    expect(typeof result.current.updateBatchSize).toBe('function')
  })

  it('handles refresh status error gracefully', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('Failed to fetch'))

    const { result } = renderHook(() => useSync(), { wrapper })

    await act(async () => {
      await result.current.refreshStatus()
    })

    // エラーが発生しても例外を投げない（コンソールエラーのみ）
    expect(result.current.metadata).toBeDefined()
  })

  it('sets isSyncing to true during sync', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined)

    const { result } = renderHook(() => useSync(), { wrapper })

    await act(async () => {
      await result.current.startSync()
    })

    await waitFor(() => {
      expect(result.current.isSyncing).toBe(true)
    })
  })

  it('updates metadata after successful operations', async () => {
    let metadataState = {
      sync_status: 'idle' as const,
      total_synced_count: 50,
      batch_size: 50,
    }

    vi.mocked(invoke).mockImplementation((cmd: string, args?: any) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve(metadataState)
      }
      if (cmd === 'update_batch_size') {
        // バッチサイズ更新をシミュレート
        metadataState = { ...metadataState, batch_size: args?.batchSize || 100 }
        return Promise.resolve(undefined)
      }
      return Promise.resolve(undefined)
    })

    const { result } = renderHook(() => useSync(), { wrapper })

    await waitFor(() => {
      expect(result.current.metadata?.total_synced_count).toBe(50)
    }, { timeout: 3000 })

    await act(async () => {
      await result.current.updateBatchSize(100)
    })

    await waitFor(() => {
      expect(result.current.metadata?.batch_size).toBe(100)
    }, { timeout: 3000 })
  })
})
