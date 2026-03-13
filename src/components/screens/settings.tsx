import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Settings as SettingsIcon } from 'lucide-react';
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
import { Checkbox } from '@/components/ui/checkbox';
import { Label } from '@/components/ui/label';
import { PageHeader } from '@/components/ui/page-header';
import { toastError } from '@/lib/toast';
import { useConfigSave } from '@/hooks/useConfigSave';

interface GeminiConfig {
  batch_size: number;
  delay_seconds: number;
}

interface SchedulerConfig {
  interval_minutes: number;
  enabled: boolean;
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
  const [schedulerEnabled, setSchedulerEnabled] = useState(true);
  const [schedulerInterval, setSchedulerInterval] = useState<string>('');
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
    const loadConfigs = async () => {
      const [geminiResult, schedulerResult] = await Promise.allSettled([
        invoke<GeminiConfig>('get_gemini_config'),
        invoke<SchedulerConfig>('get_scheduler_config'),
      ]);
      if (geminiResult.status === 'fulfilled') {
        setGeminiBatchSize(geminiResult.value.batch_size.toString());
        setGeminiDelaySeconds(geminiResult.value.delay_seconds.toString());
      } else {
        console.error('Failed to load Gemini config:', geminiResult.reason);
      }
      if (schedulerResult.status === 'fulfilled') {
        setSchedulerEnabled(schedulerResult.value.enabled);
        setSchedulerInterval(schedulerResult.value.interval_minutes.toString());
      } else {
        console.error(
          'Failed to load scheduler config:',
          schedulerResult.reason
        );
      }
    };
    loadConfigs();
  }, []);

  const batchSizeConfig = useConfigSave(async () => {
    const value = parseInt(batchSize, 10);
    if (isNaN(value) || value <= 0) {
      toastError('バッチサイズは1以上の整数を入力してください');
      return false;
    }
    await updateBatchSize(value);
  }, 'バッチサイズ');

  const maxIterationsConfig = useConfigSave(async () => {
    const value = parseInt(maxIterations, 10);
    if (isNaN(value) || value <= 0) {
      toastError('最大繰り返し回数は1以上の整数を入力してください');
      return false;
    }
    await updateMaxIterations(value);
  }, '最大繰り返し回数');

  const maxResultsPerPageConfig = useConfigSave(async () => {
    const value = parseInt(maxResultsPerPage, 10);
    if (isNaN(value) || value < 1 || value > 500) {
      toastError('1ページあたり取得件数は1〜500の範囲で入力してください');
      return false;
    }
    await updateMaxResultsPerPage(value);
  }, '1ページあたり取得件数');

  const timeoutMinutesConfig = useConfigSave(async () => {
    const value = parseInt(timeoutMinutes, 10);
    if (isNaN(value) || value < 1 || value > 120) {
      toastError('同期タイムアウトは1〜120分の範囲で入力してください');
      return false;
    }
    await updateTimeoutMinutes(value);
  }, '同期タイムアウト');

  const parseBatchSizeConfig = useConfigSave(async () => {
    const value = parseInt(parseBatchSize, 10);
    if (isNaN(value) || value <= 0) {
      toastError('パースバッチサイズは1以上の整数を入力してください');
      return false;
    }
    await updateParseBatchSize(value);
  }, 'パースバッチサイズ');

  const schedulerEnabledConfig = useConfigSave(async () => {
    await invoke('update_scheduler_enabled', { enabled: schedulerEnabled });
  }, 'スケジューラの有効/無効');

  const schedulerIntervalConfig = useConfigSave(async () => {
    const value = parseInt(schedulerInterval, 10);
    if (isNaN(value) || value < 1 || value > 10080) {
      toastError('実行間隔は1〜10080分（7日）の範囲で入力してください');
      return false;
    }
    await invoke('update_scheduler_interval', { intervalMinutes: value });
  }, 'スケジューラの実行間隔');

  const geminiBatchSizeConfig = useConfigSave(async () => {
    const value = parseInt(geminiBatchSize, 10);
    if (isNaN(value) || value < 1 || value > 50) {
      toastError('商品名パースのバッチサイズは1〜50の範囲で入力してください');
      return false;
    }
    await invoke('update_gemini_batch_size', { batchSize: value });
  }, '商品名パースのバッチサイズ');

  const geminiDelaySecondsConfig = useConfigSave(async () => {
    const value = parseInt(geminiDelaySeconds, 10);
    if (isNaN(value) || value < 0 || value > 60) {
      toastError('リクエスト間の待機秒数は0〜60の範囲で入力してください');
      return false;
    }
    await invoke('update_gemini_delay_seconds', { delaySeconds: value });
  }, 'リクエスト間の待機秒数');

  return (
    <div className="container mx-auto pt-0 pb-10 px-6 space-y-6">
      <PageHeader title="設定" icon={SettingsIcon} />

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
                disabled={batchSizeConfig.isSaving}
                className="max-w-xs"
              />
              <Button
                onClick={batchSizeConfig.save}
                disabled={batchSizeConfig.isSaving}
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
                disabled={maxIterationsConfig.isSaving}
                className="max-w-xs"
              />
              <Button
                onClick={maxIterationsConfig.save}
                disabled={maxIterationsConfig.isSaving}
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
                disabled={maxResultsPerPageConfig.isSaving}
                className="max-w-xs"
              />
              <Button
                onClick={maxResultsPerPageConfig.save}
                disabled={maxResultsPerPageConfig.isSaving}
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
                disabled={timeoutMinutesConfig.isSaving}
                className="max-w-xs"
              />
              <Button
                onClick={timeoutMinutesConfig.save}
                disabled={timeoutMinutesConfig.isSaving}
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
                disabled={geminiBatchSizeConfig.isSaving}
                className="max-w-xs"
              />
              <Button
                onClick={geminiBatchSizeConfig.save}
                disabled={geminiBatchSizeConfig.isSaving}
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
                disabled={geminiDelaySecondsConfig.isSaving}
                className="max-w-xs"
              />
              <Button
                onClick={geminiDelaySecondsConfig.save}
                disabled={geminiDelaySecondsConfig.isSaving}
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
                disabled={parseBatchSizeConfig.isSaving}
                className="max-w-xs"
              />
              <Button
                onClick={parseBatchSizeConfig.save}
                disabled={parseBatchSizeConfig.isSaving}
                aria-label="パースバッチサイズを保存"
              >
                保存
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>スケジューラ設定</CardTitle>
          <CardDescription>
            定期的なバックグラウンド処理（差分同期→メールパース→商品名解析→配達状況確認）を調整します
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="space-y-2">
            <div className="flex items-center gap-3">
              <Checkbox
                id="scheduler-enabled"
                checked={schedulerEnabled}
                onCheckedChange={(checked) =>
                  setSchedulerEnabled(checked === true)
                }
                disabled={schedulerEnabledConfig.isSaving}
              />
              <Label
                htmlFor="scheduler-enabled"
                className="text-sm font-medium"
              >
                スケジューラを有効にする
              </Label>
            </div>
            <p className="text-sm text-muted-foreground">
              有効にすると、アプリ起動中に自動でバッチ処理を定期実行します
            </p>
            <div>
              <Button
                onClick={schedulerEnabledConfig.save}
                disabled={schedulerEnabledConfig.isSaving}
                aria-label="スケジューラの有効/無効を保存"
              >
                保存
              </Button>
            </div>
          </div>

          <div className="space-y-2">
            <label htmlFor="scheduler-interval" className="text-sm font-medium">
              実行間隔（分）
            </label>
            <p className="text-sm text-muted-foreground">
              パイプラインの実行間隔 (1〜10080分、推奨: 1440 = 1日)
            </p>
            <div className="flex gap-2">
              <Input
                id="scheduler-interval"
                type="number"
                min="1"
                max="10080"
                value={schedulerInterval}
                onChange={(e) => setSchedulerInterval(e.target.value)}
                disabled={schedulerIntervalConfig.isSaving}
                className="max-w-xs"
              />
              <Button
                onClick={schedulerIntervalConfig.save}
                disabled={schedulerIntervalConfig.isSaving}
                aria-label="スケジューラの実行間隔を保存"
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
