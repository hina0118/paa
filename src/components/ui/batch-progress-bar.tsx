import { Progress } from '@/components/ui/progress';
import type { BatchProgress } from '@/contexts/batch-progress-types';

interface BatchProgressBarProps {
  /** 進捗データ */
  progress: BatchProgress;
  /** 完了メッセージ（デフォルト: "処理が完了しました"） */
  completeMessage?: string;
  /** 成功/失敗カウントを表示するか */
  showCounts?: boolean;
  /** クラス名 */
  className?: string;
}

/**
 * 共通のバッチ処理進捗表示コンポーネント
 *
 * メール同期、メールパース、商品名パースで共通して使用できます。
 */
export function BatchProgressBar({
  progress,
  completeMessage = '処理が完了しました',
  showCounts = true,
  className,
}: BatchProgressBarProps) {
  return (
    <div className={`space-y-4 ${className || ''}`}>
      {/* プログレスバー */}
      <div className="space-y-2">
        <div className="flex justify-between text-sm">
          <span>
            {progress.processed_count} / {progress.total_items} 件
          </span>
          <span>{Math.round(progress.progress_percent)}%</span>
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

/**
 * シンプルなプログレスバー（カウントなし）
 */
export function SimpleBatchProgressBar({
  progress,
  className,
}: {
  progress: BatchProgress;
  className?: string;
}) {
  return (
    <div className={`space-y-2 ${className || ''}`}>
      <div className="flex justify-between text-sm">
        <span>バッチ {progress.batch_number}</span>
        <span>{progress.processed_count} 件処理済み</span>
      </div>
      <Progress value={progress.progress_percent} />
      <div className="text-sm text-muted-foreground">
        {progress.status_message}
      </div>
    </div>
  );
}
