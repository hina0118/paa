import { useState, useEffect } from 'react';
import { useSync } from '@/contexts/sync-context';
import { useParse } from '@/contexts/parse-context';
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
  const { metadata: parseMetadata, updateBatchSize: updateParseBatchSize } =
    useParse();
  const [batchSize, setBatchSize] = useState<string>('');
  const [maxIterations, setMaxIterations] = useState<string>('');
  const [parseBatchSize, setParseBatchSize] = useState<string>('');
  const [isSavingBatchSize, setIsSavingBatchSize] = useState(false);
  const [isSavingMaxIterations, setIsSavingMaxIterations] = useState(false);
  const [isSavingParseBatchSize, setIsSavingParseBatchSize] = useState(false);
  const [successMessage, setSuccessMessage] = useState<string>('');
  const [errorMessage, setErrorMessage] = useState<string>('');
  const [isInitialized, setIsInitialized] = useState(false);

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

  return (
    <div className="container mx-auto py-10 space-y-6">
      <h1 className="text-3xl font-bold">設定</h1>

      {successMessage && (
        <div className="p-3 bg-green-50 border border-green-200 rounded text-sm text-green-800">
          {successMessage}
        </div>
      )}

      {errorMessage && (
        <div className="p-3 bg-red-50 border border-red-200 rounded text-sm text-red-800">
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
