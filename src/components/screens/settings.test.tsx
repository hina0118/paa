import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Settings } from './settings';
import { SyncProvider } from '@/contexts/sync-provider';
import { ParseProvider } from '@/contexts/parse-provider';
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
      if (cmd === 'has_gemini_api_key') {
        return Promise.resolve(false);
      }
      if (cmd === 'is_google_search_configured') {
        return Promise.resolve(false);
      }
      if (cmd === 'has_gmail_oauth_credentials') {
        return Promise.resolve(false);
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
          screen.getByText(/更新に失敗しました.*Parse error/)
        ).toBeInTheDocument();
      });
    });
  });

  // Gemini API キー保存/削除テスト
  describe('handleSaveGeminiApiKey / handleDeleteGeminiApiKey', () => {
    it('saves Gemini API key successfully and clears input', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'save_gemini_api_key') return Promise.resolve(undefined);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      // Use ID selector to distinguish from SerpApi API key input
      const apiKeyInput = document.getElementById(
        'gemini-api-key'
      ) as HTMLInputElement;
      await user.type(apiKeyInput, 'test-api-key-123');

      await user.click(
        screen.getByRole('button', { name: 'Gemini APIキーを保存' })
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('save_gemini_api_key', {
          apiKey: 'test-api-key-123',
        });
      });

      await waitFor(() => {
        expect(
          screen.getByText(/Gemini APIキーを保存しました/)
        ).toBeInTheDocument();
      });

      await waitFor(() => {
        expect(apiKeyInput).toHaveValue('');
      });
    });

    it('calls refreshGeminiApiKeyStatus after save', async () => {
      const user = userEvent.setup();
      const invokeCalls: string[] = [];
      mockInvoke.mockImplementation((cmd: string) => {
        invokeCalls.push(cmd);
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'save_gemini_api_key') return Promise.resolve(undefined);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      // Use ID selector to distinguish from SerpApi API key input
      const apiKeyInput = document.getElementById(
        'gemini-api-key'
      ) as HTMLInputElement;
      await user.type(apiKeyInput, 'key');
      await user.click(
        screen.getByRole('button', { name: 'Gemini APIキーを保存' })
      );

      await waitFor(() => {
        expect(invokeCalls).toContain('save_gemini_api_key');
        const saveIdx = invokeCalls.indexOf('save_gemini_api_key');
        const hasKeyAfterSave = invokeCalls
          .slice(saveIdx + 1)
          .includes('has_gemini_api_key');
        expect(hasKeyAfterSave).toBe(true);
      });
    });

    it('shows error message when save fails', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'save_gemini_api_key')
          return Promise.reject(new Error('Save failed'));
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      // Use ID selector to distinguish from SerpApi API key input
      const apiKeyInput = document.getElementById(
        'gemini-api-key'
      ) as HTMLInputElement;
      await user.type(apiKeyInput, 'key');
      await user.click(
        screen.getByRole('button', { name: 'Gemini APIキーを保存' })
      );

      await waitFor(() => {
        expect(
          screen.getByText(/保存に失敗しました.*Save failed/)
        ).toBeInTheDocument();
      });
    });

    it('shows validation error when API key is empty', async () => {
      const user = userEvent.setup();
      renderWithProviders(<Settings />);

      await user.click(
        screen.getByRole('button', { name: 'Gemini APIキーを保存' })
      );

      await waitFor(() => {
        expect(
          screen.getByText('APIキーを入力してください')
        ).toBeInTheDocument();
      });
    });

    it('deletes Gemini API key when confirm is accepted', async () => {
      const user = userEvent.setup();
      vi.stubGlobal(
        'confirm',
        vi.fn(() => true)
      );
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(true);
        if (cmd === 'delete_gemini_api_key') return Promise.resolve(undefined);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: 'Gemini APIキーを削除' })
        ).toBeInTheDocument();
      });
      await user.click(
        screen.getByRole('button', { name: 'Gemini APIキーを削除' })
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('delete_gemini_api_key');
      });

      await waitFor(() => {
        expect(
          screen.getByText('Gemini APIキーを削除しました')
        ).toBeInTheDocument();
      });

      vi.unstubAllGlobals();
    });

    it('does not delete when confirm is cancelled', async () => {
      const user = userEvent.setup();
      vi.stubGlobal(
        'confirm',
        vi.fn(() => false)
      );
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(true);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: 'Gemini APIキーを削除' })
        ).toBeInTheDocument();
      });
      await user.click(
        screen.getByRole('button', { name: 'Gemini APIキーを削除' })
      );

      expect(mockInvoke).not.toHaveBeenCalledWith('delete_gemini_api_key');
      vi.unstubAllGlobals();
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

  // geminiApiKeyStatus / serpApiStatus 状態表示テスト
  describe('geminiApiKeyStatus and serpApiStatus state display', () => {
    it('displays checking state for Gemini API key status', async () => {
      let resolveGemini: (value: boolean) => void;
      const geminiPromise = new Promise<boolean>((resolve) => {
        resolveGemini = resolve;
      });
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return geminiPromise;
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      // 確認中状態で「APIキーの状態を確認中...」が表示される（SerpApiは即解決するため1件）
      await waitFor(() => {
        expect(
          screen.getByText('APIキーの状態を確認中...')
        ).toBeInTheDocument();
      });

      resolveGemini!(false);
    });

    it('displays checking state for SerpApi status', async () => {
      let resolveSerp: (value: boolean) => void;
      const serpPromise = new Promise<boolean>((resolve) => {
        resolveSerp = resolve;
      });
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured') return serpPromise;
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      // 確認中状態で「APIキーの状態を確認中...」が表示される（Geminiは即解決するため1件）
      await waitFor(() => {
        expect(
          screen.getByText('APIキーの状態を確認中...')
        ).toBeInTheDocument();
      });

      resolveSerp!(false);
    });

    it('displays error state for Gemini when backend is not running', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key')
          return Promise.reject(new Error('Backend not running'));
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByText(
            'APIキーの状態を取得できません（バックエンド未起動の可能性）'
          )
        ).toBeInTheDocument();
      });
    });

    it('displays error state for SerpApi when backend is not running', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.reject(new Error('Backend not running'));
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByText(
            'APIキーの状態を取得できません（バックエンド未起動の可能性）'
          )
        ).toBeInTheDocument();
      });
    });

    it('shows placeholder "********" when Gemini API key is available', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(true);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        const input = document.getElementById('gemini-api-key');
        expect(input).toHaveAttribute('placeholder', '********');
      });
    });

    it('shows placeholder "APIキーを入力" when Gemini API key is unavailable', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        const input = document.getElementById('gemini-api-key');
        expect(input).toHaveAttribute('placeholder', 'APIキーを入力');
      });
    });

    it('shows placeholder "********" when SerpApi is available', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured') return Promise.resolve(true);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        const input = document.getElementById('serpapi-key');
        expect(input).toHaveAttribute('placeholder', '********');
      });
    });

    it('shows placeholder "APIキーを入力" when SerpApi is unavailable', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        const input = document.getElementById('serpapi-key');
        expect(input).toHaveAttribute('placeholder', 'APIキーを入力');
      });
    });

    it('shows delete button when Gemini API key is available', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(true);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: 'Gemini APIキーを削除' })
        ).toBeInTheDocument();
      });
    });

    it('hides delete button when Gemini API key is unavailable', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        const input = document.getElementById('gemini-api-key');
        expect(input).toHaveAttribute('placeholder', 'APIキーを入力');
      });
      expect(
        screen.queryByRole('button', { name: 'Gemini APIキーを削除' })
      ).not.toBeInTheDocument();
    });

    it('hides delete button when Gemini API key status is error', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key')
          return Promise.reject(new Error('Backend error'));
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByText(
            'APIキーの状態を取得できません（バックエンド未起動の可能性）'
          )
        ).toBeInTheDocument();
      });
      expect(
        screen.queryByRole('button', { name: 'Gemini APIキーを削除' })
      ).not.toBeInTheDocument();
    });

    it('shows delete button when SerpApi is available', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured') return Promise.resolve(true);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: 'SerpApi APIキーを削除' })
        ).toBeInTheDocument();
      });
    });

    it('hides delete button when SerpApi is unavailable', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        const input = document.getElementById('serpapi-key');
        expect(input).toHaveAttribute('placeholder', 'APIキーを入力');
      });
      expect(
        screen.queryByRole('button', { name: 'SerpApi APIキーを削除' })
      ).not.toBeInTheDocument();
    });

    it('hides delete button when SerpApi status is error', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.reject(new Error('Backend error'));
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByText(
            'APIキーの状態を取得できません（バックエンド未起動の可能性）'
          )
        ).toBeInTheDocument();
      });
      expect(
        screen.queryByRole('button', { name: 'SerpApi APIキーを削除' })
      ).not.toBeInTheDocument();
    });
  });

  // SerpApi 設定カード表示テスト
  it('renders SerpApi settings card', async () => {
    renderWithProviders(<Settings />);
    await waitFor(() => {
      expect(
        screen.getByRole('heading', { name: /SerpApi/ })
      ).toBeInTheDocument();
    });
  });

  // SerpApi API キー保存/削除テスト
  describe('handleSaveSerpApiKey / handleDeleteSerpApiKey', () => {
    it('saves SerpApi API key successfully', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'save_google_search_api_key')
          return Promise.resolve(undefined);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      const apiKeyInput = document.getElementById(
        'serpapi-key'
      ) as HTMLInputElement;
      await user.type(apiKeyInput, 'serp-api-key-456');

      await user.click(
        screen.getByRole('button', { name: 'SerpApi APIキーを保存' })
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('save_google_search_api_key', {
          apiKey: 'serp-api-key-456',
        });
      });

      await waitFor(() => {
        expect(
          screen.getByText(/SerpApi APIキーを保存しました/)
        ).toBeInTheDocument();
      });
    });

    it('shows validation error when SerpApi API key is empty', async () => {
      const user = userEvent.setup();
      renderWithProviders(<Settings />);

      await user.click(
        screen.getByRole('button', { name: 'SerpApi APIキーを保存' })
      );

      await waitFor(() => {
        expect(
          screen.getByText('APIキーを入力してください')
        ).toBeInTheDocument();
      });
    });

    it('shows error when SerpApi save fails', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'save_google_search_api_key')
          return Promise.reject(new Error('Save failed'));
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      const apiKeyInput = document.getElementById(
        'serpapi-key'
      ) as HTMLInputElement;
      await user.type(apiKeyInput, 'key');
      await user.click(
        screen.getByRole('button', { name: 'SerpApi APIキーを保存' })
      );

      await waitFor(() => {
        expect(
          screen.getByText(/保存に失敗しました.*Save failed/)
        ).toBeInTheDocument();
      });
    });

    it('deletes SerpApi API key when confirm is accepted', async () => {
      const user = userEvent.setup();
      vi.stubGlobal(
        'confirm',
        vi.fn(() => true)
      );
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured') return Promise.resolve(true);
        if (cmd === 'delete_google_search_config')
          return Promise.resolve(undefined);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: 'SerpApi APIキーを削除' })
        ).toBeInTheDocument();
      });
      await user.click(
        screen.getByRole('button', { name: 'SerpApi APIキーを削除' })
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('delete_google_search_config');
      });

      await waitFor(() => {
        expect(
          screen.getByText('SerpApi APIキーを削除しました')
        ).toBeInTheDocument();
      });

      vi.unstubAllGlobals();
    });

    it('does not delete SerpApi when confirm is cancelled', async () => {
      const user = userEvent.setup();
      vi.stubGlobal(
        'confirm',
        vi.fn(() => false)
      );
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured') return Promise.resolve(true);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: 'SerpApi APIキーを削除' })
        ).toBeInTheDocument();
      });
      await user.click(
        screen.getByRole('button', { name: 'SerpApi APIキーを削除' })
      );

      expect(mockInvoke).not.toHaveBeenCalledWith(
        'delete_google_search_config'
      );
      vi.unstubAllGlobals();
    });

    it('shows error when SerpApi delete fails', async () => {
      const user = userEvent.setup();
      vi.stubGlobal(
        'confirm',
        vi.fn(() => true)
      );
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured') return Promise.resolve(true);
        if (cmd === 'delete_google_search_config')
          return Promise.reject(new Error('Delete failed'));
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: 'SerpApi APIキーを削除' })
        ).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: 'SerpApi APIキーを削除' })
      );

      await waitFor(() => {
        expect(
          screen.getByText(/削除に失敗しました.*Delete failed/)
        ).toBeInTheDocument();
      });

      vi.unstubAllGlobals();
    });
  });

  // Gmail OAuth 設定カード表示テスト
  it('renders Gmail OAuth settings card', async () => {
    renderWithProviders(<Settings />);
    await waitFor(() => {
      expect(
        screen.getByRole('heading', { name: /Gmail OAuth認証/ })
      ).toBeInTheDocument();
    });
  });

  // gmailOAuthStatus 状態表示テスト
  describe('gmailOAuthStatus state display', () => {
    it('displays checking state for Gmail OAuth status', async () => {
      let resolveGmail: (value: boolean) => void;
      const gmailPromise = new Promise<boolean>((resolve) => {
        resolveGmail = resolve;
      });
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials') return gmailPromise;
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByText('認証情報の状態を確認中...')
        ).toBeInTheDocument();
      });

      resolveGmail!(false);
    });

    it('displays error state for Gmail OAuth when backend is not running', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.reject(new Error('Backend not running'));
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByText(
            '認証情報の状態を取得できません（バックエンド未起動の可能性）'
          )
        ).toBeInTheDocument();
      });
    });

    it('displays available state for Gmail OAuth', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials') return Promise.resolve(true);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(screen.getByText('認証情報は設定済みです')).toBeInTheDocument();
      });
    });

    it('displays unavailable state for Gmail OAuth', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByText('認証情報を設定してください')
        ).toBeInTheDocument();
      });
    });

    it('shows delete button when Gmail OAuth is available', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials') return Promise.resolve(true);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: 'Gmail OAuth認証情報を削除' })
        ).toBeInTheDocument();
      });
    });

    it('hides delete button when Gmail OAuth is unavailable', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByText('認証情報を設定してください')
        ).toBeInTheDocument();
      });
      expect(
        screen.queryByRole('button', { name: 'Gmail OAuth認証情報を削除' })
      ).not.toBeInTheDocument();
    });

    it('hides delete button when Gmail OAuth status is error', async () => {
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.reject(new Error('Backend error'));
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByText(
            '認証情報の状態を取得できません（バックエンド未起動の可能性）'
          )
        ).toBeInTheDocument();
      });
      expect(
        screen.queryByRole('button', { name: 'Gmail OAuth認証情報を削除' })
      ).not.toBeInTheDocument();
    });
  });

  // Gmail OAuth 保存/削除テスト
  describe('handleSaveGmailOAuth / handleDeleteGmailOAuth', () => {
    it('saves Gmail OAuth credentials successfully via JSON paste', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials') return Promise.resolve(true);
        if (cmd === 'save_gmail_oauth_credentials')
          return Promise.resolve(undefined);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      const validJson = JSON.stringify({
        installed: {
          client_id: 'test-client-id.apps.googleusercontent.com',
          client_secret: 'GOCSPX-test-secret',
        },
      });

      const textarea = screen.getByLabelText(/client_secret\.json の内容/);
      fireEvent.change(textarea, { target: { value: validJson } });

      await user.click(
        screen.getByRole('button', { name: 'Gmail OAuth認証情報を保存' })
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          'save_gmail_oauth_credentials',
          { jsonContent: validJson }
        );
      });

      await waitFor(() => {
        expect(
          screen.getByText(/Gmail OAuth認証情報を保存しました/)
        ).toBeInTheDocument();
      });
    });

    it('disables save button when JSON textarea is empty', async () => {
      renderWithProviders(<Settings />);

      const saveButton = screen.getByRole('button', {
        name: 'Gmail OAuth認証情報を保存',
      });
      expect(saveButton).toBeDisabled();
    });

    it('shows error when JSON format is invalid', async () => {
      const user = userEvent.setup();
      renderWithProviders(<Settings />);

      const textarea = screen.getByLabelText(/client_secret\.json の内容/);
      fireEvent.change(textarea, { target: { value: 'not valid json' } });

      await user.click(
        screen.getByRole('button', { name: 'Gmail OAuth認証情報を保存' })
      );

      await waitFor(() => {
        expect(screen.getByText('無効なJSON形式です')).toBeInTheDocument();
      });
    });

    it('shows error when Gmail OAuth save fails', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        if (cmd === 'save_gmail_oauth_credentials')
          return Promise.reject(new Error('Save failed'));
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      const validJson = JSON.stringify({
        installed: {
          client_id: 'test-id',
          client_secret: 'test-secret',
        },
      });
      const textarea = screen.getByLabelText(/client_secret\.json の内容/);
      fireEvent.change(textarea, { target: { value: validJson } });
      await user.click(
        screen.getByRole('button', { name: 'Gmail OAuth認証情報を保存' })
      );

      await waitFor(() => {
        expect(
          screen.getByText(/保存に失敗しました.*Save failed/)
        ).toBeInTheDocument();
      });
    });

    it('deletes Gmail OAuth credentials when confirm is accepted', async () => {
      const user = userEvent.setup();
      vi.stubGlobal(
        'confirm',
        vi.fn(() => true)
      );
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials') return Promise.resolve(true);
        if (cmd === 'delete_gmail_oauth_credentials')
          return Promise.resolve(undefined);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: 'Gmail OAuth認証情報を削除' })
        ).toBeInTheDocument();
      });
      await user.click(
        screen.getByRole('button', { name: 'Gmail OAuth認証情報を削除' })
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          'delete_gmail_oauth_credentials'
        );
      });

      await waitFor(() => {
        expect(
          screen.getByText('Gmail OAuth認証情報を削除しました')
        ).toBeInTheDocument();
      });

      vi.unstubAllGlobals();
    });

    it('does not delete Gmail OAuth when confirm is cancelled', async () => {
      const user = userEvent.setup();
      vi.stubGlobal(
        'confirm',
        vi.fn(() => false)
      );
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials') return Promise.resolve(true);
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: 'Gmail OAuth認証情報を削除' })
        ).toBeInTheDocument();
      });
      await user.click(
        screen.getByRole('button', { name: 'Gmail OAuth認証情報を削除' })
      );

      expect(mockInvoke).not.toHaveBeenCalledWith(
        'delete_gmail_oauth_credentials'
      );
      vi.unstubAllGlobals();
    });

    it('shows error when Gmail OAuth delete fails', async () => {
      const user = userEvent.setup();
      vi.stubGlobal(
        'confirm',
        vi.fn(() => true)
      );
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_sync_status')
          return Promise.resolve(defaultSyncMetadata);
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials') return Promise.resolve(true);
        if (cmd === 'delete_gmail_oauth_credentials')
          return Promise.reject(new Error('Delete failed'));
        return Promise.resolve(null);
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: 'Gmail OAuth認証情報を削除' })
        ).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: 'Gmail OAuth認証情報を削除' })
      );

      await waitFor(() => {
        expect(
          screen.getByText(/削除に失敗しました.*Delete failed/)
        ).toBeInTheDocument();
      });

      vi.unstubAllGlobals();
    });
  });

  // Gmail OAuth ファイルアップロード・inputMode切り替えテスト
  describe('handleFileUpload / inputMode', () => {
    it('switches between paste and file input modes', async () => {
      const user = userEvent.setup();
      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByRole('heading', { name: /Gmail OAuth認証/ })
        ).toBeInTheDocument();
      });

      // 初期状態: JSON貼り付けモード
      expect(
        screen.getByLabelText(/client_secret\.json の内容/)
      ).toBeInTheDocument();

      // ファイルアップロードモードに切り替え
      await user.click(
        screen.getByRole('radio', { name: /ファイルをアップロード/ })
      );
      expect(
        screen.getByLabelText(/client_secret\.json ファイル/)
      ).toBeInTheDocument();

      // JSON貼り付けモードに戻す
      await user.click(screen.getByRole('radio', { name: /JSONを貼り付け/ }));
      expect(
        screen.getByLabelText(/client_secret\.json の内容/)
      ).toBeInTheDocument();
    });

    it('loads file content and displays success message when file upload succeeds', async () => {
      const user = userEvent.setup();
      const validJson = JSON.stringify({
        installed: {
          client_id: 'test-id.apps.googleusercontent.com',
          client_secret: 'GOCSPX-secret',
        },
      });
      const file = new File([validJson], 'client_secret.json', {
        type: 'application/json',
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByRole('heading', { name: /Gmail OAuth認証/ })
        ).toBeInTheDocument();
      });

      // ファイルアップロードモードに切り替え
      await user.click(
        screen.getByRole('radio', { name: /ファイルをアップロード/ })
      );

      const fileInput = screen.getByLabelText(/client_secret\.json ファイル/);
      await user.upload(fileInput, file);

      await waitFor(() => {
        expect(
          screen.getByText('ファイルが読み込まれました')
        ).toBeInTheDocument();
      });
    });

    it('shows error message when file read fails', async () => {
      const user = userEvent.setup();
      class MockFileReader {
        onload:
          | ((this: FileReader, ev: ProgressEvent<FileReader>) => void)
          | null = null;
        onerror:
          | ((this: FileReader, ev: ProgressEvent<FileReader>) => void)
          | null = null;
        readAsText() {
          queueMicrotask(() => {
            if (this.onerror) {
              this.onerror.call(
                this as unknown as FileReader,
                new ProgressEvent('error') as ProgressEvent<FileReader>
              );
            }
          });
        }
      }
      vi.stubGlobal('FileReader', MockFileReader);

      const file = new File(['content'], 'test.json', {
        type: 'application/json',
      });

      renderWithProviders(<Settings />);

      await waitFor(() => {
        expect(
          screen.getByRole('heading', { name: /Gmail OAuth認証/ })
        ).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('radio', { name: /ファイルをアップロード/ })
      );

      const fileInput = screen.getByLabelText(/client_secret\.json ファイル/);
      await user.upload(fileInput, file);

      await waitFor(() => {
        expect(
          screen.getByText('ファイルの読み込みに失敗しました')
        ).toBeInTheDocument();
      });

      vi.unstubAllGlobals();
    });
  });
});
