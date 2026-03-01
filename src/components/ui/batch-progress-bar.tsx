import { CircularProgress } from '@/components/ui/circular-progress';
import type { BatchProgress } from '@/contexts/batch-progress-types';

interface BatchProgressBarProps {
  /** 進捗データ */
  progress: BatchProgress;
  /** 完了メッセージ（デフォルト: "処理が完了しました"） */
  completeMessage?: string;
  /** 成功/失敗カウントを表示するか */
  showCounts?: boolean;
  /** バッチ番号を表示するか（trueの場合、件数表示の代わりにバッチ番号を表示） */
  showBatchNumber?: boolean;
  /** クラス名 */
  className?: string;
}

/**
 * バッチ処理進捗表示コンポーネント
 *
 * メール同期、メールパース、商品名パースで共通して使用します。
 * - showBatchNumber: バッチ番号表示（Gmail同期向け）
 * - showCounts: 成功/失敗カウント表示（パース処理向け）
 */
export function BatchProgressBar({
  progress,
  completeMessage = '処理が完了しました',
  showCounts = true,
  showBatchNumber = false,
  className,
}: BatchProgressBarProps) {
  return (
    <div className={`space-y-4 ${className || ''}`}>
      <div className="flex items-center gap-6">
        <CircularProgress value={progress.progress_percent} aria-label="バッチ処理進捗" />

        <div className="flex-1 space-y-1">
          {showBatchNumber ? (
            <>
              <div className="text-sm font-medium">
                バッチ {progress.batch_number}
              </div>
              <div className="text-sm text-muted-foreground">
                {progress.processed_count} 件処理済み
              </div>
            </>
          ) : (
            <div className="text-sm font-medium">
              {progress.processed_count} / {progress.total_items} 件
            </div>
          )}

          {showCounts && (
            <div className="flex gap-4 text-sm">
              <span>
                <span className="text-muted-foreground">成功: </span>
                <span className="font-semibold text-green-600">
                  {progress.success_count}
                </span>
              </span>
              <span>
                <span className="text-muted-foreground">失敗: </span>
                <span className="font-semibold text-red-600">
                  {progress.failed_count}
                </span>
              </span>
            </div>
          )}

          <div className="text-sm text-muted-foreground">
            {progress.status_message}
          </div>
        </div>
      </div>

      {progress.is_complete && !progress.error && (
        <div
          className="p-3 bg-green-50 border border-green-200 rounded text-sm text-green-800"
          data-testid="success-message"
          role="status"
        >
          {completeMessage}
        </div>
      )}

      {progress.error && (
        <div
          className="p-3 bg-red-50 border border-red-200 rounded text-sm text-red-800"
          data-testid="error-message"
          role="alert"
        >
          {progress.error}
        </div>
      )}
    </div>
  );
}
