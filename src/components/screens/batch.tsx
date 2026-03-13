import { useEffect } from 'react';
import { useSync } from '@/contexts/use-sync';
import { useParse } from '@/contexts/use-parse';
import { useDeliveryCheck } from '@/contexts/use-delivery-check';
import { useSurugayaSession } from '@/contexts/use-surugaya-session';
import { useFullParsePipeline } from '@/contexts/use-full-parse-pipeline';
import { PIPELINE_STEP_LABELS } from '@/contexts/full-parse-pipeline-context-value';
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
import { Layers, PlayCircle } from 'lucide-react';

export function Batch() {
  const {
    isChecking,
    progress: deliveryCheckProgress,
    startDeliveryCheck,
    cancelDeliveryCheck,
  } = useDeliveryCheck();
  const {
    isFetching,
    progress: surugayaProgress,
    openLoginWindow,
    startFetch,
    cancelFetch,
  } = useSurugayaSession();
  const {
    isSyncing,
    progress: syncProgress,
    metadata: syncMetadata,
    startSync,
    startIncrementalSync,
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
  const {
    isRunning: isPipelineRunning,
    currentStep: pipelineCurrentStep,
    startPipeline,
  } = useFullParsePipeline();

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

  const handleStartIncrementalSync = async () => {
    try {
      await startIncrementalSync();
    } catch (err) {
      toastError(`差分同期の開始に失敗しました: ${formatError(err)}`);
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

  const handleStartDeliveryCheck = async () => {
    try {
      await startDeliveryCheck();
    } catch (err) {
      toastError(`配送状況確認の開始に失敗しました: ${formatError(err)}`);
    }
  };

  const handleCancelDeliveryCheck = async () => {
    try {
      await cancelDeliveryCheck();
    } catch (err) {
      toastError(`配送状況確認の中止に失敗しました: ${formatError(err)}`);
    }
  };

  // --- Full parse pipeline handler ---
  const handleStartPipeline = async () => {
    try {
      await startPipeline();
    } catch (err) {
      toastError(`一括パースの開始に失敗しました: ${formatError(err)}`);
    }
  };

  // --- Surugaya mypage fetch handlers ---
  const handleOpenSurugayaLogin = async () => {
    try {
      await openLoginWindow();
    } catch (err) {
      toastError(`ログインウィンドウの起動に失敗しました: ${formatError(err)}`);
    }
  };

  const handleStartSurugayaFetch = async () => {
    try {
      await startFetch();
    } catch (err) {
      toastError(
        `駿河屋マイページ取得の開始に失敗しました: ${formatError(err)}`
      );
    }
  };

  const handleCancelSurugayaFetch = async () => {
    try {
      await cancelFetch();
    } catch (err) {
      toastError(
        `駿河屋マイページ取得の中止に失敗しました: ${formatError(err)}`
      );
    }
  };

  return (
    <div className="container mx-auto pt-0 pb-10 px-6 space-y-6">
      <PageHeader title="バッチ処理" icon={Layers} />

      {/* 0. 一括パース実行 */}
      <Card className="border-primary/30 bg-primary/5">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-primary">
            <PlayCircle className="h-5 w-5" />
            一括パース実行
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <p className="text-sm text-muted-foreground">
            メールパース → 駿河屋HTMLパース → 商品名解析 → 配送状況確認
            をまとめて順番に実行します。
            各ステップの成否に関わらず次のステップへ進みます。
          </p>
          {isPipelineRunning && pipelineCurrentStep && (
            <p className="text-sm font-medium text-primary">
              実行中: {PIPELINE_STEP_LABELS[pipelineCurrentStep]}...
            </p>
          )}
          <Button
            onClick={handleStartPipeline}
            disabled={
              isPipelineRunning ||
              isSyncing ||
              isParsing ||
              isProductNameParsing ||
              isChecking ||
              isFetching
            }
            className="w-full sm:w-auto"
          >
            {isPipelineRunning ? '実行中...' : '一括パースを実行'}
          </Button>
        </CardContent>
      </Card>

      {/* 1. Gmail同期 */}
      <BatchSection
        title="1. Gmail同期"
        controlTitle="同期コントロール"
        controlDescription="Gmail からメールを取得します"
        isRunning={isSyncing}
        progress={syncProgress}
        onStart={handleStartIncrementalSync}
        onCancel={handleCancelSync}
        cancelVariant="destructive"
        cancelLabel="中止"
        startLabel="差分同期"
        runningLabel="同期中..."
        startDisabled={isPipelineRunning || isParsing || isProductNameParsing}
        completeMessage="同期が完了しました"
        showBatchNumber
        showCounts={false}
        status={syncMetadata?.sync_status}
        statusConfig={SYNC_STATUS_CONFIG}
        progressTitle="同期進捗"
        extraContent={
          <div className="space-y-2">
            <Button
              onClick={handleStartSync}
              disabled={
                isPipelineRunning ||
                isSyncing ||
                isParsing ||
                isProductNameParsing
              }
              variant="outline"
              size="sm"
            >
              全件同期
            </Button>
            <p className="text-xs text-muted-foreground">
              差分同期は最新の受信日時以降のメールのみ取得します。
              全件同期は全期間のメールを取得します。
            </p>
          </div>
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
        startDisabled={isPipelineRunning || isSyncing || isProductNameParsing}
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
          isPipelineRunning ||
          isSyncing ||
          isParsing ||
          geminiApiKeyStatus !== 'available'
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

      {/* 4. 配送状況確認 */}
      <BatchSection
        title="4. 配送状況確認"
        controlTitle="配送状況確認コントロール"
        controlDescription="各配送業者のHPにアクセスして追跡番号の現在状況を確認します"
        isRunning={isChecking}
        progress={deliveryCheckProgress}
        onStart={handleStartDeliveryCheck}
        onCancel={handleCancelDeliveryCheck}
        startLabel="配送状況を確認"
        runningLabel="確認中..."
        startDisabled={
          isPipelineRunning || isSyncing || isParsing || isProductNameParsing
        }
        completeMessage="配送状況確認が完了しました"
        progressTitle="確認進捗"
        extraContent={
          <p className="text-sm text-muted-foreground">
            未配達の荷物の追跡番号で配送業者のHPを確認します。
            追跡情報が見つからない場合は配送ステータスを「配達完了」として更新します。
            未対応の配送業者の場合は、その旨をチェック結果として記録し、配送ステータスは変更しません。
            バッチ間に3秒のインターバルを設けています。
          </p>
        }
      />

      {/* 5. 駿河屋マイページHTML取得 */}
      <BatchSection
        title="5. 駿河屋マイページHTML取得"
        controlTitle="マイページ取得コントロール"
        controlDescription="駿河屋マイページにアクセスして注文HTMLを取得します"
        isRunning={isFetching}
        progress={surugayaProgress}
        onStart={handleStartSurugayaFetch}
        onCancel={handleCancelSurugayaFetch}
        startLabel="取得開始"
        runningLabel="取得中..."
        startDisabled={
          isPipelineRunning ||
          isSyncing ||
          isParsing ||
          isProductNameParsing ||
          isChecking
        }
        completeMessage="マイページHTML取得が完了しました"
        progressTitle="取得進捗"
        showBatchNumber={false}
        showCounts={false}
        extraContent={
          <div className="space-y-2">
            <Button
              onClick={handleOpenSurugayaLogin}
              disabled={isFetching}
              variant="outline"
              size="sm"
            >
              ログインウィンドウを開く
            </Button>
            <p className="text-xs text-muted-foreground">
              まずログインウィンドウを開いて駿河屋にログインしてから、取得開始を押してください。
            </p>
          </div>
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
            <li>配送状況確認で各荷物の現在状況を記録</li>
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
