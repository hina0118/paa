import { useState, useEffect } from 'react';
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

interface GeminiConfig {
  batch_size: number;
  delay_seconds: number;
}

export function Settings() {
  const {
    metadata,
    updateBatchSize,
    updateMaxIterations,
    updateMaxResultsPerPage,
    updateTimeoutMinutes,
  } = useSync();
  const { metadata: parseMetadata, updateBatchSize: updateParseBatchSize } =
    useParse();
  const [batchSize, setBatchSize] = useState<string>('');
  const [maxIterations, setMaxIterations] = useState<string>('');
  const [maxResultsPerPage, setMaxResultsPerPage] = useState<string>('');
  const [timeoutMinutes, setTimeoutMinutes] = useState<string>('');
  const [parseBatchSize, setParseBatchSize] = useState<string>('');
  const [geminiBatchSize, setGeminiBatchSize] = useState<string>('');
  const [geminiDelaySeconds, setGeminiDelaySeconds] = useState<string>('');
  const [isSavingBatchSize, setIsSavingBatchSize] = useState(false);
  const [isSavingMaxIterations, setIsSavingMaxIterations] = useState(false);
  const [isSavingMaxResultsPerPage, setIsSavingMaxResultsPerPage] =
    useState(false);
  const [isSavingTimeoutMinutes, setIsSavingTimeoutMinutes] = useState(false);
  const [isSavingParseBatchSize, setIsSavingParseBatchSize] = useState(false);
  const [isSavingGeminiBatchSize, setIsSavingGeminiBatchSize] = useState(false);
  const [isSavingGeminiDelaySeconds, setIsSavingGeminiDelaySeconds] =
    useState(false);
  const [successMessage, setSuccessMessage] = useState<string>('');
  const [errorMessage, setErrorMessage] = useState<string>('');
  const [isInitialized, setIsInitialized] = useState(false);

  useEffect(() => {
    if (metadata && !isInitialized) {
      setBatchSize(metadata.batch_size.toString());
      setMaxIterations(metadata.max_iterations.toString());
      setMaxResultsPerPage(metadata.max_results_per_page.toString());
      setTimeoutMinutes(metadata.timeout_minutes.toString());
      setIsInitialized(true);
    }
  }, [metadata, isInitialized]);

  useEffect(() => {
    if (parseMetadata) {
      setParseBatchSize(parseMetadata.batch_size.toString());
    }
  }, [parseMetadata]);

  useEffect(() => {
    const loadGeminiConfig = async () => {
      try {
        const config = await invoke<GeminiConfig>('get_gemini_config');
        setGeminiBatchSize(config.batch_size.toString());
        setGeminiDelaySeconds(config.delay_seconds.toString());
      } catch (error) {
        console.error('Failed to load Gemini config:', error);
      }
    };
    loadGeminiConfig();
  }, []);

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

  const handleSaveMaxResultsPerPage = async () => {
    const value = parseInt(maxResultsPerPage, 10);
    if (isNaN(value) || value < 1 || value > 500) {
      setErrorMessage('1ページあたり取得件数は1〜500の範囲で入力してください');
      return;
    }

    setIsSavingMaxResultsPerPage(true);
    setErrorMessage('');
    setSuccessMessage('');

    try {
      await updateMaxResultsPerPage(value);
      setSuccessMessage('1ページあたり取得件数を更新しました');
      setTimeout(() => setSuccessMessage(''), 3000);
    } catch (error) {
      setErrorMessage(
        `更新に失敗しました: ${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      setIsSavingMaxResultsPerPage(false);
    }
  };

  const handleSaveTimeoutMinutes = async () => {
    const value = parseInt(timeoutMinutes, 10);
    if (isNaN(value) || value < 1 || value > 120) {
      setErrorMessage('同期タイムアウトは1〜120分の範囲で入力してください');
      return;
    }

    setIsSavingTimeoutMinutes(true);
    setErrorMessage('');
    setSuccessMessage('');

    try {
      await updateTimeoutMinutes(value);
      setSuccessMessage('同期タイムアウトを更新しました');
      setTimeout(() => setSuccessMessage(''), 3000);
    } catch (error) {
      setErrorMessage(
        `更新に失敗しました: ${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      setIsSavingTimeoutMinutes(false);
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

  const handleSaveGeminiBatchSize = async () => {
    const value = parseInt(geminiBatchSize, 10);
    if (isNaN(value) || value < 1 || value > 50) {
      setErrorMessage(
        '商品名パースのバッチサイズは1〜50の範囲で入力してください'
      );
      return;
    }

    setIsSavingGeminiBatchSize(true);
    setErrorMessage('');
    setSuccessMessage('');

    try {
      await invoke('update_gemini_batch_size', { batchSize: value });
      setSuccessMessage('商品名パースのバッチサイズを更新しました');
      setTimeout(() => setSuccessMessage(''), 3000);
    } catch (error) {
      setErrorMessage(
        `更新に失敗しました: ${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      setIsSavingGeminiBatchSize(false);
    }
  };

  const handleSaveGeminiDelaySeconds = async () => {
    const value = parseInt(geminiDelaySeconds, 10);
    if (isNaN(value) || value < 0 || value > 60) {
      setErrorMessage('リクエスト間の待機秒数は0〜60の範囲で入力してください');
      return;
    }

    setIsSavingGeminiDelaySeconds(true);
    setErrorMessage('');
    setSuccessMessage('');

    try {
      await invoke('update_gemini_delay_seconds', { delaySeconds: value });
      setSuccessMessage('リクエスト間の待機秒数を更新しました');
      setTimeout(() => setSuccessMessage(''), 3000);
    } catch (error) {
      setErrorMessage(
        `更新に失敗しました: ${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      setIsSavingGeminiDelaySeconds(false);
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

          <div className="space-y-2">
            <label
              htmlFor="max-results-per-page"
              className="text-sm font-medium"
            >
              1ページあたり取得件数
            </label>
            <p className="text-sm text-muted-foreground">
              Gmail API の1回のリクエストで取得するメール件数 (1-500、推奨: 100)
            </p>
            <div className="flex gap-2">
              <Input
                id="max-results-per-page"
                type="number"
                min="1"
                max="500"
                value={maxResultsPerPage}
                onChange={(e) => setMaxResultsPerPage(e.target.value)}
                disabled={isSavingMaxResultsPerPage}
                className="max-w-xs"
              />
              <Button
                onClick={handleSaveMaxResultsPerPage}
                disabled={isSavingMaxResultsPerPage}
                aria-label="1ページあたり取得件数を保存"
              >
                保存
              </Button>
            </div>
          </div>

          <div className="space-y-2">
            <label htmlFor="timeout-minutes" className="text-sm font-medium">
              同期タイムアウト（分）
            </label>
            <p className="text-sm text-muted-foreground">
              同期処理の最大実行時間 (1-120分、推奨: 30)
            </p>
            <div className="flex gap-2">
              <Input
                id="timeout-minutes"
                type="number"
                min="1"
                max="120"
                value={timeoutMinutes}
                onChange={(e) => setTimeoutMinutes(e.target.value)}
                disabled={isSavingTimeoutMinutes}
                className="max-w-xs"
              />
              <Button
                onClick={handleSaveTimeoutMinutes}
                disabled={isSavingTimeoutMinutes}
                aria-label="同期タイムアウトを保存"
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
          <CardTitle>Gemini設定</CardTitle>
          <CardDescription>
            商品名解析（Gemini API）の動作を調整します
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="space-y-2">
            <label htmlFor="gemini-batch-size" className="text-sm font-medium">
              バッチサイズ
            </label>
            <p className="text-sm text-muted-foreground">
              1リクエストあたりの商品数 (1-50、推奨: 10)
            </p>
            <div className="flex gap-2">
              <Input
                id="gemini-batch-size"
                type="number"
                min="1"
                max="50"
                value={geminiBatchSize}
                onChange={(e) => setGeminiBatchSize(e.target.value)}
                disabled={isSavingGeminiBatchSize}
                className="max-w-xs"
              />
              <Button
                onClick={handleSaveGeminiBatchSize}
                disabled={isSavingGeminiBatchSize}
                aria-label="商品名パースのバッチサイズを保存"
              >
                保存
              </Button>
            </div>
          </div>

          <div className="space-y-2">
            <label
              htmlFor="gemini-delay-seconds"
              className="text-sm font-medium"
            >
              リクエスト間の待機秒数
            </label>
            <p className="text-sm text-muted-foreground">
              レート制限対策の待機時間 (0-60秒、推奨: 10)
            </p>
            <div className="flex gap-2">
              <Input
                id="gemini-delay-seconds"
                type="number"
                min="0"
                max="60"
                value={geminiDelaySeconds}
                onChange={(e) => setGeminiDelaySeconds(e.target.value)}
                disabled={isSavingGeminiDelaySeconds}
                className="max-w-xs"
              />
              <Button
                onClick={handleSaveGeminiDelaySeconds}
                disabled={isSavingGeminiDelaySeconds}
                aria-label="リクエスト間の待機秒数を保存"
              >
                保存
              </Button>
            </div>
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
