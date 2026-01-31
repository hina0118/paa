import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Settings } from './settings';
import { SyncProvider } from '@/contexts/sync-context';
import { ParseProvider } from '@/contexts/parse-context';
import { mockInvoke, mockListen } from '@/test/setup';

const renderWithProviders = (ui: React.ReactElement) => {
  return render(
    <SyncProvider>
      <ParseProvider>{ui}</ParseProvider>
    </SyncProvider>
  );
};

const defaultSyncMetadata = {
  batch_size: 50,
  max_iterations: 100,
  sync_status: 'idle',
  last_sync_started_at: null,
  last_sync_completed_at: null,
  total_synced_count: 0,
};

const defaultParseMetadata = {
  batch_size: 100,
  parse_status: 'idle',
  last_parse_started_at: null,
  last_parse_completed_at: null,
  last_error_message: null,
  total_parsed_count: 0,
};

describe('Settings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // セットアップのモックを上書き
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve(defaultSyncMetadata);
      }
      if (cmd === 'get_parse_status') {
        return Promise.resolve(defaultParseMetadata);
      }
      return Promise.resolve(null);
    });
    mockListen.mockResolvedValue(() => {});
  });

  it('renders settings heading', () => {
    renderWithProviders(<Settings />);
    expect(
      screen.getByRole('heading', { name: /設定/i, level: 1 })
    ).toBeInTheDocument();
  });

  it('renders with correct heading level', () => {
    renderWithProviders(<Settings />);
    const heading = screen.getByRole('heading', { name: /設定/i, level: 1 });
    expect(heading.tagName).toBe('H1');
  });

  it('applies container styling', () => {
    const { container } = renderWithProviders(<Settings />);
    const div = container.querySelector('.container');
    expect(div).toBeInTheDocument();
    expect(div).toHaveClass('mx-auto');
    expect(div).toHaveClass('py-10');
  });

  it('applies heading styling', () => {
    renderWithProviders(<Settings />);
    const heading = screen.getByRole('heading', { name: /設定/i, level: 1 });
    expect(heading).toHaveClass('text-3xl');
    expect(heading).toHaveClass('font-bold');
  });

  it('renders without errors', () => {
    expect(() => renderWithProviders(<Settings />)).not.toThrow();
  });

  // 初期値の表示テスト
  it('displays initial batch size from metadata', async () => {
    renderWithProviders(<Settings />);

    await waitFor(() => {
      const batchSizeInput = document.getElementById('batch-size');
      expect(batchSizeInput).toHaveValue(50);
    });
  });

  it('displays initial max iterations from metadata', async () => {
    renderWithProviders(<Settings />);

    await waitFor(() => {
      const maxIterationsInput = document.getElementById('max-iterations');
      expect(maxIterationsInput).toHaveValue(100);
    });
  });

  // バッチサイズ更新テスト
  describe('handleSaveBatchSize', () => {
    it('saves batch size successfully', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve({
            ...defaultSyncMetadata,
            batch_size: 75, // 初期値を75に
          });
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve(defaultParseMetadata);
        }
        if (cmd === 'update_batch_size') {
          return Promise.resolve(undefined);
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        const input = document.getElementById('batch-size');
        expect(input).toBeInTheDocument();
        expect(input).toHaveValue(75);
      });

      const saveButtons = screen.getAllByRole('button', { name: '保存' });
      await user.click(saveButtons[0]);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('update_batch_size', {
          batchSize: 75,
        });
      });

      await waitFor(() => {
        expect(
          screen.getByText('バッチサイズを更新しました')
        ).toBeInTheDocument();
      });
    });

    it('shows validation error for invalid batch size', async () => {
      const user = userEvent.setup();
      // 初期値を0にしてバリデーションエラーをテスト
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve({
            ...defaultSyncMetadata,
            batch_size: 0,
          });
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve(defaultParseMetadata);
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('batch-size')).toBeInTheDocument();
      });

      const saveButtons = screen.getAllByRole('button', { name: '保存' });
      await user.click(saveButtons[0]);

      await waitFor(() => {
        expect(
          screen.getByText('バッチサイズは1以上の整数を入力してください')
        ).toBeInTheDocument();
      });
    });

    it('shows validation error for negative batch size', async () => {
      const user = userEvent.setup();
      // 初期値を負の値にしてバリデーションエラーをテスト
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve({
            ...defaultSyncMetadata,
            batch_size: -5,
          });
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve(defaultParseMetadata);
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('batch-size')).toBeInTheDocument();
      });

      const saveButtons = screen.getAllByRole('button', { name: '保存' });
      await user.click(saveButtons[0]);

      await waitFor(() => {
        expect(
          screen.getByText('バッチサイズは1以上の整数を入力してください')
        ).toBeInTheDocument();
      });
    });

    it('handles batch size update error', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve({
            ...defaultSyncMetadata,
            batch_size: 75, // 初期値を75に
          });
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve(defaultParseMetadata);
        }
        if (cmd === 'update_batch_size') {
          return Promise.reject(new Error('Network error'));
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('batch-size')).toBeInTheDocument();
      });

      const saveButtons = screen.getAllByRole('button', { name: '保存' });
      await user.click(saveButtons[0]);

      await waitFor(() => {
        expect(
          screen.getByText(/更新に失敗しました.*Network error/)
        ).toBeInTheDocument();
      });
    });
  });

  // 最大繰り返し回数更新テスト
  describe('handleSaveMaxIterations', () => {
    it('saves max iterations successfully', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve({
            ...defaultSyncMetadata,
            max_iterations: 200, // 初期値を200に
          });
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve(defaultParseMetadata);
        }
        if (cmd === 'update_max_iterations') {
          return Promise.resolve(undefined);
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('max-iterations')).toBeInTheDocument();
      });

      const saveButtons = screen.getAllByRole('button', { name: '保存' });
      await user.click(saveButtons[1]);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('update_max_iterations', {
          maxIterations: 200,
        });
      });

      await waitFor(() => {
        expect(
          screen.getByText('最大繰り返し回数を更新しました')
        ).toBeInTheDocument();
      });
    });

    it('shows validation error for invalid max iterations', async () => {
      const user = userEvent.setup();
      // 初期値を0にしてバリデーションエラーをテスト
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve({
            ...defaultSyncMetadata,
            max_iterations: 0,
          });
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve(defaultParseMetadata);
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('max-iterations')).toBeInTheDocument();
      });

      const saveButtons = screen.getAllByRole('button', { name: '保存' });
      await user.click(saveButtons[1]);

      await waitFor(() => {
        expect(
          screen.getByText('最大繰り返し回数は1以上の整数を入力してください')
        ).toBeInTheDocument();
      });
    });

    it('handles max iterations update error', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve({
            ...defaultSyncMetadata,
            max_iterations: 200, // 初期値を200に
          });
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve(defaultParseMetadata);
        }
        if (cmd === 'update_max_iterations') {
          return Promise.reject(new Error('Server error'));
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('max-iterations')).toBeInTheDocument();
      });

      const saveButtons = screen.getAllByRole('button', { name: '保存' });
      await user.click(saveButtons[1]);

      await waitFor(() => {
        expect(
          screen.getByText(/更新に失敗しました.*Server error/)
        ).toBeInTheDocument();
      });
    });
  });

  // パースバッチサイズ更新テスト
  describe('handleSaveParseBatchSize', () => {
    it('saves parse batch size successfully', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve(defaultSyncMetadata);
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve({
            ...defaultParseMetadata,
            batch_size: 150, // 初期値を150に
          });
        }
        if (cmd === 'update_parse_batch_size') {
          return Promise.resolve(undefined);
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('parse-batch-size')).toBeInTheDocument();
      });

      const saveButtons = screen.getAllByRole('button', { name: '保存' });
      // 3番目の保存ボタン（パース設定）
      await user.click(saveButtons[2]);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('update_parse_batch_size', {
          batchSize: 150,
        });
      });

      await waitFor(() => {
        expect(
          screen.getByText('パースバッチサイズを更新しました')
        ).toBeInTheDocument();
      });
    });

    it('shows validation error for invalid parse batch size', async () => {
      const user = userEvent.setup();
      // 初期値を0にしてバリデーションエラーをテスト
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve(defaultSyncMetadata);
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve({
            ...defaultParseMetadata,
            batch_size: 0,
          });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('parse-batch-size')).toBeInTheDocument();
      });

      const saveButtons = screen.getAllByRole('button', { name: '保存' });
      await user.click(saveButtons[2]);

      await waitFor(() => {
        expect(
          screen.getByText('パースバッチサイズは1以上の整数を入力してください')
        ).toBeInTheDocument();
      });
    });

    it('handles parse batch size update error', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve(defaultSyncMetadata);
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve({
            ...defaultParseMetadata,
            batch_size: 150, // 初期値を150に
          });
        }
        if (cmd === 'update_parse_batch_size') {
          return Promise.reject(new Error('Parse error'));
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('parse-batch-size')).toBeInTheDocument();
      });

      const saveButtons = screen.getAllByRole('button', { name: '保存' });
      await user.click(saveButtons[2]);

      await waitFor(() => {
        expect(
          screen.getByText(/更新に失敗しました.*Parse error/)
        ).toBeInTheDocument();
      });
    });
  });

  // 最大取得件数の表示テスト
  it('displays calculated max fetch count', async () => {
    renderWithProviders(<Settings />);

    await waitFor(() => {
      // 50 * 100 = 5000（千の位にカンマなし）
      expect(screen.getByText(/5000 件/)).toBeInTheDocument();
    });
  });
});
