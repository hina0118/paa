import { useState, useEffect, useCallback } from 'react';
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

export function Settings() {
  const { metadata, updateBatchSize, updateMaxIterations } = useSync();
  const {
    metadata: parseMetadata,
    updateBatchSize: updateParseBatchSize,
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
  const [geminiApiKeyStatus, setGeminiApiKeyStatus] = useState<
    'checking' | 'available' | 'unavailable' | 'error'
  >('checking');
  // SerpApi
  const [serpApiKey, setSerpApiKey] = useState<string>('');
  const [isSavingSerpApi, setIsSavingSerpApi] = useState(false);
  const [isDeletingSerpApi, setIsDeletingSerpApi] = useState(false);
  const [serpApiStatus, setSerpApiStatus] = useState<
    'checking' | 'available' | 'unavailable' | 'error'
  >('checking');

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

  const refreshGeminiStatus = useCallback(async () => {
    setGeminiApiKeyStatus('checking');
    try {
      const has = await invoke<boolean>('has_gemini_api_key');
      setGeminiApiKeyStatus(has ? 'available' : 'unavailable');
    } catch (error) {
      console.error('Failed to check Gemini API key status:', error);
      setGeminiApiKeyStatus('error');
    }
  }, []);

  useEffect(() => {
    refreshGeminiApiKeyStatus();
    refreshGeminiStatus();
  }, [refreshGeminiApiKeyStatus, refreshGeminiStatus]);

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
      await refreshGeminiStatus();
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
      await refreshGeminiStatus();
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
