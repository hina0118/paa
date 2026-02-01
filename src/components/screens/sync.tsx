import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useSync } from '@/contexts/use-sync';
import { formatDateTime } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';

export function Sync() {
  const {
    isSyncing,
    progress,
    metadata,
    startSync,
    cancelSync,
    refreshStatus,
  } = useSync();
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  useEffect(() => {
    // Refresh status when component mounts or returns to sync screen
    refreshStatus();
  }, [refreshStatus]);

  const handleStartSync = async () => {
    setError(null);
    try {
      await startSync();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleCancelSync = async () => {
    try {
      await cancelSync();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleResetSyncDate = async () => {
    if (
      !confirm(
        '同期日時をリセットして、最新のメールから再度同期しますか？\nこれにより、新しい店舗設定や件名フィルターで過去のメールも取得できます。'
      )
    ) {
      return;
    }

    setError(null);
    setSuccessMessage(null);
    try {
      await invoke('reset_sync_date');
      setSuccessMessage(
        '同期日時をリセットしました。次回の同期から最新のメールが取得されます。'
      );
      setTimeout(() => setSuccessMessage(null), 5000);
      await refreshStatus();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const progressPercentage =
    progress?.total_synced && metadata?.total_synced_count
      ? Math.min(
          (progress.total_synced /
            Math.max(metadata.total_synced_count, progress.total_synced)) *
            100,
          100
        )
      : 0;

  const getStatusBadgeClass = (status?: string) => {
    switch (status) {
      case 'syncing':
        return 'bg-blue-100 text-blue-800';
      case 'idle':
        return 'bg-green-100 text-green-800';
      case 'paused':
        return 'bg-yellow-100 text-yellow-800';
      case 'error':
        return 'bg-red-100 text-red-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  const getStatusText = (status?: string) => {
    switch (status) {
      case 'syncing':
        return '同期中';
      case 'idle':
        return '待機中';
      case 'paused':
        return '一時停止';
      case 'error':
        return 'エラー';
      default:
        return '不明';
    }
  };

  return (
    <div className="container mx-auto py-10 space-y-6">
      <h1 className="text-3xl font-bold">Gmail同期</h1>

      {successMessage && (
        <div
          className="p-3 bg-green-50 border border-green-200 rounded text-sm text-green-800"
          data-testid="success-message"
          role="status"
        >
          {successMessage}
        </div>
      )}

      {error && (
        <div
          className="p-3 bg-red-50 border border-red-200 rounded text-sm text-red-800"
          data-testid="error-message"
          role="alert"
        >
          {error}
        </div>
      )}

      {/* Sync Controls */}
      <Card>
        <CardHeader>
          <CardTitle>同期コントロール</CardTitle>
          <CardDescription>
            Gmail からメールを段階的に取得します
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex gap-4">
            <Button
              onClick={handleStartSync}
              disabled={isSyncing}
              variant={isSyncing ? 'secondary' : 'default'}
            >
              {isSyncing
                ? '同期中...'
                : metadata?.sync_status === 'paused'
                  ? '同期を再開'
                  : '同期を開始'}
            </Button>

            {isSyncing && (
              <Button onClick={handleCancelSync} variant="destructive">
                中止
              </Button>
            )}

            {!isSyncing && (
              <Button onClick={handleResetSyncDate} variant="outline">
                同期日時をリセット
              </Button>
            )}
          </div>

          {/* Status Badge */}
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium">ステータス:</span>
            <span
              className={`px-2 py-1 rounded text-xs font-semibold ${getStatusBadgeClass(metadata?.sync_status)}`}
            >
              {getStatusText(metadata?.sync_status)}
            </span>
          </div>
        </CardContent>
      </Card>

      {/* Progress Display */}
      {(isSyncing || progress) && (
        <Card>
          <CardHeader>
            <CardTitle>同期進捗</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {progress && (
              <>
                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span>バッチ {progress.batch_number}</span>
                    <span>{progress.total_synced} 件取得済み</span>
                  </div>
                  <Progress value={progressPercentage} />
                </div>

                <div className="text-sm text-muted-foreground">
                  {progress.status_message}
                </div>

                {progress.is_complete && !progress.error && (
                  <div
                    className="p-3 bg-green-50 border border-green-200 rounded text-sm text-green-800"
                    data-testid="success-message"
                    role="status"
                  >
                    同期が完了しました
                  </div>
                )}
              </>
            )}
          </CardContent>
        </Card>
      )}

      {/* Sync Statistics */}
      {metadata && (
        <Card>
          <CardHeader>
            <CardTitle>同期統計</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <span className="text-muted-foreground">総取得件数:</span>
                <div className="text-2xl font-bold">
                  {metadata.total_synced_count}
                </div>
              </div>
              <div>
                <span className="text-muted-foreground">バッチサイズ:</span>
                <div className="text-2xl font-bold">
                  {metadata.batch_size}件
                </div>
              </div>
              {metadata.oldest_fetched_date && (
                <div className="col-span-2">
                  <span className="text-muted-foreground">最古メール日付:</span>
                  <div className="text-sm font-mono">
                    {formatDateTime(metadata.oldest_fetched_date)}
                  </div>
                </div>
              )}
              {metadata.last_sync_completed_at && (
                <div className="col-span-2">
                  <span className="text-muted-foreground">最終同期:</span>
                  <div className="text-sm">
                    {formatDateTime(metadata.last_sync_completed_at)}
                  </div>
                </div>
              )}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Error Display */}
      {(error || progress?.error) && (
        <Card
          className="border-red-200 bg-red-50"
          data-testid="error-message"
          role="alert"
        >
          <CardHeader>
            <CardTitle className="text-red-800">エラー</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-red-700">{error || progress?.error}</p>
          </CardContent>
        </Card>
      )}

      {/* Setup Instructions */}
      <Card className="bg-blue-50 border-blue-200">
        <CardHeader>
          <CardTitle className="text-blue-900">初回セットアップ</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2 text-sm text-blue-800">
          <p>
            Gmail APIを使用するには、事前にGoogle Cloud
            Consoleでの設定が必要です。
          </p>
          <p>
            詳細は README.md の「Gmail API
            セットアップ」セクションを参照してください。
          </p>
          <div className="mt-3 p-3 bg-yellow-50 border border-yellow-200 rounded">
            <p className="font-semibold text-yellow-800 mb-1">
              初回認証について
            </p>
            <p className="text-xs text-yellow-700">
              初回実行時は、ブラウザで認証画面が自動的に開きます。
              もし開かない場合は、コンソール（開発者ツール）に表示されるURLをコピーして、
              手動でブラウザで開いてください。
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
