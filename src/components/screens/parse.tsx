import { useEffect, useState } from 'react';
import { useParse } from '@/contexts/use-parse';
import { useNavigation } from '@/contexts/use-navigation';
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
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

export function Parse() {
  const {
    isParsing,
    progress,
    metadata,
    startParse,
    cancelParse,
    refreshStatus,
    isProductNameParsing,
    productNameProgress,
    startProductNameParse,
    hasGeminiApiKey,
  } = useParse();
  const { setCurrentScreen } = useNavigation();
  const [error, setError] = useState<string | null>(null);
  const [showConfirmDialog, setShowConfirmDialog] = useState(false);
  const [productNameError, setProductNameError] = useState<string | null>(null);

  useEffect(() => {
    // Refresh status when component mounts
    refreshStatus();
  }, [refreshStatus]);

  const handleStartParse = () => {
    // 確認ダイアログを表示
    setShowConfirmDialog(true);
  };

  const handleConfirmParse = async () => {
    setShowConfirmDialog(false);
    setError(null);
    try {
      // batch_sizeを指定せずに呼び出すとparse_metadataの値が使われる
      await startParse();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleCancelDialog = () => {
    setShowConfirmDialog(false);
  };

  const handleCancelParse = async () => {
    try {
      await cancelParse();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
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

  const progressPercentage =
    progress?.total_emails && progress.parsed_count
      ? Math.min((progress.parsed_count / progress.total_emails) * 100, 100)
      : 0;

  const getStatusBadgeClass = (status?: string) => {
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

  const getStatusText = (status?: string) => {
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
      <h1 className="text-3xl font-bold">メールパース</h1>

      {/* Confirmation Dialog */}
      <Dialog open={showConfirmDialog} onOpenChange={setShowConfirmDialog}>
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
            <Button variant="outline" onClick={handleCancelDialog}>
              キャンセル
            </Button>
            <Button variant="destructive" onClick={handleConfirmParse}>
              削除して実行
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Parse Controls */}
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
            バッチサイズ: {metadata?.batch_size || 100}件 （設定画面で変更可能）
          </p>

          {/* Status Badge */}
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium">ステータス:</span>
            <span
              className={`px-2 py-1 rounded text-xs font-semibold ${getStatusBadgeClass(metadata?.parse_status)}`}
            >
              {getStatusText(metadata?.parse_status)}
            </span>
          </div>
        </CardContent>
      </Card>

      {/* Product Name Parse (Gemini API) */}
      <Card>
        <CardHeader>
          <CardTitle>商品名解析 (AI)</CardTitle>
          <CardDescription>
            Gemini APIを使用して商品名からメーカー情報を抽出します
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {!hasGeminiApiKey && (
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
              disabled={isProductNameParsing || isParsing || !hasGeminiApiKey}
              variant={isProductNameParsing ? 'secondary' : 'default'}
            >
              {isProductNameParsing ? '解析中...' : '商品名を解析'}
            </Button>
          </div>
          <p className="text-sm text-muted-foreground">
            メールパース後に実行してください。10件ずつ処理し、間に10秒のディレイを入れます。
          </p>

          {/* Product Name Parse Progress */}
          {(isProductNameParsing || productNameProgress) && (
            <div className="space-y-4 pt-4 border-t">
              {productNameProgress && (
                <>
                  <div className="space-y-2">
                    <div className="flex justify-between text-sm">
                      <span>
                        {productNameProgress.parsed_count} /{' '}
                        {productNameProgress.total_items} 件
                      </span>
                      <span>
                        {productNameProgress.total_items > 0
                          ? Math.round(
                              (productNameProgress.parsed_count /
                                productNameProgress.total_items) *
                                100
                            )
                          : 0}
                        %
                      </span>
                    </div>
                    <Progress
                      value={
                        productNameProgress.total_items > 0
                          ? (productNameProgress.parsed_count /
                              productNameProgress.total_items) *
                            100
                          : 0
                      }
                    />
                  </div>

                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <span className="text-muted-foreground">成功:</span>
                      <div className="text-lg font-bold text-green-600">
                        {productNameProgress.success_count}
                      </div>
                    </div>
                    <div>
                      <span className="text-muted-foreground">失敗:</span>
                      <div className="text-lg font-bold text-red-600">
                        {productNameProgress.failed_count}
                      </div>
                    </div>
                  </div>

                  <div className="text-sm text-muted-foreground">
                    {productNameProgress.status_message}
                  </div>

                  {productNameProgress.is_complete &&
                    !productNameProgress.error && (
                      <div className="p-3 bg-green-50 border border-green-200 rounded text-sm text-green-800">
                        商品名解析が完了しました
                      </div>
                    )}

                  {productNameProgress.error && (
                    <div className="p-3 bg-red-50 border border-red-200 rounded text-sm text-red-800">
                      {productNameProgress.error}
                    </div>
                  )}
                </>
              )}
            </div>
          )}

          {productNameError && (
            <div className="p-3 bg-red-50 border border-red-200 rounded text-sm text-red-800">
              {productNameError}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Progress Display */}
      {(isParsing || progress) && (
        <Card>
          <CardHeader>
            <CardTitle>パース進捗</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {progress && (
              <>
                <div className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span>
                      {progress.parsed_count} / {progress.total_emails} 件
                    </span>
                    <span>{Math.round(progressPercentage)}%</span>
                  </div>
                  <Progress value={progressPercentage} />
                </div>

                <div className="grid grid-cols-2 gap-4 text-sm">
                  <div>
                    <span className="text-muted-foreground">成功:</span>
                    <div className="text-lg font-bold text-green-600">
                      {progress.success_count}
                    </div>
                  </div>
                  <div>
                    <span className="text-muted-foreground">失敗:</span>
                    <div className="text-lg font-bold text-red-600">
                      {progress.failed_count}
                    </div>
                  </div>
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
                    パースが完了しました
                  </div>
                )}
              </>
            )}
          </CardContent>
        </Card>
      )}

      {/* Parse Statistics */}
      {metadata && (
        <Card>
          <CardHeader>
            <CardTitle>パース統計</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <span className="text-muted-foreground">総パース件数:</span>
                <div className="text-2xl font-bold">
                  {metadata.total_parsed_count}
                </div>
              </div>
              {metadata.last_parse_started_at && (
                <div>
                  <span className="text-muted-foreground">開始日時:</span>
                  <div className="text-sm">
                    {formatDateTime(metadata.last_parse_started_at)}
                  </div>
                </div>
              )}
              {metadata.last_parse_completed_at && (
                <div className="col-span-2">
                  <span className="text-muted-foreground">最終完了:</span>
                  <div className="text-sm">
                    {formatDateTime(metadata.last_parse_completed_at)}
                  </div>
                </div>
              )}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Error Display */}
      {(error || progress?.error || metadata?.last_error_message) && (
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
              {error || progress?.error || metadata?.last_error_message}
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
          <p>
            メールパース機能は、データベースに保存されたメールから注文情報を自動抽出します。
          </p>
          <ol className="list-decimal list-inside space-y-1 ml-2">
            <li>店舗設定で対象のメールアドレスとパーサータイプを登録</li>
            <li>Gmail同期でメールを取得</li>
            <li>このページでパースを実行</li>
          </ol>
          <div className="mt-3 p-3 bg-yellow-50 border border-yellow-200 rounded">
            <p className="font-semibold text-yellow-800 mb-1">注意事項</p>
            <p className="text-xs text-yellow-700">
              パース処理は店舗設定で有効化された送信元アドレスのメールのみを対象とします。
              対象メールがない場合はエラーとなります。
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
