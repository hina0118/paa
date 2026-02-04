import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Sync } from './sync';
import { SyncProvider } from '@/contexts/sync-provider';
import { mockInvoke, mockListen } from '@/test/setup';

const mockSyncMetadata = {
  sync_status: 'idle' as const,
  total_synced_count: 0,
  batch_size: 50,
};

describe('Sync', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // デフォルトのモック実装を設定
    mockInvoke.mockResolvedValue(mockSyncMetadata);
    mockListen.mockResolvedValue(() => {});
  });

  const renderWithProvider = async () => {
    const result = render(
      <SyncProvider>
        <Sync />
      </SyncProvider>
    );
    // 初期化が完了するのを待つ
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalled();
    });
    return result;
  };

  it('renders sync heading', async () => {
    await renderWithProvider();
    expect(
      screen.getByRole('heading', { name: /Gmail同期/i })
    ).toBeInTheDocument();
  });

  it('renders sync control card', async () => {
    await renderWithProvider();
    expect(screen.getByText('同期コントロール')).toBeInTheDocument();
    expect(
      screen.getByText('Gmail からメールを段階的に取得します')
    ).toBeInTheDocument();
  });

  it('renders start sync button', async () => {
    await renderWithProvider();
    expect(
      screen.getByRole('button', { name: /同期を開始/i })
    ).toBeInTheDocument();
  });

  it('renders setup instructions card', async () => {
    await renderWithProvider();
    expect(screen.getByText('初回セットアップ')).toBeInTheDocument();
    expect(screen.getByText(/Gmail APIを使用するには/i)).toBeInTheDocument();
  });

  it('renders status badge with correct initial text', async () => {
    await renderWithProvider();
    expect(screen.getByText('ステータス:')).toBeInTheDocument();
  });

  it('applies correct styling to main container', async () => {
    const { container } = await renderWithProvider();
    const mainDiv = container.querySelector('.container');
    expect(mainDiv).toBeInTheDocument();
    expect(mainDiv).toHaveClass('mx-auto');
    expect(mainDiv).toHaveClass('py-10');
    expect(mainDiv).toHaveClass('space-y-6');
  });

  it('renders sync statistics card when metadata is available', async () => {
    await renderWithProvider();
    expect(screen.getByText('同期統計')).toBeInTheDocument();
  });

  it('displays total synced count', async () => {
    await renderWithProvider();
    expect(screen.getByText('総取得件数:')).toBeInTheDocument();
  });

  it('displays batch size', async () => {
    await renderWithProvider();
    expect(
      await screen.findByText('バッチサイズ:', { timeout: 3000 })
    ).toBeInTheDocument();
  });

  it('displays initial authentication warning', async () => {
    await renderWithProvider();
    expect(screen.getByText('初回認証について')).toBeInTheDocument();
    expect(
      screen.getByText(/初回実行時は、ブラウザで認証画面/i)
    ).toBeInTheDocument();
  });

  it('renders without errors', async () => {
    expect(async () => await renderWithProvider()).not.toThrow();
  });

  it('has accessible heading structure', async () => {
    await renderWithProvider();
    const headings = screen.getAllByRole('heading');
    expect(headings.length).toBeGreaterThan(0);

    const mainHeading = screen.getByRole('heading', { name: /Gmail同期/i });
    expect(mainHeading.tagName).toBe('H1');
  });

  it('setup instructions are in a card with blue styling', async () => {
    const { container } = await renderWithProvider();
    const setupCard = container.querySelector('.bg-blue-50');
    expect(setupCard).toBeInTheDocument();
  });

  it('calls startSync when start button is clicked', async () => {
    const user = userEvent.setup();
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve(mockSyncMetadata);
      }
      if (cmd === 'start_sync') {
        return Promise.resolve(undefined);
      }
      return Promise.resolve(undefined);
    });

    await renderWithProvider();
    const startButton = screen.getByRole('button', { name: /同期を開始/i });
    await user.click(startButton);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('start_sync');
    });
  });

  it('displays error when startSync fails', async () => {
    const user = userEvent.setup();
    const errorMessage = 'Sync failed: connection error';

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve(mockSyncMetadata);
      }
      if (cmd === 'start_sync') {
        return Promise.reject(new Error(errorMessage));
      }
      return Promise.resolve(undefined);
    });

    await renderWithProvider();
    const startButton = screen.getByRole('button', { name: /同期を開始/i });
    await user.click(startButton);

    await waitFor(() => {
      // エラーメッセージが表示されることを確認（複数要素が存在する可能性があるためgetAllByTextを使用）
      const errorHeadings = screen.getAllByText('エラー');
      expect(errorHeadings.length).toBeGreaterThan(0);
      const errorMessages = screen.getAllByText(errorMessage);
      expect(errorMessages.length).toBeGreaterThan(0);
    });
  });

  it('displays 一時停止 when sync_status is paused', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve({
          sync_status: 'paused' as const,
          total_synced_count: 0,
          batch_size: 50,
        });
      }
      return Promise.resolve(undefined);
    });
    await renderWithProvider();
    await waitFor(() => {
      expect(screen.getByText('一時停止')).toBeInTheDocument();
    });
  });

  it('displays エラー when sync_status is error', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve({
          sync_status: 'error' as const,
          total_synced_count: 0,
          batch_size: 50,
        });
      }
      return Promise.resolve(undefined);
    });
    await renderWithProvider();
    await waitFor(() => {
      expect(screen.getByText('エラー')).toBeInTheDocument();
    });
  });

  it('displays 不明 when sync_status is unknown', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve({
          sync_status: 'unknown' as never,
          total_synced_count: 0,
          batch_size: 50,
        });
      }
      return Promise.resolve(undefined);
    });
    await renderWithProvider();
    await waitFor(() => {
      expect(screen.getByText('不明')).toBeInTheDocument();
    });
  });

  it('shows cancel button when syncing', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve({
          sync_status: 'syncing' as const,
          total_synced_count: 100,
          batch_size: 50,
        });
      }
      return Promise.resolve(undefined);
    });

    await renderWithProvider();

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /中止/i })).toBeInTheDocument();
    });
  });

  it('calls cancelSync when cancel button is clicked', async () => {
    const user = userEvent.setup();
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve({
          sync_status: 'syncing' as const,
          total_synced_count: 100,
          batch_size: 50,
        });
      }
      if (cmd === 'cancel_sync') {
        return Promise.resolve(undefined);
      }
      return Promise.resolve(undefined);
    });

    await renderWithProvider();

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /中止/i })).toBeInTheDocument();
    });

    const cancelButton = screen.getByRole('button', { name: /中止/i });
    await user.click(cancelButton);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('cancel_sync');
    });
  });

  it('displays error when cancelSync fails', async () => {
    const user = userEvent.setup();
    const errorMessage = 'Cancel failed';

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve({
          sync_status: 'syncing' as const,
          total_synced_count: 100,
          batch_size: 50,
        });
      }
      if (cmd === 'cancel_sync') {
        return Promise.reject(new Error(errorMessage));
      }
      return Promise.resolve(undefined);
    });

    await renderWithProvider();

    const cancelButton = await screen.findByRole('button', { name: /中止/i });
    await user.click(cancelButton);

    await waitFor(() => {
      // エラーメッセージが表示されることを確認（複数要素が存在する可能性があるためgetAllByTextを使用）
      const errorElements = screen.getAllByText(errorMessage);
      expect(errorElements.length).toBeGreaterThan(0);
    });
  });

  it('displays completion message when progress is complete without error', async () => {
    let progressCallback: ((e: { payload: unknown }) => void) | null = null;
    mockListen.mockImplementation((event: string, cb: (e: unknown) => void) => {
      if (event === 'batch-progress') {
        progressCallback = cb as (e: { payload: unknown }) => void;
      }
      return Promise.resolve(() => {});
    });

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve({
          sync_status: 'syncing' as const,
          total_synced_count: 100,
          batch_size: 50,
        });
      }
      return Promise.resolve(undefined);
    });

    await renderWithProvider();

    await act(async () => {
      progressCallback?.({
        payload: {
          task_name: 'メール同期',
          batch_number: 1,
          batch_size: 50,
          total_items: 100,
          processed_count: 100,
          success_count: 95,
          failed_count: 5,
          progress_percent: 100,
          status_message: 'Complete',
          is_complete: true,
          error: undefined,
        },
      });
    });

    await waitFor(() => {
      expect(screen.getByText('同期が完了しました')).toBeInTheDocument();
    });
  });

  it('displays "waiting" status badge for idle state', async () => {
    mockInvoke.mockResolvedValue({
      sync_status: 'idle' as const,
      total_synced_count: 0,
      batch_size: 50,
    });

    await renderWithProvider();

    await waitFor(() => {
      expect(screen.getByText('待機中')).toBeInTheDocument();
    });
  });

  it('displays "syncing" status badge when syncing', async () => {
    mockInvoke.mockResolvedValue({
      sync_status: 'syncing' as const,
      total_synced_count: 100,
      batch_size: 50,
    });

    await renderWithProvider();

    await waitFor(() => {
      expect(screen.getByText('同期中')).toBeInTheDocument();
    });
  });

  it('displays "paused" status badge when paused', async () => {
    mockInvoke.mockResolvedValue({
      sync_status: 'paused' as const,
      total_synced_count: 100,
      batch_size: 50,
    });

    await renderWithProvider();

    await waitFor(() => {
      expect(screen.getByText('一時停止')).toBeInTheDocument();
    });
  });

  it('displays "error" status badge when error state', async () => {
    mockInvoke.mockResolvedValue({
      sync_status: 'error' as const,
      total_synced_count: 100,
      batch_size: 50,
    });

    await renderWithProvider();

    await waitFor(() => {
      expect(screen.getByText('エラー')).toBeInTheDocument();
    });
  });

  it('shows "resume sync" button text when paused', async () => {
    mockInvoke.mockResolvedValue({
      sync_status: 'paused' as const,
      total_synced_count: 100,
      batch_size: 50,
    });

    await renderWithProvider();

    await waitFor(() => {
      expect(
        screen.getByRole('button', { name: /同期を再開/i })
      ).toBeInTheDocument();
    });
  });

  it('displays oldest fetched date when available', async () => {
    const testDate = '2024-01-15T10:30:00Z';
    mockInvoke.mockResolvedValue({
      sync_status: 'idle' as const,
      total_synced_count: 100,
      batch_size: 50,
      oldest_fetched_date: testDate,
    });

    await renderWithProvider();

    await waitFor(() => {
      expect(screen.getByText('最古メール日付:')).toBeInTheDocument();
    });
  });

  it('displays last sync completed date when available', async () => {
    const testDate = '2024-01-15T10:30:00Z';
    mockInvoke.mockResolvedValue({
      sync_status: 'idle' as const,
      total_synced_count: 100,
      batch_size: 50,
      last_sync_completed_at: testDate,
    });

    await renderWithProvider();

    await waitFor(() => {
      expect(screen.getByText('最終同期:')).toBeInTheDocument();
    });
  });

  it('disables start button when syncing', async () => {
    mockInvoke.mockResolvedValue({
      sync_status: 'syncing' as const,
      total_synced_count: 100,
      batch_size: 50,
    });

    await renderWithProvider();

    await waitFor(() => {
      const button = screen.getByRole('button', { name: /同期中/i });
      expect(button).toBeDisabled();
    });
  });

  // handleResetSyncDate関数のテスト
  describe('handleResetSyncDate', () => {
    it('shows reset sync date button when not syncing', async () => {
      await renderWithProvider();

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: /同期日時をリセット/i })
        ).toBeInTheDocument();
      });
    });

    it('does not show reset button when syncing', async () => {
      mockInvoke.mockResolvedValue({
        sync_status: 'syncing' as const,
        total_synced_count: 100,
        batch_size: 50,
      });

      await renderWithProvider();

      await waitFor(() => {
        expect(
          screen.queryByRole('button', { name: /同期日時をリセット/i })
        ).not.toBeInTheDocument();
      });
    });

    it('calls reset_sync_date when confirmed', async () => {
      const user = userEvent.setup();
      // window.confirmをモック
      const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve(mockSyncMetadata);
        }
        if (cmd === 'reset_sync_date') {
          return Promise.resolve(undefined);
        }
        return Promise.resolve(undefined);
      });

      await renderWithProvider();

      const resetButton = screen.getByRole('button', {
        name: /同期日時をリセット/i,
      });
      await user.click(resetButton);

      await waitFor(() => {
        expect(confirmSpy).toHaveBeenCalled();
        expect(mockInvoke).toHaveBeenCalledWith('reset_sync_date');
      });

      confirmSpy.mockRestore();
    });

    it('does not call reset_sync_date when cancelled', async () => {
      const user = userEvent.setup();
      // window.confirmをモックしてfalseを返す
      const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve(mockSyncMetadata);
        }
        return Promise.resolve(undefined);
      });

      await renderWithProvider();

      const resetButton = screen.getByRole('button', {
        name: /同期日時をリセット/i,
      });
      await user.click(resetButton);

      // confirmは呼ばれるが、reset_sync_dateは呼ばれない
      expect(confirmSpy).toHaveBeenCalled();
      expect(mockInvoke).not.toHaveBeenCalledWith('reset_sync_date');

      confirmSpy.mockRestore();
    });

    it('displays success message after reset', async () => {
      const user = userEvent.setup();
      const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve(mockSyncMetadata);
        }
        if (cmd === 'reset_sync_date') {
          return Promise.resolve(undefined);
        }
        return Promise.resolve(undefined);
      });

      await renderWithProvider();

      const resetButton = screen.getByRole('button', {
        name: /同期日時をリセット/i,
      });
      await user.click(resetButton);

      await waitFor(() => {
        expect(
          screen.getByText(
            '同期日時をリセットしました。次回の同期から最新のメールが取得されます。'
          )
        ).toBeInTheDocument();
      });

      confirmSpy.mockRestore();
    });

    it('displays error message when reset fails', async () => {
      const user = userEvent.setup();
      const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
      const errorMessage = 'Reset failed: database error';

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve(mockSyncMetadata);
        }
        if (cmd === 'reset_sync_date') {
          return Promise.reject(new Error(errorMessage));
        }
        return Promise.resolve(undefined);
      });

      await renderWithProvider();

      const resetButton = screen.getByRole('button', {
        name: /同期日時をリセット/i,
      });
      await user.click(resetButton);

      await waitFor(() => {
        // エラーメッセージが複数箇所に表示される可能性があるため、getAllByTextを使用
        const errorElements = screen.getAllByText(errorMessage);
        expect(errorElements.length).toBeGreaterThan(0);
      });

      confirmSpy.mockRestore();
    });
  });
});
