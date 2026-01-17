import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Sync } from './sync'
import { SyncProvider } from '@/contexts/sync-context'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

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

describe('Sync', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // デフォルトのモック実装を設定
    vi.mocked(invoke).mockResolvedValue(mockSyncMetadata)
    vi.mocked(listen).mockResolvedValue(() => {})
  })

  const renderWithProvider = async () => {
    const result = render(
      <SyncProvider>
        <Sync />
      </SyncProvider>
    )
    // 初期化が完了するのを待つ
    await waitFor(() => {
      expect(invoke).toHaveBeenCalled()
    })
    return result
  }

  it('renders sync heading', async () => {
    await renderWithProvider()
    expect(screen.getByRole('heading', { name: /Gmail同期/i })).toBeInTheDocument()
  })

  it('renders sync control card', async () => {
    await renderWithProvider()
    expect(screen.getByText('同期コントロール')).toBeInTheDocument()
    expect(screen.getByText('Gmail からメールを段階的に取得します')).toBeInTheDocument()
  })

  it('renders start sync button', async () => {
    await renderWithProvider()
    expect(screen.getByRole('button', { name: /同期を開始/i })).toBeInTheDocument()
  })

  it('renders setup instructions card', async () => {
    await renderWithProvider()
    expect(screen.getByText('初回セットアップ')).toBeInTheDocument()
    expect(screen.getByText(/Gmail APIを使用するには/i)).toBeInTheDocument()
  })

  it('renders status badge with correct initial text', async () => {
    await renderWithProvider()
    expect(screen.getByText('ステータス:')).toBeInTheDocument()
  })

  it('applies correct styling to main container', async () => {
    const { container } = await renderWithProvider()
    const mainDiv = container.querySelector('.container')
    expect(mainDiv).toBeInTheDocument()
    expect(mainDiv).toHaveClass('mx-auto')
    expect(mainDiv).toHaveClass('py-10')
    expect(mainDiv).toHaveClass('space-y-6')
  })

  it('renders sync statistics card when metadata is available', async () => {
    await renderWithProvider()
    expect(screen.getByText('同期統計')).toBeInTheDocument()
  })

  it('displays total synced count', async () => {
    await renderWithProvider()
    expect(screen.getByText('総取得件数:')).toBeInTheDocument()
  })

  it('displays batch size', async () => {
    await renderWithProvider()
    expect(screen.getByText('バッチサイズ:')).toBeInTheDocument()
  })

  it('displays initial authentication warning', async () => {
    await renderWithProvider()
    expect(screen.getByText('初回認証について')).toBeInTheDocument()
    expect(screen.getByText(/初回実行時は、ブラウザで認証画面/i)).toBeInTheDocument()
  })

  it('renders without errors', async () => {
    expect(async () => await renderWithProvider()).not.toThrow()
  })

  it('has accessible heading structure', async () => {
    await renderWithProvider()
    const headings = screen.getAllByRole('heading')
    expect(headings.length).toBeGreaterThan(0)

    const mainHeading = screen.getByRole('heading', { name: /Gmail同期/i })
    expect(mainHeading.tagName).toBe('H1')
  })

  it('setup instructions are in a card with blue styling', async () => {
    const { container } = await renderWithProvider()
    const setupCard = container.querySelector('.bg-blue-50')
    expect(setupCard).toBeInTheDocument()
  })

  it('calls startSync when start button is clicked', async () => {
    const user = userEvent.setup()
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve(mockSyncMetadata)
      }
      if (cmd === 'start_sync') {
        return Promise.resolve(undefined)
      }
      return Promise.resolve(undefined)
    })

    await renderWithProvider()
    const startButton = screen.getByRole('button', { name: /同期を開始/i })
    await user.click(startButton)

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('start_sync')
    })
  })

  it('displays error when startSync fails', async () => {
    const user = userEvent.setup()
    const errorMessage = 'Sync failed: connection error'

    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve(mockSyncMetadata)
      }
      if (cmd === 'start_sync') {
        return Promise.reject(new Error(errorMessage))
      }
      return Promise.resolve(undefined)
    })

    await renderWithProvider()
    const startButton = screen.getByRole('button', { name: /同期を開始/i })
    await user.click(startButton)

    await waitFor(() => {
      expect(screen.getByText('エラー')).toBeInTheDocument()
      expect(screen.getByText(errorMessage)).toBeInTheDocument()
    })
  })

  it('shows cancel button when syncing', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve({
          sync_status: 'syncing' as const,
          total_synced_count: 100,
          batch_size: 50,
        })
      }
      return Promise.resolve(undefined)
    })

    await renderWithProvider()

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /中止/i })).toBeInTheDocument()
    })
  })

  it('calls cancelSync when cancel button is clicked', async () => {
    const user = userEvent.setup()
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve({
          sync_status: 'syncing' as const,
          total_synced_count: 100,
          batch_size: 50,
        })
      }
      if (cmd === 'cancel_sync') {
        return Promise.resolve(undefined)
      }
      return Promise.resolve(undefined)
    })

    await renderWithProvider()

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /中止/i })).toBeInTheDocument()
    })

    const cancelButton = screen.getByRole('button', { name: /中止/i })
    await user.click(cancelButton)

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('cancel_sync')
    })
  })

  it('displays error when cancelSync fails', async () => {
    const user = userEvent.setup()
    const errorMessage = 'Cancel failed'

    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve({
          sync_status: 'syncing' as const,
          total_synced_count: 100,
          batch_size: 50,
        })
      }
      if (cmd === 'cancel_sync') {
        return Promise.reject(new Error(errorMessage))
      }
      return Promise.resolve(undefined)
    })

    await renderWithProvider()

    const cancelButton = await screen.findByRole('button', { name: /中止/i })
    await user.click(cancelButton)

    await waitFor(() => {
      expect(screen.getByText(errorMessage)).toBeInTheDocument()
    })
  })

  it('displays "waiting" status badge for idle state', async () => {
    vi.mocked(invoke).mockResolvedValue({
      sync_status: 'idle' as const,
      total_synced_count: 0,
      batch_size: 50,
    })

    await renderWithProvider()

    await waitFor(() => {
      expect(screen.getByText('待機中')).toBeInTheDocument()
    })
  })

  it('displays "syncing" status badge when syncing', async () => {
    vi.mocked(invoke).mockResolvedValue({
      sync_status: 'syncing' as const,
      total_synced_count: 100,
      batch_size: 50,
    })

    await renderWithProvider()

    await waitFor(() => {
      expect(screen.getByText('同期中')).toBeInTheDocument()
    })
  })

  it('displays "paused" status badge when paused', async () => {
    vi.mocked(invoke).mockResolvedValue({
      sync_status: 'paused' as const,
      total_synced_count: 100,
      batch_size: 50,
    })

    await renderWithProvider()

    await waitFor(() => {
      expect(screen.getByText('一時停止')).toBeInTheDocument()
    })
  })

  it('displays "error" status badge when error state', async () => {
    vi.mocked(invoke).mockResolvedValue({
      sync_status: 'error' as const,
      total_synced_count: 100,
      batch_size: 50,
    })

    await renderWithProvider()

    await waitFor(() => {
      expect(screen.getByText('エラー')).toBeInTheDocument()
    })
  })

  it('shows "resume sync" button text when paused', async () => {
    vi.mocked(invoke).mockResolvedValue({
      sync_status: 'paused' as const,
      total_synced_count: 100,
      batch_size: 50,
    })

    await renderWithProvider()

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /同期を再開/i })).toBeInTheDocument()
    })
  })

  it('displays oldest fetched date when available', async () => {
    const testDate = '2024-01-15T10:30:00Z'
    vi.mocked(invoke).mockResolvedValue({
      sync_status: 'idle' as const,
      total_synced_count: 100,
      batch_size: 50,
      oldest_fetched_date: testDate,
    })

    await renderWithProvider()

    await waitFor(() => {
      expect(screen.getByText('最古メール日付:')).toBeInTheDocument()
    })
  })

  it('displays last sync completed date when available', async () => {
    const testDate = '2024-01-15T10:30:00Z'
    vi.mocked(invoke).mockResolvedValue({
      sync_status: 'idle' as const,
      total_synced_count: 100,
      batch_size: 50,
      last_sync_completed_at: testDate,
    })

    await renderWithProvider()

    await waitFor(() => {
      expect(screen.getByText('最終同期:')).toBeInTheDocument()
    })
  })

  it('disables start button when syncing', async () => {
    vi.mocked(invoke).mockResolvedValue({
      sync_status: 'syncing' as const,
      total_synced_count: 100,
      batch_size: 50,
    })

    await renderWithProvider()

    await waitFor(() => {
      const button = screen.getByRole('button', { name: /同期中/i })
      expect(button).toBeDisabled()
    })
  })
})
