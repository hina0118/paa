import { Progress } from '@/components/ui/progress';
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
      {/* プログレスバー */}
      <div className="space-y-2">
        <div className="flex justify-between text-sm">
          {showBatchNumber ? (
            <>
              <span>バッチ {progress.batch_number}</span>
              <span>{progress.processed_count} 件処理済み</span>
            </>
          ) : (
            <>
              <span>
                {progress.processed_count} / {progress.total_items} 件
              </span>
              <span>{Math.round(progress.progress_percent)}%</span>
            </>
          )}
        </div>
        <Progress value={progress.progress_percent} />
      </div>

      {/* 成功/失敗カウント */}
      {showCounts && (
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
      )}

      {/* ステータスメッセージ */}
      <div className="text-sm text-muted-foreground">
        {progress.status_message}
      </div>

      {/* 完了メッセージ */}
      {progress.is_complete && !progress.error && (
        <div
          className="p-3 bg-green-50 border border-green-200 rounded text-sm text-green-800"
          data-testid="success-message"
          role="status"
        >
          {completeMessage}
        </div>
      )}

      {/* エラーメッセージ */}
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
