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

/**
 * テスト用モックファクトリ。共通コマンドのデフォルトレスポンスを提供し、
 * 個別テストでは差分だけ上書きできる。
 * overrides の値が Error インスタンスの場合は Promise.reject を返す。
 */
const createMockInvoke =
  (overrides: Record<string, unknown> = {}) =>
  (cmd: string) => {
    if (Object.prototype.hasOwnProperty.call(overrides, cmd)) {
      const val = overrides[cmd];
      if (val instanceof Error) return Promise.reject(val);
      return Promise.resolve(val);
    }
    if (cmd === 'get_sync_status') return Promise.resolve(defaultSyncMetadata);
    if (cmd === 'get_parse_status')
      return Promise.resolve(defaultParseMetadata);
    if (cmd === 'get_gemini_config')
      return Promise.resolve({ batch_size: 10, delay_seconds: 10 });
    if (cmd === 'get_scheduler_config')
      return Promise.resolve({ interval_minutes: 1440, enabled: true });
    return Promise.resolve(null);
  };

describe('Settings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockImplementation(createMockInvoke());
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
    expect(div).toHaveClass('pt-0');
    expect(div).toHaveClass('pb-10');
  });

  it('applies heading styling', () => {
    renderWithProviders(<Settings />);
    const heading = screen.getByRole('heading', { name: /設定/i, level: 1 });
    expect(heading).toHaveClass('text-2xl');
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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_sync_status: { ...defaultSyncMetadata, batch_size: 75 },
          update_batch_size: undefined,
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_sync_status: { ...defaultSyncMetadata, batch_size: 0 },
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_sync_status: { ...defaultSyncMetadata, batch_size: -5 },
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_sync_status: { ...defaultSyncMetadata, batch_size: 75 },
          update_batch_size: new Error('Network error'),
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_sync_status: { ...defaultSyncMetadata, max_iterations: 200 },
          update_max_iterations: undefined,
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_sync_status: { ...defaultSyncMetadata, max_iterations: 0 },
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_sync_status: { ...defaultSyncMetadata, max_iterations: 200 },
          update_max_iterations: new Error('Server error'),
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_parse_status: { ...defaultParseMetadata, batch_size: 150 },
          update_parse_batch_size: undefined,
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_parse_status: { ...defaultParseMetadata, batch_size: 0 },
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_parse_status: { ...defaultParseMetadata, batch_size: 150 },
          update_parse_batch_size: new Error('Parse error'),
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_sync_status: {
            ...defaultSyncMetadata,
            max_results_per_page: 200,
          },
          update_max_results_per_page: undefined,
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_sync_status: {
            ...defaultSyncMetadata,
            max_results_per_page: 600,
          },
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_sync_status: { ...defaultSyncMetadata, timeout_minutes: 60 },
          update_timeout_minutes: undefined,
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_gemini_config: { batch_size: 20, delay_seconds: 10 },
          update_gemini_batch_size: undefined,
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_gemini_config: { batch_size: 100, delay_seconds: 10 },
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_gemini_config: { batch_size: 10, delay_seconds: 5 },
          update_gemini_delay_seconds: undefined,
        })
      );

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
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_gemini_config: { batch_size: 10, delay_seconds: 90 },
        })
      );

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

  // スケジューラ有効/無効更新テスト
  describe('handleSaveSchedulerEnabled', () => {
    it('saves scheduler enabled successfully', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation(
        createMockInvoke({ update_scheduler_enabled: undefined })
      );

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          document.getElementById('scheduler-enabled')
        ).toBeInTheDocument();
      });

      // チェックを外してからチェック状態を変更
      await user.click(document.getElementById('scheduler-enabled')!);

      await user.click(
        screen.getByRole('button', { name: 'スケジューラの有効/無効を保存' })
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('update_scheduler_enabled', {
          enabled: false,
        });
      });

      await waitFor(() => {
        expect(
          screen.getByText('スケジューラの有効/無効を更新しました')
        ).toBeInTheDocument();
      });
    });

    it('handles scheduler enabled update error', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation(
        createMockInvoke({
          update_scheduler_enabled: new Error('Scheduler error'),
        })
      );

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          document.getElementById('scheduler-enabled')
        ).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: 'スケジューラの有効/無効を保存' })
      );

      await waitFor(() => {
        expect(screen.getByText(/更新に失敗しました/)).toBeInTheDocument();
        expect(screen.getByText(/Scheduler error/)).toBeInTheDocument();
      });
    });
  });

  // スケジューラ実行間隔更新テスト
  describe('handleSaveSchedulerInterval', () => {
    it('saves scheduler interval successfully', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation(
        createMockInvoke({ update_scheduler_interval: undefined })
      );

      renderWithProviders(<Settings />);

      const input = document.getElementById('scheduler-interval')!;
      await waitFor(() => {
        expect(input).toBeInTheDocument();
        expect(input).toHaveValue(1440);
      });

      await user.clear(input);
      await user.type(input, '60');

      await waitFor(() => {
        expect(input).toHaveValue(60);
      });

      await user.click(
        screen.getByRole('button', { name: 'スケジューラの実行間隔を保存' })
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('update_scheduler_interval', {
          intervalMinutes: 60,
        });
      });

      await waitFor(() => {
        expect(
          screen.getByText('スケジューラの実行間隔を更新しました')
        ).toBeInTheDocument();
      });
    });

    it('shows validation error for out of range scheduler interval', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation(
        createMockInvoke({
          get_scheduler_config: { interval_minutes: 20000, enabled: true },
        })
      );

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          document.getElementById('scheduler-interval')
        ).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: 'スケジューラの実行間隔を保存' })
      );

      await waitFor(() => {
        expect(
          screen.getByText(
            '実行間隔は1〜10080分（7日）の範囲で入力してください'
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

  describe('setting updates', () => {
    const testSettingUpdate = async (config: {
      inputId: string;
      initialValue: number;
      newValue: string;
      buttonName: string;
      invokeCommand: string;
      invokePayload: Record<string, number>;
      successMessage: string;
    }) => {
      const user = userEvent.setup();

      mockInvoke.mockImplementation(
        createMockInvoke({
          update_batch_size: undefined,
          update_max_iterations: undefined,
          update_max_results_per_page: undefined,
          update_timeout_minutes: undefined,
          update_parse_batch_size: undefined,
          update_gemini_batch_size: undefined,
          update_gemini_delay_seconds: undefined,
        })
      );

      renderWithProviders(<Settings />);

      const input = document.getElementById(config.inputId)!;

      await waitFor(() => {
        expect(input).toHaveValue(config.initialValue);
      });

      await user.clear(input);
      await user.type(input, config.newValue);
      await waitFor(() => {
        expect(input).toHaveValue(Number(config.newValue));
      });

      await user.click(screen.getByRole('button', { name: config.buttonName }));
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          config.invokeCommand,
          config.invokePayload
        );
      });

      await waitFor(() => {
        expect(screen.getByText(config.successMessage)).toBeInTheDocument();
      });
    };

    it('updates batch size', async () => {
      await testSettingUpdate({
        inputId: 'batch-size',
        initialValue: 50,
        newValue: '60',
        buttonName: '同期バッチサイズを保存',
        invokeCommand: 'update_batch_size',
        invokePayload: { batchSize: 60 },
        successMessage: 'バッチサイズを更新しました',
      });
    });

    it('updates max iterations', async () => {
      await testSettingUpdate({
        inputId: 'max-iterations',
        initialValue: 100,
        newValue: '120',
        buttonName: '最大繰り返し回数を保存',
        invokeCommand: 'update_max_iterations',
        invokePayload: { maxIterations: 120 },
        successMessage: '最大繰り返し回数を更新しました',
      });
    });

    it('updates max results per page', async () => {
      await testSettingUpdate({
        inputId: 'max-results-per-page',
        initialValue: 100,
        newValue: '150',
        buttonName: '1ページあたり取得件数を保存',
        invokeCommand: 'update_max_results_per_page',
        invokePayload: { maxResultsPerPage: 150 },
        successMessage: '1ページあたり取得件数を更新しました',
      });
    });

    it('updates timeout minutes', async () => {
      await testSettingUpdate({
        inputId: 'timeout-minutes',
        initialValue: 30,
        newValue: '40',
        buttonName: '同期タイムアウトを保存',
        invokeCommand: 'update_timeout_minutes',
        invokePayload: { timeoutMinutes: 40 },
        successMessage: '同期タイムアウトを更新しました',
      });
    });

    it('updates parse batch size', async () => {
      await testSettingUpdate({
        inputId: 'parse-batch-size',
        initialValue: 100,
        newValue: '140',
        buttonName: 'パースバッチサイズを保存',
        invokeCommand: 'update_parse_batch_size',
        invokePayload: { batchSize: 140 },
        successMessage: 'パースバッチサイズを更新しました',
      });
    });

    it('updates gemini batch size', async () => {
      await testSettingUpdate({
        inputId: 'gemini-batch-size',
        initialValue: 10,
        newValue: '11',
        buttonName: '商品名パースのバッチサイズを保存',
        invokeCommand: 'update_gemini_batch_size',
        invokePayload: { batchSize: 11 },
        successMessage: '商品名パースのバッチサイズを更新しました',
      });
    });

    it('updates gemini delay seconds', async () => {
      await testSettingUpdate({
        inputId: 'gemini-delay-seconds',
        initialValue: 10,
        newValue: '5',
        buttonName: 'リクエスト間の待機秒数を保存',
        invokeCommand: 'update_gemini_delay_seconds',
        invokePayload: { delaySeconds: 5 },
        successMessage: 'リクエスト間の待機秒数を更新しました',
      });
    });
  });
});
