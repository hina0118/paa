import { useEffect, useState } from 'react';
import { useSync } from '@/contexts/use-sync';
import { useParse } from '@/contexts/use-parse';
import { useNavigation } from '@/contexts/use-navigation';
import { formatDateTime } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import {
  SimpleBatchProgressBar,
  BatchProgressBar,
} from '@/components/ui/batch-progress-bar';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

export function Batch() {
  const {
    isSyncing,
    progress: syncProgress,
    metadata: syncMetadata,
    startSync,
    cancelSync,
    refreshStatus: refreshSyncStatus,
  } = useSync();
  const {
    isParsing,
    progress: parseProgress,
    metadata: parseMetadata,
    startParse,
    cancelParse,
    refreshStatus: refreshParseStatus,
    isProductNameParsing,
    productNameProgress,
    startProductNameParse,
    geminiApiKeyStatus,
  } = useParse();
  const { setCurrentScreen } = useNavigation();

  const [syncError, setSyncError] = useState<string | null>(null);
  const [parseError, setParseError] = useState<string | null>(null);
  const [showParseConfirmDialog, setShowParseConfirmDialog] = useState(false);
  const [productNameError, setProductNameError] = useState<string | null>(null);

  useEffect(() => {
    refreshSyncStatus();
    refreshParseStatus();
  }, [refreshSyncStatus, refreshParseStatus]);

  // --- Sync handlers ---
  const handleStartSync = async () => {
    setSyncError(null);
    try {
      await startSync();
    } catch (err) {
      setSyncError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleCancelSync = async () => {
    try {
      await cancelSync();
    } catch (err) {
      setSyncError(err instanceof Error ? err.message : String(err));
    }
  };

  // --- Parse handlers ---
  const handleStartParse = () => setShowParseConfirmDialog(true);

  const handleConfirmParse = async () => {
    setShowParseConfirmDialog(false);
    setParseError(null);
    try {
      await startParse();
    } catch (err) {
      setParseError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleCancelParse = async () => {
    try {
      await cancelParse();
    } catch (err) {
      setParseError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleStartProductNameParse = async () => {
    setProductNameError(null);
    try {
      await startProductNameParse();
    } catch (err) {
      setProductNameError(err instanceof Error ? err.message : String(err));
    }
  };

  const getSyncStatusBadgeClass = (status?: string) => {
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

  const getSyncStatusText = (status?: string) => {
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

  const getParseStatusBadgeClass = (status?: string) => {
    switch (status) {
      case 'running':
        return 'bg-blue-100 text-blue-800';
      case 'idle':
        return 'bg-green-100 text-green-800';
      case 'completed':
        return 'bg-green-100 text-green-800';
      case 'error':
        return 'bg-red-100 text-red-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  const getParseStatusText = (status?: string) => {
    switch (status) {
      case 'running':
        return 'パース中';
      case 'idle':
        return '待機中';
      case 'completed':
        return '完了';
      case 'error':
        return 'エラー';
      default:
        return '不明';
    }
  };

  return (
    <div className="container mx-auto py-10 space-y-6">
      <h1 className="text-3xl font-bold">バッチ処理</h1>

      {/* Parse Confirmation Dialog */}
      <Dialog
        open={showParseConfirmDialog}
        onOpenChange={setShowParseConfirmDialog}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>パース処理の確認</DialogTitle>
            <DialogDescription className="space-y-3 pt-2">
              <p className="font-semibold text-yellow-800">⚠️ 重要な確認事項</p>
              <p>
                パース処理を開始すると、以下のテーブルの全データが削除されます：
              </p>
              <ul className="list-disc list-inside space-y-1 ml-2 text-sm">
                <li>注文情報（orders）</li>
                <li>商品情報（items）</li>
                <li>配送情報（deliveries）</li>
                <li>注文とメールの紐付け（order_emails）</li>
              </ul>
              <p className="text-sm">
                この操作は、パーサーの更新時にデータを再作成するために必要です。
              </p>
              <p className="font-semibold text-red-700">
                削除されたデータは復元できません。本当に実行しますか？
              </p>
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowParseConfirmDialog(false)}
            >
              キャンセル
            </Button>
            <Button variant="destructive" onClick={handleConfirmParse}>
              削除して実行
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Sync Section */}
      <section className="space-y-4">
        <h2 className="text-xl font-semibold">1. Gmail同期</h2>

        {syncError && (
          <div
            className="p-3 bg-red-50 border border-red-200 rounded text-sm text-red-800"
            data-testid="sync-error-message"
            role="alert"
          >
            {syncError}
          </div>
        )}

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
                  : syncMetadata?.sync_status === 'paused'
                    ? '同期を再開'
                    : '同期を開始'}
              </Button>
              {isSyncing && (
                <Button onClick={handleCancelSync} variant="destructive">
                  中止
                </Button>
              )}
            </div>
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium">ステータス:</span>
              <span
                className={`px-2 py-1 rounded text-xs font-semibold ${getSyncStatusBadgeClass(syncMetadata?.sync_status)}`}
              >
                {getSyncStatusText(syncMetadata?.sync_status)}
              </span>
            </div>
          </CardContent>
        </Card>

        {(isSyncing || syncProgress) && (
          <Card>
            <CardHeader>
              <CardTitle>同期進捗</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              {syncProgress && (
                <>
                  <SimpleBatchProgressBar progress={syncProgress} />
                  {syncProgress.is_complete && !syncProgress.error && (
                    <div
                      className="p-3 bg-green-50 border border-green-200 rounded text-sm text-green-800"
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

        {syncMetadata && (
          <Card>
            <CardHeader>
              <CardTitle>同期統計</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-muted-foreground">総取得件数:</span>
                  <div className="text-2xl font-bold">
                    {syncMetadata.total_synced_count}
                  </div>
                </div>
                <div>
                  <span className="text-muted-foreground">バッチサイズ:</span>
                  <div className="text-2xl font-bold">
                    {syncMetadata.batch_size}件
                  </div>
                </div>
                {syncMetadata.last_sync_completed_at && (
                  <div className="col-span-2">
                    <span className="text-muted-foreground">最終同期:</span>
                    <div className="text-sm">
                      {formatDateTime(syncMetadata.last_sync_completed_at)}
                    </div>
                  </div>
                )}
              </div>
            </CardContent>
          </Card>
        )}
      </section>

      {/* Parse Section */}
      <section className="space-y-4">
        <h2 className="text-xl font-semibold">2. メールパース</h2>

        <Card>
          <CardHeader>
            <CardTitle>パースコントロール</CardTitle>
            <CardDescription>
              データベースからメールを取得して注文情報をパースします
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex gap-4">
              <Button
                onClick={handleStartParse}
                disabled={isParsing}
                variant={isParsing ? 'secondary' : 'default'}
              >
                {isParsing ? 'パース中...' : 'パースを開始'}
              </Button>
              {isParsing && (
                <Button onClick={handleCancelParse} variant="outline">
                  キャンセル
                </Button>
              )}
            </div>
            <p className="text-sm text-muted-foreground">
              バッチサイズ: {parseMetadata?.batch_size || 100}件
              （設定画面で変更可能）
            </p>
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium">ステータス:</span>
              <span
                className={`px-2 py-1 rounded text-xs font-semibold ${getParseStatusBadgeClass(parseMetadata?.parse_status)}`}
              >
                {getParseStatusText(parseMetadata?.parse_status)}
              </span>
            </div>
          </CardContent>
        </Card>

        {(isParsing || parseProgress) && (
          <Card>
            <CardHeader>
              <CardTitle>パース進捗</CardTitle>
            </CardHeader>
            <CardContent>
              {parseProgress && (
                <BatchProgressBar
                  progress={parseProgress}
                  completeMessage="パースが完了しました"
                />
              )}
            </CardContent>
          </Card>
        )}

        {parseMetadata && (
          <Card>
            <CardHeader>
              <CardTitle>パース統計</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-muted-foreground">総パース件数:</span>
                  <div className="text-2xl font-bold">
                    {parseMetadata.total_parsed_count}
                  </div>
                </div>
                {parseMetadata.last_parse_started_at && (
                  <div>
                    <span className="text-muted-foreground">開始日時:</span>
                    <div className="text-sm">
                      {formatDateTime(parseMetadata.last_parse_started_at)}
                    </div>
                  </div>
                )}
                {parseMetadata.last_parse_completed_at && (
                  <div className="col-span-2">
                    <span className="text-muted-foreground">最終完了:</span>
                    <div className="text-sm">
                      {formatDateTime(parseMetadata.last_parse_completed_at)}
                    </div>
                  </div>
                )}
              </div>
            </CardContent>
          </Card>
        )}
      </section>

      {/* Product Name Parse Section */}
      <section className="space-y-4">
        <h2 className="text-xl font-semibold">3. 商品名解析 (AI)</h2>

        <Card>
          <CardHeader>
            <CardTitle>商品名解析</CardTitle>
            <CardDescription>
              Gemini APIを使用して商品名からメーカー情報を抽出します
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {geminiApiKeyStatus !== 'available' && (
              <div className="p-3 bg-amber-50 border border-amber-200 rounded text-sm text-amber-800">
                初回利用時は設定画面でGemini
                APIキーを設定してください。keyring（OSのセキュアストレージ）に保存されます。
                <Button
                  variant="link"
                  className="p-0 h-auto ml-1 text-amber-800 underline"
                  onClick={() => setCurrentScreen('settings')}
                >
                  設定へ →
                </Button>
              </div>
            )}
            <div className="flex gap-4">
              <Button
                onClick={handleStartProductNameParse}
                disabled={
                  isProductNameParsing ||
                  isParsing ||
                  geminiApiKeyStatus !== 'available'
                }
                variant={isProductNameParsing ? 'secondary' : 'default'}
              >
                {isProductNameParsing ? '解析中...' : '商品名を解析'}
              </Button>
            </div>
            <p className="text-sm text-muted-foreground">
              メールパース後に実行してください。10件ずつ処理し、間に10秒のディレイを入れます。
            </p>
            {(isProductNameParsing || productNameProgress) &&
              productNameProgress && (
                <div className="pt-4 border-t">
                  <BatchProgressBar
                    progress={productNameProgress}
                    completeMessage="商品名解析が完了しました"
                  />
                </div>
              )}
            {productNameError && (
              <div className="p-3 bg-red-50 border border-red-200 rounded text-sm text-red-800">
                {productNameError}
              </div>
            )}
          </CardContent>
        </Card>
      </section>

      {/* Error Display */}
      {(syncError ||
        syncProgress?.error ||
        parseError ||
        parseProgress?.error ||
        parseMetadata?.last_error_message) && (
        <Card
          className="border-red-200 bg-red-50"
          data-testid="error-message"
          role="alert"
        >
          <CardHeader>
            <CardTitle className="text-red-800">エラー</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-red-700">
              {syncError ||
                syncProgress?.error ||
                parseError ||
                parseProgress?.error ||
                parseMetadata?.last_error_message}
            </p>
          </CardContent>
        </Card>
      )}

      {/* Setup Instructions */}
      <Card className="bg-blue-50 border-blue-200">
        <CardHeader>
          <CardTitle className="text-blue-900">使い方</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2 text-sm text-blue-800">
          <p>バッチ処理は以下の順序で実行します：</p>
          <ol className="list-decimal list-inside space-y-1 ml-2">
            <li>Gmail同期でメールを取得</li>
            <li>メールパースで注文情報を抽出</li>
            <li>商品名解析（AI）でメーカー情報を抽出</li>
          </ol>
          <p>
            Gmail APIを使用するには、事前にGoogle Cloud
            Consoleでの設定が必要です。詳細は README.md を参照してください。
          </p>
          <div className="mt-3 p-3 bg-yellow-50 border border-yellow-200 rounded">
            <p className="font-semibold text-yellow-800 mb-1">
              初回認証について
            </p>
            <p className="text-xs text-yellow-700">
              初回実行時は、ブラウザで認証画面が自動的に開きます。
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
