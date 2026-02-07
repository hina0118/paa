import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Key } from 'lucide-react';
import { useParse } from '@/contexts/use-parse';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { toastSuccess, toastError, formatError } from '@/lib/toast';

export function ApiKeys() {
  const { geminiApiKeyStatus, refreshGeminiApiKeyStatus } = useParse();
  // Gemini API キー
  const [geminiApiKey, setGeminiApiKey] = useState<string>('');
  const [isSavingGeminiApiKey, setIsSavingGeminiApiKey] = useState(false);
  const [isDeletingGeminiApiKey, setIsDeletingGeminiApiKey] = useState(false);
  // SerpApi
  const [serpApiKey, setSerpApiKey] = useState<string>('');
  const [isSavingSerpApi, setIsSavingSerpApi] = useState(false);
  const [isDeletingSerpApi, setIsDeletingSerpApi] = useState(false);
  const [serpApiStatus, setSerpApiStatus] = useState<
    'checking' | 'available' | 'unavailable' | 'error'
  >('checking');
  // Gmail OAuth
  const [gmailOAuthStatus, setGmailOAuthStatus] = useState<
    'checking' | 'available' | 'unavailable' | 'error'
  >('checking');
  const [gmailOAuthJson, setGmailOAuthJson] = useState<string>('');
  const [isSavingGmailOAuth, setIsSavingGmailOAuth] = useState(false);
  const [isDeletingGmailOAuth, setIsDeletingGmailOAuth] = useState(false);
  const [inputMode, setInputMode] = useState<'paste' | 'file'>('paste');
  const fileInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    refreshGeminiApiKeyStatus();
  }, [refreshGeminiApiKeyStatus]);

  const refreshSerpApiStatus = useCallback(async () => {
    setSerpApiStatus('checking');
    try {
      const configured = await invoke<boolean>('is_google_search_configured');
      setSerpApiStatus(configured ? 'available' : 'unavailable');
    } catch (error) {
      console.error('Failed to check SerpApi config:', error);
      setSerpApiStatus('error');
    }
  }, []);

  useEffect(() => {
    refreshSerpApiStatus();
  }, [refreshSerpApiStatus]);

  const refreshGmailOAuthStatus = useCallback(async () => {
    setGmailOAuthStatus('checking');
    try {
      const configured = await invoke<boolean>('has_gmail_oauth_credentials');
      setGmailOAuthStatus(configured ? 'available' : 'unavailable');
    } catch (error) {
      console.error('Failed to check Gmail OAuth config:', error);
      setGmailOAuthStatus('error');
    }
  }, []);

  useEffect(() => {
    refreshGmailOAuthStatus();
  }, [refreshGmailOAuthStatus]);

  const handleSaveGeminiApiKey = async () => {
    const key = geminiApiKey.trim();
    if (!key) {
      toastError('APIキーを入力してください');
      return;
    }

    setIsSavingGeminiApiKey(true);
    try {
      await invoke('save_gemini_api_key', { apiKey: key });
      toastSuccess(
        'Gemini APIキーを保存しました（OSのセキュアストレージに保存）'
      );
      setGeminiApiKey('');
      await refreshGeminiApiKeyStatus();
    } catch (error) {
      toastError(`保存に失敗しました: ${formatError(error)}`);
    } finally {
      setIsSavingGeminiApiKey(false);
    }
  };

  const handleDeleteGeminiApiKey = async () => {
    if (!confirm('Gemini APIキーを削除しますか？')) {
      return;
    }

    setIsDeletingGeminiApiKey(true);
    try {
      await invoke('delete_gemini_api_key');
      toastSuccess('Gemini APIキーを削除しました');
      setGeminiApiKey('');
      await refreshGeminiApiKeyStatus();
    } catch (error) {
      toastError(`削除に失敗しました: ${formatError(error)}`);
    } finally {
      setIsDeletingGeminiApiKey(false);
    }
  };

  const handleSaveSerpApiKey = async () => {
    const apiKey = serpApiKey.trim();

    if (!apiKey) {
      toastError('APIキーを入力してください');
      return;
    }

    setIsSavingSerpApi(true);
    try {
      await invoke('save_google_search_api_key', { apiKey });
      toastSuccess(
        'SerpApi APIキーを保存しました（OSのセキュアストレージに保存）'
      );
      setSerpApiKey('');
      await refreshSerpApiStatus();
    } catch (error) {
      toastError(`保存に失敗しました: ${formatError(error)}`);
    } finally {
      setIsSavingSerpApi(false);
    }
  };

  const handleDeleteSerpApiKey = async () => {
    if (!confirm('SerpApi APIキーを削除しますか？')) {
      return;
    }

    setIsDeletingSerpApi(true);
    try {
      await invoke('delete_google_search_config');
      toastSuccess('SerpApi APIキーを削除しました');
      setSerpApiKey('');
      await refreshSerpApiStatus();
    } catch (error) {
      toastError(`削除に失敗しました: ${formatError(error)}`);
    } finally {
      setIsDeletingSerpApi(false);
    }
  };

  const handleSaveGmailOAuth = async () => {
    const jsonContent = gmailOAuthJson.trim();

    if (!jsonContent) {
      toastError('JSONを入力してください');
      return;
    }

    try {
      JSON.parse(jsonContent);
    } catch {
      toastError('無効なJSON形式です');
      return;
    }

    setIsSavingGmailOAuth(true);
    try {
      await invoke('save_gmail_oauth_credentials', { jsonContent });
      toastSuccess(
        'Gmail OAuth認証情報を保存しました（OSのセキュアストレージに保存）'
      );
      setGmailOAuthJson('');
      await refreshGmailOAuthStatus();
    } catch (error) {
      toastError(`保存に失敗しました: ${formatError(error)}`);
    } finally {
      setIsSavingGmailOAuth(false);
    }
  };

  const handleDeleteGmailOAuth = async () => {
    if (!confirm('Gmail OAuth認証情報を削除しますか？')) {
      return;
    }

    setIsDeletingGmailOAuth(true);
    try {
      await invoke('delete_gmail_oauth_credentials');
      toastSuccess('Gmail OAuth認証情報を削除しました');
      setGmailOAuthJson('');
      await refreshGmailOAuthStatus();
    } catch (error) {
      toastError(`削除に失敗しました: ${formatError(error)}`);
    } finally {
      setIsDeletingGmailOAuth(false);
    }
  };

  const handleFileUpload = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      const content = e.target?.result as string;
      setGmailOAuthJson(content);
    };
    reader.onerror = () => {
      toastError('ファイルの読み込みに失敗しました');
    };
    reader.readAsText(file);

    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  return (
    <div className="container mx-auto py-10 px-6 space-y-6">
      <div className="mb-8 space-y-2">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-primary/10">
            <Key className="h-6 w-6 text-primary" />
          </div>
          <h1 className="text-3xl font-bold tracking-tight">APIキー設定</h1>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Gmail OAuth認証</CardTitle>
          <CardDescription>
            Gmail同期に使用するOAuth認証情報を設定します（OSのセキュアストレージに保存）
            <br />
            <a
              href="https://console.cloud.google.com/apis/credentials"
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-600 hover:underline"
            >
              Google Cloud ConsoleでOAuth認証情報を取得
            </a>
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="space-y-2">
            <p className="text-sm text-muted-foreground">
              {gmailOAuthStatus === 'checking' && '認証情報の状態を確認中...'}
              {gmailOAuthStatus === 'error' &&
                '認証情報の状態を取得できません（バックエンド未起動の可能性）'}
              {gmailOAuthStatus === 'available' && '認証情報は設定済みです'}
              {gmailOAuthStatus === 'unavailable' &&
                '認証情報を設定してください'}
            </p>
          </div>
          <div className="space-y-4">
            <div className="flex gap-4">
              <label className="flex items-center gap-2">
                <input
                  type="radio"
                  name="input-mode"
                  checked={inputMode === 'paste'}
                  onChange={() => setInputMode('paste')}
                  disabled={isSavingGmailOAuth || isDeletingGmailOAuth}
                />
                <span className="text-sm">JSONを貼り付け</span>
              </label>
              <label className="flex items-center gap-2">
                <input
                  type="radio"
                  name="input-mode"
                  checked={inputMode === 'file'}
                  onChange={() => setInputMode('file')}
                  disabled={isSavingGmailOAuth || isDeletingGmailOAuth}
                />
                <span className="text-sm">ファイルをアップロード</span>
              </label>
            </div>

            {inputMode === 'paste' ? (
              <div className="space-y-2">
                <label
                  htmlFor="gmail-oauth-json"
                  className="text-sm font-medium"
                >
                  client_secret.json の内容
                </label>
                <Textarea
                  id="gmail-oauth-json"
                  placeholder='{"installed":{"client_id":"...","client_secret":"..."}}'
                  value={gmailOAuthJson}
                  onChange={(e) => setGmailOAuthJson(e.target.value)}
                  disabled={isSavingGmailOAuth || isDeletingGmailOAuth}
                  className="min-h-[120px] font-mono text-sm"
                />
              </div>
            ) : (
              <div className="space-y-2">
                <label
                  htmlFor="gmail-oauth-file"
                  className="text-sm font-medium"
                >
                  client_secret.json ファイル
                </label>
                <div className="flex gap-2 items-center">
                  <input
                    id="gmail-oauth-file"
                    ref={fileInputRef}
                    type="file"
                    accept=".json,application/json"
                    onChange={handleFileUpload}
                    disabled={isSavingGmailOAuth || isDeletingGmailOAuth}
                    className="text-sm"
                  />
                </div>
                {gmailOAuthJson && (
                  <p className="text-sm text-muted-foreground">
                    ファイルが読み込まれました
                  </p>
                )}
              </div>
            )}

            <div className="flex gap-2">
              <Button
                onClick={handleSaveGmailOAuth}
                disabled={
                  isSavingGmailOAuth ||
                  isDeletingGmailOAuth ||
                  !gmailOAuthJson.trim()
                }
                aria-label="Gmail OAuth認証情報を保存"
              >
                保存
              </Button>
              {gmailOAuthStatus === 'available' && (
                <Button
                  variant="destructive"
                  onClick={handleDeleteGmailOAuth}
                  disabled={isSavingGmailOAuth || isDeletingGmailOAuth}
                  aria-label="Gmail OAuth認証情報を削除"
                >
                  削除
                </Button>
              )}
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Gemini API</CardTitle>
          <CardDescription>
            商品名解析に使用するGemini
            APIキーを設定します（OSのセキュアストレージに保存）
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="space-y-2">
            <label htmlFor="gemini-api-key" className="text-sm font-medium">
              APIキー
            </label>
            <p className="text-sm text-muted-foreground">
              {geminiApiKeyStatus === 'checking' && 'APIキーの状態を確認中...'}
              {geminiApiKeyStatus === 'error' &&
                'APIキーの状態を取得できません（バックエンド未起動の可能性）'}
              {geminiApiKeyStatus === 'available' && 'APIキーは設定済みです'}
              {geminiApiKeyStatus === 'unavailable' &&
                'APIキーを入力して保存してください'}
            </p>
            <div className="flex gap-2">
              <Input
                id="gemini-api-key"
                type="password"
                placeholder={
                  geminiApiKeyStatus === 'available'
                    ? '********'
                    : 'APIキーを入力'
                }
                value={geminiApiKey}
                onChange={(e) => setGeminiApiKey(e.target.value)}
                disabled={isSavingGeminiApiKey || isDeletingGeminiApiKey}
                className="max-w-md"
              />
              <Button
                onClick={handleSaveGeminiApiKey}
                disabled={isSavingGeminiApiKey || isDeletingGeminiApiKey}
                aria-label="Gemini APIキーを保存"
              >
                保存
              </Button>
              {geminiApiKeyStatus === 'available' && (
                <Button
                  variant="destructive"
                  onClick={handleDeleteGeminiApiKey}
                  disabled={isSavingGeminiApiKey || isDeletingGeminiApiKey}
                  aria-label="Gemini APIキーを削除"
                >
                  削除
                </Button>
              )}
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>SerpApi（画像検索）</CardTitle>
          <CardDescription>
            商品画像検索に使用するSerpApiの設定です（OSのセキュアストレージに保存）
            <br />
            <a
              href="https://serpapi.com/"
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-600 hover:underline"
            >
              SerpApiでAPIキーを取得
            </a>
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="space-y-2">
            <p className="text-sm text-muted-foreground">
              {serpApiStatus === 'checking' && 'APIキーの状態を確認中...'}
              {serpApiStatus === 'error' &&
                'APIキーの状態を取得できません（バックエンド未起動の可能性）'}
              {serpApiStatus === 'available' && 'APIキーは設定済みです'}
              {serpApiStatus === 'unavailable' &&
                'APIキーを入力して保存してください'}
            </p>
          </div>
          <div className="space-y-2">
            <label htmlFor="serpapi-key" className="text-sm font-medium">
              APIキー
            </label>
            <div className="flex gap-2">
              <Input
                id="serpapi-key"
                type="password"
                placeholder={
                  serpApiStatus === 'available' ? '********' : 'APIキーを入力'
                }
                value={serpApiKey}
                onChange={(e) => setSerpApiKey(e.target.value)}
                disabled={isSavingSerpApi || isDeletingSerpApi}
                className="max-w-md"
              />
              <Button
                onClick={handleSaveSerpApiKey}
                disabled={isSavingSerpApi || isDeletingSerpApi}
                aria-label="SerpApi APIキーを保存"
              >
                保存
              </Button>
              {serpApiStatus === 'available' && (
                <Button
                  variant="destructive"
                  onClick={handleDeleteSerpApiKey}
                  disabled={isSavingSerpApi || isDeletingSerpApi}
                  aria-label="SerpApi APIキーを削除"
                >
                  削除
                </Button>
              )}
            </div>
          </div>
          <div className="pt-4 border-t">
            <p className="text-sm text-muted-foreground">
              無料枠: 月100リクエストまで
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
