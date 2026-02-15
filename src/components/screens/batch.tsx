import { useEffect } from 'react';
import { useSync } from '@/contexts/use-sync';
import { useParse } from '@/contexts/use-parse';
import { useNavigation } from '@/contexts/use-navigation';
import { formatDateTime } from '@/lib/utils';
import { toastError, formatError } from '@/lib/toast';
import { Button } from '@/components/ui/button';
import { BatchSection } from '@/components/ui/batch-section';
import {
  SYNC_STATUS_CONFIG,
  PARSE_STATUS_CONFIG,
} from '@/components/ui/status-badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Layers } from 'lucide-react';

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

  useEffect(() => {
    refreshSyncStatus();
    refreshParseStatus();
  }, [refreshSyncStatus, refreshParseStatus]);

  // --- Sync handlers ---
  const handleStartSync = async () => {
    try {
      await startSync();
    } catch (err) {
      toastError(`同期の開始に失敗しました: ${formatError(err)}`);
    }
  };

  const handleCancelSync = async () => {
    try {
      await cancelSync();
    } catch (err) {
      toastError(`同期の中止に失敗しました: ${formatError(err)}`);
    }
  };

  // --- Parse handlers ---
  const handleStartParse = async () => {
    try {
      await startParse();
    } catch (err) {
      toastError(`メールパースの開始に失敗しました: ${formatError(err)}`);
    }
  };

  const handleCancelParse = async () => {
    try {
      await cancelParse();
    } catch (err) {
      toastError(`パースの中止に失敗しました: ${formatError(err)}`);
    }
  };

  const handleStartProductNameParse = async () => {
    try {
      await startProductNameParse();
    } catch (err) {
      toastError(`商品名解析の開始に失敗しました: ${formatError(err)}`);
    }
  };

  return (
    <div className="container mx-auto py-10 px-6 space-y-6">
      <div className="mb-8 space-y-2">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-primary/10">
            <Layers className="h-6 w-6 text-primary" />
          </div>
          <h1 className="text-3xl font-bold tracking-tight">バッチ処理</h1>
        </div>
      </div>

      {/* 1. Gmail同期 */}
      <BatchSection
        title="1. Gmail同期"
        controlTitle="同期コントロール"
        controlDescription="Gmail からメールを段階的に取得します"
        isRunning={isSyncing}
        progress={syncProgress}
        onStart={handleStartSync}
        onCancel={handleCancelSync}
        startLabel={
          syncMetadata?.sync_status === 'paused' ? '同期を再開' : '同期を開始'
        }
        runningLabel="同期中..."
        startDisabled={isParsing || isProductNameParsing}
        completeMessage="同期が完了しました"
        progressTitle="同期進捗"
        showBatchNumber
        showCounts={false}
        status={syncMetadata?.sync_status}
        statusConfig={SYNC_STATUS_CONFIG}
        statistics={
          syncMetadata && (
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
          )
        }
      />

      {/* 2. メールパース */}
      <BatchSection
        title="2. メールパース"
        controlTitle="パースコントロール"
        controlDescription="データベースからメールを取得して注文情報をパースします"
        isRunning={isParsing}
        progress={parseProgress}
        onStart={handleStartParse}
        onCancel={handleCancelParse}
        startLabel="パースを開始"
        runningLabel="パース中..."
        startDisabled={isSyncing || isProductNameParsing}
        completeMessage="パースが完了しました"
        progressTitle="パース進捗"
        status={parseMetadata?.parse_status}
        statusConfig={PARSE_STATUS_CONFIG}
        confirmDialog={{
          title: 'パース処理の確認',
          description: (
            <div className="space-y-3 pt-2">
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
            </div>
          ),
          confirmLabel: '削除して実行',
        }}
        extraContent={
          <p className="text-sm text-muted-foreground">
            バッチサイズ: {parseMetadata?.batch_size || 100}件
            （設定画面で変更可能）
          </p>
        }
        statistics={
          parseMetadata && (
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
          )
        }
      />

      {/* 3. 商品名解析 (AI) */}
      <BatchSection
        title="3. 商品名解析 (AI)"
        controlTitle="商品名解析"
        controlDescription="Gemini APIを使用して商品名からメーカー情報を抽出します"
        isRunning={isProductNameParsing}
        progress={productNameProgress}
        onStart={handleStartProductNameParse}
        startLabel="商品名を解析"
        runningLabel="解析中..."
        startDisabled={
          isSyncing || isParsing || geminiApiKeyStatus !== 'available'
        }
        completeMessage="商品名解析が完了しました"
        progressTitle="解析進捗"
        extraContent={
          <>
            {geminiApiKeyStatus !== 'available' && (
              <div className="p-3 bg-amber-50 border border-amber-200 rounded text-sm text-amber-800">
                初回利用時は設定画面でGemini
                APIキーを設定してください。keyring（OSのセキュアストレージ）に保存されます。
                <Button
                  variant="link"
                  className="p-0 h-auto ml-1 text-amber-800 underline"
                  onClick={() => setCurrentScreen('api-keys')}
                >
                  APIキー設定へ →
                </Button>
              </div>
            )}
            <p className="text-sm text-muted-foreground">
              メールパース後に実行してください。10件ずつ処理し、間に10秒のディレイを入れます。
            </p>
          </>
        }
      />

      {/* Error Display */}
      {(syncProgress?.error ||
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
              {syncProgress?.error ||
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
