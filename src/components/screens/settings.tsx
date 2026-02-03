import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useSync } from '@/contexts/use-sync';
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

export function Settings() {
  const { metadata, updateBatchSize, updateMaxIterations } = useSync();
  const {
    metadata: parseMetadata,
    updateBatchSize: updateParseBatchSize,
    hasGeminiApiKey,
    refreshGeminiApiKeyStatus,
  } = useParse();
  const [batchSize, setBatchSize] = useState<string>('');
  const [maxIterations, setMaxIterations] = useState<string>('');
  const [parseBatchSize, setParseBatchSize] = useState<string>('');
  const [isSavingBatchSize, setIsSavingBatchSize] = useState(false);
  const [isSavingMaxIterations, setIsSavingMaxIterations] = useState(false);
  const [isSavingParseBatchSize, setIsSavingParseBatchSize] = useState(false);
  const [successMessage, setSuccessMessage] = useState<string>('');
  const [errorMessage, setErrorMessage] = useState<string>('');
  const [isInitialized, setIsInitialized] = useState(false);
  // Gemini API キー
  const [geminiApiKey, setGeminiApiKey] = useState<string>('');
  const [isSavingGeminiApiKey, setIsSavingGeminiApiKey] = useState(false);
  const [isDeletingGeminiApiKey, setIsDeletingGeminiApiKey] = useState(false);
  // SerpApi
  const [isSerpApiConfigured, setIsSerpApiConfigured] = useState(false);
  const [serpApiKey, setSerpApiKey] = useState<string>('');
  const [isSavingSerpApi, setIsSavingSerpApi] = useState(false);
  const [isDeletingSerpApi, setIsDeletingSerpApi] = useState(false);
  // Gmail OAuth
  const [isGmailOAuthConfigured, setIsGmailOAuthConfigured] = useState(false);
  const [gmailOAuthJson, setGmailOAuthJson] = useState<string>('');
  const [isSavingGmailOAuth, setIsSavingGmailOAuth] = useState(false);
  const [isDeletingGmailOAuth, setIsDeletingGmailOAuth] = useState(false);
  const [inputMode, setInputMode] = useState<'paste' | 'file'>('paste');
  const fileInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (metadata && !isInitialized) {
      setBatchSize(metadata.batch_size.toString());
      setMaxIterations(metadata.max_iterations.toString());
      setIsInitialized(true);
    }
  }, [metadata, isInitialized]);

  useEffect(() => {
    if (parseMetadata) {
      setParseBatchSize(parseMetadata.batch_size.toString());
    }
  }, [parseMetadata]);

  useEffect(() => {
    refreshGeminiApiKeyStatus();
  }, [refreshGeminiApiKeyStatus]);

  const refreshSerpApiStatus = useCallback(async () => {
    try {
      const configured = await invoke<boolean>('is_google_search_configured');
      setIsSerpApiConfigured(configured);
    } catch (error) {
      console.error('Failed to check SerpApi config:', error);
    }
  }, []);

  useEffect(() => {
    refreshSerpApiStatus();
  }, [refreshSerpApiStatus]);

  const refreshGmailOAuthStatus = useCallback(async () => {
    try {
      const configured = await invoke<boolean>('has_gmail_oauth_credentials');
      setIsGmailOAuthConfigured(configured);
    } catch (error) {
      console.error('Failed to check Gmail OAuth config:', error);
    }
  }, []);

  useEffect(() => {
    refreshGmailOAuthStatus();
  }, [refreshGmailOAuthStatus]);

  const handleSaveBatchSize = async () => {
    const value = parseInt(batchSize, 10);
    if (isNaN(value) || value <= 0) {
      setErrorMessage('バッチサイズは1以上の整数を入力してください');
      return;
    }

    setIsSavingBatchSize(true);
    setErrorMessage('');
    setSuccessMessage('');

    try {
      await updateBatchSize(value);
      setSuccessMessage('バッチサイズを更新しました');
      setTimeout(() => setSuccessMessage(''), 3000);
    } catch (error) {
      setErrorMessage(
        `更新に失敗しました: ${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      setIsSavingBatchSize(false);
    }
  };

  const handleSaveMaxIterations = async () => {
    const value = parseInt(maxIterations, 10);
    if (isNaN(value) || value <= 0) {
      setErrorMessage('最大繰り返し回数は1以上の整数を入力してください');
      return;
    }

    setIsSavingMaxIterations(true);
    setErrorMessage('');
    setSuccessMessage('');

    try {
      await updateMaxIterations(value);
      setSuccessMessage('最大繰り返し回数を更新しました');
      setTimeout(() => setSuccessMessage(''), 3000);
    } catch (error) {
      setErrorMessage(
        `更新に失敗しました: ${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      setIsSavingMaxIterations(false);
    }
  };

  const handleSaveGeminiApiKey = async () => {
    const key = geminiApiKey.trim();
    if (!key) {
      setErrorMessage('APIキーを入力してください');
      return;
    }

    setIsSavingGeminiApiKey(true);
    setErrorMessage('');
    setSuccessMessage('');

    try {
      await invoke('save_gemini_api_key', { apiKey: key });
      setSuccessMessage(
        'Gemini APIキーを保存しました（OSのセキュアストレージに保存）'
      );
      setGeminiApiKey(''); // セキュリティのため入力欄をクリア
      await refreshGeminiApiKeyStatus();
      setTimeout(() => setSuccessMessage(''), 3000);
    } catch (error) {
      setErrorMessage(
        `保存に失敗しました: ${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      setIsSavingGeminiApiKey(false);
    }
  };

  const handleDeleteGeminiApiKey = async () => {
    if (!confirm('Gemini APIキーを削除しますか？')) {
      return;
    }

    setIsDeletingGeminiApiKey(true);
    setErrorMessage('');
    setSuccessMessage('');

    try {
      await invoke('delete_gemini_api_key');
      setSuccessMessage('Gemini APIキーを削除しました');
      setGeminiApiKey('');
      await refreshGeminiApiKeyStatus();
      setTimeout(() => setSuccessMessage(''), 3000);
    } catch (error) {
      setErrorMessage(
        `削除に失敗しました: ${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      setIsDeletingGeminiApiKey(false);
    }
  };

  const handleSaveParseBatchSize = async () => {
    const value = parseInt(parseBatchSize, 10);
    if (isNaN(value) || value <= 0) {
      setErrorMessage('パースバッチサイズは1以上の整数を入力してください');
      return;
    }

    setIsSavingParseBatchSize(true);
    setErrorMessage('');
    setSuccessMessage('');

    try {
      await updateParseBatchSize(value);
      setSuccessMessage('パースバッチサイズを更新しました');
      setTimeout(() => setSuccessMessage(''), 3000);
    } catch (error) {
      setErrorMessage(
        `更新に失敗しました: ${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      setIsSavingParseBatchSize(false);
    }
  };

  const handleSaveSerpApiKey = async () => {
    const apiKey = serpApiKey.trim();

    if (!apiKey) {
      setErrorMessage('APIキーを入力してください');
      return;
    }

    setIsSavingSerpApi(true);
    setErrorMessage('');
    setSuccessMessage('');

    try {
      await invoke('save_google_search_api_key', { apiKey });
      setSuccessMessage(
        'SerpApi APIキーを保存しました（OSのセキュアストレージに保存）'
      );
      setSerpApiKey('');
      await refreshSerpApiStatus();
      setTimeout(() => setSuccessMessage(''), 3000);
    } catch (error) {
      setErrorMessage(
        `保存に失敗しました: ${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      setIsSavingSerpApi(false);
    }
  };

  const handleDeleteSerpApiKey = async () => {
    if (!confirm('SerpApi APIキーを削除しますか？')) {
      return;
    }

    setIsDeletingSerpApi(true);
    setErrorMessage('');
    setSuccessMessage('');

    try {
      await invoke('delete_google_search_config');
      setSuccessMessage('SerpApi APIキーを削除しました');
      setSerpApiKey('');
      await refreshSerpApiStatus();
      setTimeout(() => setSuccessMessage(''), 3000);
    } catch (error) {
      setErrorMessage(
        `削除に失敗しました: ${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      setIsDeletingSerpApi(false);
    }
  };

  const handleSaveGmailOAuth = async () => {
    const jsonContent = gmailOAuthJson.trim();

    if (!jsonContent) {
      setErrorMessage('JSONを入力してください');
      return;
    }

    // JSONの形式を簡易チェック
    try {
      JSON.parse(jsonContent);
    } catch {
      setErrorMessage('無効なJSON形式です');
      return;
    }

    setIsSavingGmailOAuth(true);
    setErrorMessage('');
    setSuccessMessage('');

    try {
      await invoke('save_gmail_oauth_credentials', { jsonContent });
      setSuccessMessage(
        'Gmail OAuth認証情報を保存しました（OSのセキュアストレージに保存）'
      );
      setGmailOAuthJson(''); // セキュリティのため入力欄をクリア
      await refreshGmailOAuthStatus();
      setTimeout(() => setSuccessMessage(''), 3000);
    } catch (error) {
      setErrorMessage(
        `保存に失敗しました: ${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      setIsSavingGmailOAuth(false);
    }
  };

  const handleDeleteGmailOAuth = async () => {
    if (!confirm('Gmail OAuth認証情報を削除しますか？')) {
      return;
    }

    setIsDeletingGmailOAuth(true);
    setErrorMessage('');
    setSuccessMessage('');

    try {
      await invoke('delete_gmail_oauth_credentials');
      setSuccessMessage('Gmail OAuth認証情報を削除しました');
      setGmailOAuthJson('');
      await refreshGmailOAuthStatus();
      setTimeout(() => setSuccessMessage(''), 3000);
    } catch (error) {
      setErrorMessage(
        `削除に失敗しました: ${error instanceof Error ? error.message : String(error)}`
      );
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
      setErrorMessage('ファイルの読み込みに失敗しました');
    };
    reader.readAsText(file);

    // ファイル入力をリセット（同じファイルを再選択できるように）
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  return (
    <div className="container mx-auto py-10 space-y-6">
      <h1 className="text-3xl font-bold">設定</h1>

      {successMessage && (
        <div
          className="p-3 bg-green-50 border border-green-200 rounded text-sm text-green-800"
          data-testid="success-message"
          role="status"
        >
          {successMessage}
        </div>
      )}

      {errorMessage && (
        <div
          className="p-3 bg-red-50 border border-red-200 rounded text-sm text-red-800"
          data-testid="error-message"
          role="alert"
        >
          {errorMessage}
        </div>
      )}

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
              {isGmailOAuthConfigured
                ? '認証情報は設定済みです'
                : '認証情報を設定してください'}
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
                <label className="text-sm font-medium">
                  client_secret.json ファイル
                </label>
                <div className="flex gap-2 items-center">
                  <input
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
              {isGmailOAuthConfigured && (
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
          <CardTitle>同期設定</CardTitle>
          <CardDescription>Gmail同期の動作を調整します</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="space-y-2">
            <label htmlFor="batch-size" className="text-sm font-medium">
              バッチサイズ
            </label>
            <p className="text-sm text-muted-foreground">
              1回のリクエストで取得するメールの件数 (推奨: 10-100)
            </p>
            <div className="flex gap-2">
              <Input
                id="batch-size"
                type="number"
                min="1"
                value={batchSize}
                onChange={(e) => setBatchSize(e.target.value)}
                disabled={isSavingBatchSize}
                className="max-w-xs"
              />
              <Button
                onClick={handleSaveBatchSize}
                disabled={isSavingBatchSize}
                aria-label="同期バッチサイズを保存"
              >
                保存
              </Button>
            </div>
          </div>

          <div className="space-y-2">
            <label htmlFor="max-iterations" className="text-sm font-medium">
              最大繰り返し回数
            </label>
            <p className="text-sm text-muted-foreground">
              1回の同期で実行する最大バッチ数 (推奨: 100-10000)
              <br />
              最大取得件数 = バッチサイズ × 最大繰り返し回数
            </p>
            <div className="flex gap-2">
              <Input
                id="max-iterations"
                type="number"
                min="1"
                value={maxIterations}
                onChange={(e) => setMaxIterations(e.target.value)}
                disabled={isSavingMaxIterations}
                className="max-w-xs"
              />
              <Button
                onClick={handleSaveMaxIterations}
                disabled={isSavingMaxIterations}
                aria-label="最大繰り返し回数を保存"
              >
                保存
              </Button>
            </div>
          </div>

          {metadata && (
            <div className="pt-4 border-t">
              <p className="text-sm text-muted-foreground">
                現在の最大取得件数:{' '}
                <span className="font-semibold text-foreground">
                  {metadata.batch_size * metadata.max_iterations} 件
                </span>
              </p>
            </div>
          )}
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
              {hasGeminiApiKey
                ? 'APIキーは設定済みです'
                : 'APIキーを入力して保存してください'}
            </p>
            <div className="flex gap-2">
              <Input
                id="gemini-api-key"
                type="password"
                placeholder={hasGeminiApiKey ? '********' : 'APIキーを入力'}
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
              {hasGeminiApiKey && (
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
              {isSerpApiConfigured
                ? 'APIキーは設定済みです'
                : 'APIキーを入力して保存してください'}
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
                placeholder={isSerpApiConfigured ? '********' : 'APIキーを入力'}
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
              {isSerpApiConfigured && (
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

      <Card>
        <CardHeader>
          <CardTitle>パース設定</CardTitle>
          <CardDescription>メールパースの動作を調整します</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="space-y-2">
            <label htmlFor="parse-batch-size" className="text-sm font-medium">
              バッチサイズ
            </label>
            <p className="text-sm text-muted-foreground">
              1回のパース処理で処理するメールの件数 (推奨: 50-500)
            </p>
            <div className="flex gap-2">
              <Input
                id="parse-batch-size"
                type="number"
                min="1"
                value={parseBatchSize}
                onChange={(e) => setParseBatchSize(e.target.value)}
                disabled={isSavingParseBatchSize}
                className="max-w-xs"
              />
              <Button
                onClick={handleSaveParseBatchSize}
                disabled={isSavingParseBatchSize}
                aria-label="パースバッチサイズを保存"
              >
                保存
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
