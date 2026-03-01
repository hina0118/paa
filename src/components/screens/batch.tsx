import { useEffect } from 'react';
import { useSync } from '@/contexts/use-sync';
import { useParse } from '@/contexts/use-parse';
import { useNavigation } from '@/contexts/use-navigation';
import { toastError, formatError } from '@/lib/toast';
import { Button } from '@/components/ui/button';
import { BatchSection } from '@/components/ui/batch-section';
import {
  SYNC_STATUS_CONFIG,
  PARSE_STATUS_CONFIG,
} from '@/components/ui/status-badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { PageHeader } from '@/components/ui/page-header';
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
    <div className="container mx-auto pt-0 pb-10 px-6 space-y-6">
      <PageHeader title="バッチ処理" icon={Layers} />

      {/* 1. Gmail同期 */}
      <BatchSection
        title="1. Gmail同期"
        controlTitle="同期コントロール"
        controlDescription="Gmail からメールを段階的に取得します"
        isRunning={isSyncing}
        progress={syncProgress}
        onStart={handleStartSync}
        onCancel={handleCancelSync}
        cancelVariant="destructive"
        cancelLabel="中止"
        startLabel={
          syncMetadata?.sync_status === 'paused' ? '同期を再開' : '同期を開始'
        }
        runningLabel="同期中..."
        startDisabled={isParsing || isProductNameParsing}
        completeMessage="同期が完了しました"
        showBatchNumber
        showCounts={false}
        status={syncMetadata?.sync_status}
        statusConfig={SYNC_STATUS_CONFIG}
        progressTitle="同期進捗"
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
              <p className="font-semibold text-amber-700 dark:text-amber-400">
                ⚠️ 重要な確認事項
              </p>
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
              <p className="font-semibold text-destructive">
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
              <div className="p-3 bg-amber-500/10 border border-amber-500/20 rounded text-sm text-amber-700 dark:text-amber-400">
                初回利用時は設定画面でGemini
                APIキーを設定してください。keyring（OSのセキュアストレージ）に保存されます。
                <Button
                  variant="link"
                  className="p-0 h-auto ml-1 underline"
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
          className="border-destructive/30 bg-destructive/5"
          data-testid="error-message"
          role="alert"
        >
          <CardHeader>
            <CardTitle className="text-destructive">エラー</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-destructive">
              {syncProgress?.error ||
                parseProgress?.error ||
                parseMetadata?.last_error_message}
            </p>
          </CardContent>
        </Card>
      )}

      {/* Setup Instructions */}
      <Card className="bg-primary/5 border-primary/20">
        <CardHeader>
          <CardTitle className="text-primary">使い方</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2 text-sm text-foreground/80">
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
          <div className="mt-3 p-3 bg-amber-500/10 border border-amber-500/20 rounded">
            <p className="font-semibold text-amber-700 dark:text-amber-400 mb-1">
              初回認証について
            </p>
            <p className="text-xs text-amber-700 dark:text-amber-400">
              初回実行時は、ブラウザで認証画面が自動的に開きます。
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
