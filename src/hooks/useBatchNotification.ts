import { useCallback } from 'react';
import type { BatchProgress } from '@/contexts/batch-progress-types';
import { toastSuccess, toastError } from '@/lib/toast';
import { notify, isAppWindowVisible } from '@/lib/utils';

/**
 * ウィンドウ可視性に基づく通知の分岐パターンを共通化するフック
 *
 * 可視時: toastSuccess / toastError (丁寧なタイトル)
 * 非可視時: OS通知 (notify) (短縮タイトル) with try/catch
 *
 * タイトル生成パターン:
 * - toast成功: `${taskLabel}が完了しました`
 * - toast失敗: `${taskLabel}に失敗しました`
 * - OS通知成功: `${taskLabel}完了`
 * - OS通知失敗: `${taskLabel}失敗`
 */
export function useBatchNotification(
  taskLabel: string,
  buildSuccessMessage: (progress: BatchProgress) => string,
  notificationContext: string
): (progress: BatchProgress) => Promise<void> {
  return useCallback(
    async (data: BatchProgress) => {
      const visible = await isAppWindowVisible();
      if (visible) {
        if (data.error) {
          toastError(`${taskLabel}に失敗しました`, data.error);
        } else {
          toastSuccess(`${taskLabel}が完了しました`, buildSuccessMessage(data));
        }
      } else {
        if (data.error) {
          try {
            await notify(`${taskLabel}失敗`, data.error);
          } catch (error) {
            console.error(
              `Failed to send ${notificationContext} failure notification:`,
              error
            );
          }
        } else {
          try {
            await notify(`${taskLabel}完了`, buildSuccessMessage(data));
          } catch (error) {
            console.error(
              `Failed to send ${notificationContext} completion notification:`,
              error
            );
          }
        }
      }
    },
    [taskLabel, buildSuccessMessage, notificationContext]
  );
}
