import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Toaster } from 'sonner';
import { Settings } from './settings';
import { SyncProvider } from '@/contexts/sync-provider';
import { ParseProvider } from '@/contexts/parse-provider';
import { mockInvoke, mockListen } from '@/test/setup';

const renderWithProviders = (ui: React.ReactElement) => {
  return render(
    <>
      <SyncProvider>
        <ParseProvider>{ui}</ParseProvider>
      </SyncProvider>
      <Toaster position="top-right" richColors />
    </>
  );
};

const defaultSyncMetadata = {
  batch_size: 50,
  max_iterations: 100,
  max_results_per_page: 100,
  timeout_minutes: 30,
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
      if (cmd === 'get_gemini_config') {
        return Promise.resolve({ batch_size: 10, delay_seconds: 10 });
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
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 10, delay_seconds: 10 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        const input = document.getElementById('batch-size');
        expect(input).toBeInTheDocument();
        expect(input).toHaveValue(75);
      });

      await user.click(
        screen.getByRole('button', { name: '同期バッチサイズを保存' })
      );

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
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 10, delay_seconds: 10 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('batch-size')).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: '同期バッチサイズを保存' })
      );

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
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 10, delay_seconds: 10 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('batch-size')).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: '同期バッチサイズを保存' })
      );

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
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 10, delay_seconds: 10 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('batch-size')).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: '同期バッチサイズを保存' })
      );

      await waitFor(() => {
        // toast が title/description 等に分割されても壊れないように別々に検証する
        expect(screen.getByText(/更新に失敗しました/)).toBeInTheDocument();
        expect(screen.getByText(/Network error/)).toBeInTheDocument();
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
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 10, delay_seconds: 10 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('max-iterations')).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: '最大繰り返し回数を保存' })
      );

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
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 10, delay_seconds: 10 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('max-iterations')).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: '最大繰り返し回数を保存' })
      );

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
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 10, delay_seconds: 10 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('max-iterations')).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: '最大繰り返し回数を保存' })
      );

      await waitFor(() => {
        // toast が title/description 等に分割されても壊れないように別々に検証する
        expect(screen.getByText(/更新に失敗しました/)).toBeInTheDocument();
        expect(screen.getByText(/Server error/)).toBeInTheDocument();
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
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 10, delay_seconds: 10 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        const input = document.getElementById('parse-batch-size');
        expect(input).toBeInTheDocument();
        expect(input).toHaveValue(150);
      });

      await user.click(
        screen.getByRole('button', { name: 'パースバッチサイズを保存' })
      );

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
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 10, delay_seconds: 10 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('parse-batch-size')).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: 'パースバッチサイズを保存' })
      );

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
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 10, delay_seconds: 10 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        const input = document.getElementById('parse-batch-size');
        expect(input).toBeInTheDocument();
        expect(input).toHaveValue(150);
      });

      await user.click(
        screen.getByRole('button', { name: 'パースバッチサイズを保存' })
      );

      await waitFor(() => {
        // toast が title/description 等に分割されても壊れないように別々に検証する
        expect(screen.getByText(/更新に失敗しました/)).toBeInTheDocument();
        expect(screen.getByText(/Parse error/)).toBeInTheDocument();
      });
    });
  });

  // 1ページあたり取得件数更新テスト
  describe('handleSaveMaxResultsPerPage', () => {
    it('saves max results per page successfully', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve({
            ...defaultSyncMetadata,
            max_results_per_page: 200,
          });
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve(defaultParseMetadata);
        }
        if (cmd === 'update_max_results_per_page') {
          return Promise.resolve(undefined);
        }
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 10, delay_seconds: 10 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          document.getElementById('max-results-per-page')
        ).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: '1ページあたり取得件数を保存' })
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('update_max_results_per_page', {
          maxResultsPerPage: 200,
        });
      });

      await waitFor(() => {
        expect(
          screen.getByText('1ページあたり取得件数を更新しました')
        ).toBeInTheDocument();
      });
    });

    it('shows validation error for out of range max results per page', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve({
            ...defaultSyncMetadata,
            max_results_per_page: 600,
          });
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve(defaultParseMetadata);
        }
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 10, delay_seconds: 10 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          document.getElementById('max-results-per-page')
        ).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: '1ページあたり取得件数を保存' })
      );

      await waitFor(() => {
        expect(
          screen.getByText(
            '1ページあたり取得件数は1〜500の範囲で入力してください'
          )
        ).toBeInTheDocument();
      });
    });
  });

  // 同期タイムアウト更新テスト
  describe('handleSaveTimeoutMinutes', () => {
    it('saves timeout minutes successfully', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve({
            ...defaultSyncMetadata,
            timeout_minutes: 60,
          });
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve(defaultParseMetadata);
        }
        if (cmd === 'update_timeout_minutes') {
          return Promise.resolve(undefined);
        }
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 10, delay_seconds: 10 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(document.getElementById('timeout-minutes')).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: '同期タイムアウトを保存' })
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('update_timeout_minutes', {
          timeoutMinutes: 60,
        });
      });

      await waitFor(() => {
        expect(
          screen.getByText('同期タイムアウトを更新しました')
        ).toBeInTheDocument();
      });
    });
  });

  // 商品名パースバッチサイズ更新テスト
  describe('handleSaveGeminiBatchSize', () => {
    it('saves gemini batch size successfully', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve(defaultSyncMetadata);
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve(defaultParseMetadata);
        }
        if (cmd === 'update_gemini_batch_size') {
          return Promise.resolve(undefined);
        }
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 20, delay_seconds: 10 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          document.getElementById('gemini-batch-size')
        ).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', {
          name: '商品名パースのバッチサイズを保存',
        })
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('update_gemini_batch_size', {
          batchSize: 20,
        });
      });

      await waitFor(() => {
        expect(
          screen.getByText('商品名パースのバッチサイズを更新しました')
        ).toBeInTheDocument();
      });
    });

    it('shows validation error for out of range gemini batch size', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve(defaultSyncMetadata);
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve(defaultParseMetadata);
        }
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 100, delay_seconds: 10 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          document.getElementById('gemini-batch-size')
        ).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', {
          name: '商品名パースのバッチサイズを保存',
        })
      );

      await waitFor(() => {
        expect(
          screen.getByText(
            '商品名パースのバッチサイズは1〜50の範囲で入力してください'
          )
        ).toBeInTheDocument();
      });
    });
  });

  // リクエスト間待機秒数更新テスト
  describe('handleSaveGeminiDelaySeconds', () => {
    it('saves gemini delay seconds successfully', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve(defaultSyncMetadata);
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve(defaultParseMetadata);
        }
        if (cmd === 'update_gemini_delay_seconds') {
          return Promise.resolve(undefined);
        }
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 10, delay_seconds: 5 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          document.getElementById('gemini-delay-seconds')
        ).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', {
          name: 'リクエスト間の待機秒数を保存',
        })
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('update_gemini_delay_seconds', {
          delaySeconds: 5,
        });
      });

      await waitFor(() => {
        expect(
          screen.getByText('リクエスト間の待機秒数を更新しました')
        ).toBeInTheDocument();
      });
    });

    it('shows validation error for out of range gemini delay seconds', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status') {
          return Promise.resolve(defaultSyncMetadata);
        }
        if (cmd === 'get_parse_status') {
          return Promise.resolve(defaultParseMetadata);
        }
        if (cmd === 'get_gemini_config') {
          return Promise.resolve({ batch_size: 10, delay_seconds: 90 });
        }
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          document.getElementById('gemini-delay-seconds')
        ).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', {
          name: 'リクエスト間の待機秒数を保存',
        })
      );

      await waitFor(() => {
        expect(
          screen.getByText(
            'リクエスト間の待機秒数は0〜60の範囲で入力してください'
          )
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
