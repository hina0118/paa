import { useState, useEffect, type Dispatch, type SetStateAction } from 'react';
import { listen } from '@tauri-apps/api/event';
import {
  type BatchProgress,
  type TaskName,
  BATCH_PROGRESS_EVENT,
} from '@/contexts/batch-progress-types';

/**
 * Tauriのbatch-progressイベントをリッスンし、進捗stateを管理するフック
 *
 * - taskNameでフィルタリングし、マッチするイベントのみを処理
 * - is_complete === true のとき onComplete コールバックを呼び出す
 * - setProgress を返し、呼び出し側が startXxx 時に null リセット可能
 */
export function useBatchProgressEvent(
  taskName: TaskName,
  onComplete: (data: BatchProgress) => Promise<void>
): {
  progress: BatchProgress | null;
  setProgress: Dispatch<SetStateAction<BatchProgress | null>>;
} {
  const [progress, setProgress] = useState<BatchProgress | null>(null);

  useEffect(() => {
    const unlisten = listen<BatchProgress>(
      BATCH_PROGRESS_EVENT,
      async (event) => {
        const data = event.payload;
        if (data.task_name !== taskName) return;
        setProgress(data);
        if (data.is_complete) {
          await onComplete(data);
        }
      }
    );

    return () => {
      unlisten
        .then((fn) => fn())
        .catch((e) => {
          console.error('Failed to unlisten batch-progress event:', e);
        });
    };
  }, [taskName, onComplete]);

  return { progress, setProgress };
}
