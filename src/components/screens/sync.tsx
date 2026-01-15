import { useEffect, useState } from "react";
import { useSync } from "@/contexts/sync-context";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export function Sync() {
  const { isSyncing, progress, metadata, startSync, cancelSync, refreshStatus } = useSync();
  const [error, setError] = useState<string | null>(null);

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

  const progressPercentage = progress?.totalSynced && metadata?.totalSyncedCount
    ? Math.min((progress.totalSynced / Math.max(metadata.totalSyncedCount, progress.totalSynced)) * 100, 100)
    : 0;

  const getStatusBadgeClass = (status?: string) => {
    switch (status) {
      case "syncing":
        return "bg-blue-100 text-blue-800";
      case "idle":
        return "bg-green-100 text-green-800";
      case "paused":
        return "bg-yellow-100 text-yellow-800";
      case "error":
        return "bg-red-100 text-red-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  const getStatusText = (status?: string) => {
    switch (status) {
      case "syncing":
        return "同期中";
      case "idle":
        return "待機中";
      case "paused":
        return "一時停止";
      case "error":
        return "エラー";
      default:
        return "不明";
    }
  };

  return (
    <div className="container mx-auto py-10 space-y-6">
      <h1 className="text-3xl font-bold">Gmail同期</h1>

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
              variant={isSyncing ? "secondary" : "default"}
            >
              {isSyncing ? "同期中..." : metadata?.syncStatus === "paused" ? "同期を再開" : "同期を開始"}
            </Button>

            {isSyncing && (
              <Button
                onClick={handleCancelSync}
                variant="destructive"
              >
                中止
              </Button>
            )}
          </div>

          {/* Status Badge */}
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium">ステータス:</span>
            <span className={`px-2 py-1 rounded text-xs font-semibold ${getStatusBadgeClass(metadata?.syncStatus)}`}>
              {getStatusText(metadata?.syncStatus)}
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
                    <span>バッチ {progress.batchNumber}</span>
                    <span>{progress.totalSynced} 件取得済み</span>
                  </div>
                  <Progress value={progressPercentage} />
                </div>

                <div className="text-sm text-muted-foreground">
                  {progress.statusMessage}
                </div>

                {progress.isComplete && !progress.error && (
                  <div className="p-3 bg-green-50 border border-green-200 rounded text-sm text-green-800">
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
                <div className="text-2xl font-bold">{metadata.totalSyncedCount}</div>
              </div>
              <div>
                <span className="text-muted-foreground">バッチサイズ:</span>
                <div className="text-2xl font-bold">{metadata.batchSize}件</div>
              </div>
              {metadata.oldestFetchedDate && (
                <div className="col-span-2">
                  <span className="text-muted-foreground">最古メール日付:</span>
                  <div className="text-sm font-mono">
                    {new Date(metadata.oldestFetchedDate).toLocaleString("ja-JP")}
                  </div>
                </div>
              )}
              {metadata.lastSyncCompletedAt && (
                <div className="col-span-2">
                  <span className="text-muted-foreground">最終同期:</span>
                  <div className="text-sm">
                    {new Date(metadata.lastSyncCompletedAt).toLocaleString("ja-JP")}
                  </div>
                </div>
              )}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Error Display */}
      {(error || progress?.error) && (
        <Card className="border-red-200 bg-red-50">
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
          <p>Gmail APIを使用するには、事前にGoogle Cloud Consoleでの設定が必要です。</p>
          <p>詳細は README.md の「Gmail API セットアップ」セクションを参照してください。</p>
          <div className="mt-3 p-3 bg-yellow-50 border border-yellow-200 rounded">
            <p className="font-semibold text-yellow-800 mb-1">初回認証について</p>
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
