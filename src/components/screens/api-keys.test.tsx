import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { fireEvent } from '@testing-library/react';
import { Toaster } from 'sonner';
import { ApiKeys } from './api-keys';
import { ParseProvider } from '@/contexts/parse-provider';
import { mockInvoke, mockListen } from '@/test/setup';

const renderWithProviders = (ui: React.ReactElement) => {
  return render(
    <>
      <ParseProvider>{ui}</ParseProvider>
      <Toaster position="top-right" richColors />
    </>
  );
};

const defaultParseMetadata = {
  batch_size: 100,
  parse_status: 'idle',
  last_parse_started_at: null,
  last_parse_completed_at: null,
  last_error_message: null,
  total_parsed_count: 0,
};

describe('ApiKeys', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockImplementation((cmd: string) => {
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

  it('renders API keys heading', () => {
    renderWithProviders(<ApiKeys />);
    expect(
      screen.getByRole('heading', { name: /APIキー設定/i, level: 1 })
    ).toBeInTheDocument();
  });

  it('renders Gmail OAuth settings card', async () => {
    renderWithProviders(<ApiKeys />);
    await waitFor(() => {
      expect(
        screen.getByRole('heading', { name: /Gmail OAuth認証/ })
      ).toBeInTheDocument();
    });
  });

  it('renders SerpApi settings card', async () => {
    renderWithProviders(<ApiKeys />);
    await waitFor(() => {
      expect(
        screen.getByRole('heading', { name: /SerpApi/ })
      ).toBeInTheDocument();
    });
  });

  it('renders Gemini API settings card', async () => {
    renderWithProviders(<ApiKeys />);
    await waitFor(() => {
      expect(
        screen.getByRole('heading', { name: /Gemini API/ })
      ).toBeInTheDocument();
    });
  });

  describe('handleSaveGeminiApiKey / handleDeleteGeminiApiKey', () => {
    it('saves Gemini API key successfully and clears input', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'save_gemini_api_key') return Promise.resolve(undefined);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<ApiKeys />);

      await waitFor(() => {
        expect(document.getElementById('gemini-api-key')).toBeInTheDocument();
      });

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

    it('shows validation error when API key is empty', async () => {
      const user = userEvent.setup();
      renderWithProviders(<ApiKeys />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: 'Gemini APIキーを保存' })
        ).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: 'Gemini APIキーを保存' })
      );

      await waitFor(() => {
        expect(
          screen.getByText('APIキーを入力してください')
        ).toBeInTheDocument();
      });
    });

    it('shows error when Gemini API key save fails', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        if (cmd === 'save_gemini_api_key')
          return Promise.reject(new Error('Keyring error'));
        return Promise.resolve(null);
      });

      renderWithProviders(<ApiKeys />);

      const apiKeyInput = document.getElementById(
        'gemini-api-key'
      ) as HTMLInputElement;
      await user.type(apiKeyInput, 'test-api-key-123');

      await user.click(
        screen.getByRole('button', { name: 'Gemini APIキーを保存' })
      );

      await waitFor(() => {
        expect(
          screen.getByText(/保存に失敗しました: Keyring error/)
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
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(true);
        if (cmd === 'delete_gemini_api_key') return Promise.resolve(undefined);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<ApiKeys />);

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

    it('does not delete Gemini API key when confirm is cancelled', async () => {
      const user = userEvent.setup();
      const confirmMock = vi.fn(() => false);
      vi.stubGlobal('confirm', confirmMock);
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(true);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        return Promise.resolve(null);
      });

      renderWithProviders(<ApiKeys />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: 'Gemini APIキーを削除' })
        ).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: 'Gemini APIキーを削除' })
      );

      expect(confirmMock).toHaveBeenCalled();
      expect(mockInvoke).not.toHaveBeenCalledWith('delete_gemini_api_key');

      vi.unstubAllGlobals();
    });
  });

  describe('handleSaveSerpApiKey / handleDeleteSerpApiKey', () => {
    it('saves SerpApi API key successfully', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        if (cmd === 'save_google_search_api_key')
          return Promise.resolve(undefined);
        return Promise.resolve(null);
      });

      renderWithProviders(<ApiKeys />);

      await waitFor(() => {
        expect(document.getElementById('serpapi-key')).toBeInTheDocument();
      });

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
      renderWithProviders(<ApiKeys />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: 'SerpApi APIキーを保存' })
        ).toBeInTheDocument();
      });

      await user.click(
        screen.getByRole('button', { name: 'SerpApi APIキーを保存' })
      );

      await waitFor(() => {
        expect(
          screen.getByText('APIキーを入力してください')
        ).toBeInTheDocument();
      });
    });

    it('shows error when SerpApi API key save fails', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        if (cmd === 'save_google_search_api_key')
          return Promise.reject(new Error('Network error'));
        return Promise.resolve(null);
      });

      renderWithProviders(<ApiKeys />);

      const apiKeyInput = document.getElementById(
        'serpapi-key'
      ) as HTMLInputElement;
      await user.type(apiKeyInput, 'serp-api-key-456');

      await user.click(
        screen.getByRole('button', { name: 'SerpApi APIキーを保存' })
      );

      await waitFor(() => {
        expect(
          screen.getByText(/保存に失敗しました: Network error/)
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
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured') return Promise.resolve(true);
        if (cmd === 'has_gmail_oauth_credentials')
          return Promise.resolve(false);
        if (cmd === 'delete_google_search_config')
          return Promise.resolve(undefined);
        return Promise.resolve(null);
      });

      renderWithProviders(<ApiKeys />);

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
  });

  describe('handleSaveGmailOAuth / handleDeleteGmailOAuth', () => {
    it('saves Gmail OAuth credentials successfully via JSON paste', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
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

      renderWithProviders(<ApiKeys />);

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
      renderWithProviders(<ApiKeys />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', {
            name: 'Gmail OAuth認証情報を保存',
          })
        ).toBeInTheDocument();
      });

      const saveButton = screen.getByRole('button', {
        name: 'Gmail OAuth認証情報を保存',
      });
      expect(saveButton).toBeDisabled();
    });

    it('shows error when JSON format is invalid', async () => {
      const user = userEvent.setup();
      renderWithProviders(<ApiKeys />);

      const textarea = screen.getByLabelText(/client_secret\.json の内容/);
      fireEvent.change(textarea, { target: { value: 'not valid json' } });

      await user.click(
        screen.getByRole('button', { name: 'Gmail OAuth認証情報を保存' })
      );

      await waitFor(() => {
        expect(screen.getByText('無効なJSON形式です')).toBeInTheDocument();
      });
    });

    it('deletes Gmail OAuth credentials when confirm is accepted', async () => {
      const user = userEvent.setup();
      vi.stubGlobal(
        'confirm',
        vi.fn(() => true)
      );
      mockInvoke.mockImplementation((cmd: string) => {
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

      renderWithProviders(<ApiKeys />);

      await waitFor(() => {
        expect(
          screen.getByRole('button', {
            name: 'Gmail OAuth認証情報を削除',
          })
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
          screen.getByText(/Gmail OAuth認証情報を削除しました/)
        ).toBeInTheDocument();
      });

      vi.unstubAllGlobals();
    });

    it('shows error when deleting Gmail OAuth credentials fails', async () => {
      const user = userEvent.setup();
      vi.stubGlobal(
        'confirm',
        vi.fn(() => true)
      );
      mockInvoke.mockImplementation((cmd: string) => {
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

      renderWithProviders(<ApiKeys />);

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
          screen.getByText(/削除に失敗しました: Delete failed/)
        ).toBeInTheDocument();
      });

      vi.unstubAllGlobals();
    });

    it('handles Gmail OAuth save error', async () => {
      const user = userEvent.setup();
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_parse_status')
          return Promise.resolve(defaultParseMetadata);
        if (cmd === 'has_gemini_api_key') return Promise.resolve(false);
        if (cmd === 'is_google_search_configured')
          return Promise.resolve(false);
        if (cmd === 'has_gmail_oauth_credentials') return Promise.resolve(true);
        if (cmd === 'save_gmail_oauth_credentials')
          return Promise.reject(new Error('Keyring error'));
        return Promise.resolve(null);
      });

      renderWithProviders(<ApiKeys />);

      const validJson = JSON.stringify({
        installed: {
          client_id: 'test.apps.googleusercontent.com',
          client_secret: 'secret',
        },
      });
      const textarea = screen.getByLabelText(/client_secret\.json の内容/);
      fireEvent.change(textarea, { target: { value: validJson } });

      await user.click(
        screen.getByRole('button', { name: 'Gmail OAuth認証情報を保存' })
      );

      await waitFor(() => {
        expect(
          screen.getByText(/保存に失敗しました.*Keyring error/)
        ).toBeInTheDocument();
      });
    });
  });

  describe('handleFileUpload / inputMode', () => {
    it('switches between paste and file input modes', async () => {
      const user = userEvent.setup();
      renderWithProviders(<ApiKeys />);

      await waitFor(() => {
        expect(
          screen.getByRole('heading', { name: /Gmail OAuth認証/ })
        ).toBeInTheDocument();
      });

      expect(
        screen.getByLabelText(/client_secret\.json の内容/)
      ).toBeInTheDocument();

      await user.click(
        screen.getByRole('radio', { name: /ファイルをアップロード/ })
      );
      expect(
        screen.getByLabelText(/client_secret\.json ファイル/)
      ).toBeInTheDocument();

      await user.click(screen.getByRole('radio', { name: /JSONを貼り付け/ }));
      expect(
        screen.getByLabelText(/client_secret\.json の内容/)
      ).toBeInTheDocument();
    });

    it('loads gmail oauth json from uploaded file (FileReader onload)', async () => {
      const user = userEvent.setup();

      const originalFileReader = global.FileReader;
      class MockFileReader {
        public onload: ((e: ProgressEvent<FileReader>) => void) | null = null;
        public onerror: (() => void) | null = null;
        readAsText(_file: File) {
          const e = { target: { result: '{"installed":{"client_id":"x"}}' } };
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          this.onload?.(e as any);
        }
      }
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (global as any).FileReader = MockFileReader as any;

      renderWithProviders(<ApiKeys />);

      await user.click(
        screen.getByRole('radio', { name: /ファイルをアップロード/ })
      );

      const fileInput = document.getElementById(
        'gmail-oauth-file'
      ) as HTMLInputElement;
      const file = new File(['{}'], 'client_secret.json', {
        type: 'application/json',
      });

      fireEvent.change(fileInput, { target: { files: [file] } });

      await waitFor(() => {
        expect(
          screen.getByText('ファイルが読み込まれました')
        ).toBeInTheDocument();
      });

      // input の value はクリアされる（再アップロード可能にする）
      expect(fileInput.value).toBe('');

      // 復元
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (global as any).FileReader = originalFileReader as any;
    });

    it('shows error when file read fails (FileReader onerror)', async () => {
      const user = userEvent.setup();

      const originalFileReader = global.FileReader;
      class MockFileReaderError {
        public onload: ((e: ProgressEvent<FileReader>) => void) | null = null;
        public onerror: (() => void) | null = null;
        readAsText(_file: File) {
          this.onerror?.();
        }
      }
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (global as any).FileReader = MockFileReaderError as any;

      renderWithProviders(<ApiKeys />);

      await user.click(
        screen.getByRole('radio', { name: /ファイルをアップロード/ })
      );

      const fileInput = document.getElementById(
        'gmail-oauth-file'
      ) as HTMLInputElement;
      const file = new File(['{}'], 'client_secret.json', {
        type: 'application/json',
      });

      fireEvent.change(fileInput, { target: { files: [file] } });

      await waitFor(() => {
        expect(
          screen.getByText('ファイルの読み込みに失敗しました')
        ).toBeInTheDocument();
      });

      // 復元
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (global as any).FileReader = originalFileReader as any;
    });
  });
});
