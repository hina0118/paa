import { useState, type ReactNode } from 'react';
import type { BatchProgress } from '@/contexts/batch-progress-types';
import { BatchProgressBar } from '@/components/ui/batch-progress-bar';
import { StatusBadge, type StatusConfig } from '@/components/ui/status-badge';
import { Button } from '@/components/ui/button';
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

interface ConfirmDialogConfig {
  title: string;
  description: ReactNode;
  confirmLabel: string;
}

interface BatchSectionProps {
  /** セクションタイトル（例: "1. Gmail同期"） */
  title: string;
  /** コントロールCardのタイトル */
  controlTitle: string;
  /** コントロールCardの説明 */
  controlDescription: string;
  /** 処理実行中かどうか */
  isRunning: boolean;
  /** 進捗データ */
  progress: BatchProgress | null;
  /** 開始ハンドラ */
  onStart: () => void;
  /** キャンセルハンドラ（省略時はキャンセル不可） */
  onCancel?: () => void;
  /** キャンセルボタンのラベル（デフォルト: "キャンセル"） */
  cancelLabel?: string;
  /** キャンセルボタンのバリアント（デフォルト: "outline"） */
  cancelVariant?: 'outline' | 'destructive';
  /** 開始ボタンのラベル */
  startLabel: string;
  /** 処理中のボタンラベル */
  runningLabel: string;
  /** 開始ボタンの追加無効条件 */
  startDisabled?: boolean;
  /** 完了メッセージ */
  completeMessage: string;
  /** 進捗Cardのタイトル */
  progressTitle?: string;
  /** バッチ番号を表示するか */
  showBatchNumber?: boolean;
  /** 成功/失敗カウントを表示するか */
  showCounts?: boolean;
  /** ステータス値 */
  status?: string;
  /** ステータスバッジの設定 */
  statusConfig?: StatusConfig;
  /** 統計情報（スロット） */
  statistics?: ReactNode;
  /** コントロールCard内の追加コンテンツ */
  extraContent?: ReactNode;
  /** 開始前に確認ダイアログを表示する設定 */
  confirmDialog?: ConfirmDialogConfig;
}

export function BatchSection({
  title,
  controlTitle,
  controlDescription,
  isRunning,
  progress,
  onStart,
  onCancel,
  cancelLabel = 'キャンセル',
  cancelVariant = 'outline',
  startLabel,
  runningLabel,
  startDisabled = false,
  completeMessage,
  progressTitle = '進捗',
  showBatchNumber = false,
  showCounts = true,
  status,
  statusConfig,
  statistics,
  extraContent,
  confirmDialog,
}: BatchSectionProps) {
  const [showConfirmDialog, setShowConfirmDialog] = useState(false);

  const handleStart = () => {
    if (confirmDialog) {
      setShowConfirmDialog(true);
    } else {
      onStart();
    }
  };

  const handleConfirm = () => {
    setShowConfirmDialog(false);
    onStart();
  };

  return (
    <section className="space-y-4">
      <h2 className="text-xl font-semibold">{title}</h2>

      {/* 確認ダイアログ */}
      {confirmDialog && (
        <Dialog open={showConfirmDialog} onOpenChange={setShowConfirmDialog}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>{confirmDialog.title}</DialogTitle>
              <DialogDescription asChild>
                <div>{confirmDialog.description}</div>
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => setShowConfirmDialog(false)}
              >
                キャンセル
              </Button>
              <Button variant="destructive" onClick={handleConfirm}>
                {confirmDialog.confirmLabel}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      )}

      {/* コントロールCard */}
      <Card>
        <CardHeader>
          <CardTitle>{controlTitle}</CardTitle>
          <CardDescription>{controlDescription}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {extraContent}
          <div className="flex gap-4">
            <Button
              onClick={handleStart}
              disabled={isRunning || startDisabled}
              variant={isRunning ? 'secondary' : 'default'}
            >
              {isRunning ? runningLabel : startLabel}
            </Button>
            {isRunning && onCancel && (
              <Button onClick={onCancel} variant={cancelVariant}>
                {cancelLabel}
              </Button>
            )}
          </div>
          {statusConfig && (
            <StatusBadge status={status} config={statusConfig} />
          )}
        </CardContent>
      </Card>

      {/* 進捗Card */}
      {progress && (
        <Card>
          <CardHeader>
            <CardTitle>{progressTitle}</CardTitle>
          </CardHeader>
          <CardContent>
            <BatchProgressBar
              progress={progress}
              completeMessage={completeMessage}
              showBatchNumber={showBatchNumber}
              showCounts={showCounts}
            />
          </CardContent>
        </Card>
      )}

      {/* 統計Card */}
      {statistics}
    </section>
  );
}
