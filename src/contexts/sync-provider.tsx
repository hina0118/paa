import { useState, useEffect, useCallback, type ReactNode } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { type SyncMetadata, SyncContext } from './sync-context-value';
import {
  type BatchProgress,
  BATCH_PROGRESS_EVENT,
  TASK_NAMES,
} from './batch-progress-types';
import { toastSuccess, toastError } from '@/lib/toast';
import { notify, isAppWindowVisible } from '@/lib/utils';

export function SyncProvider({ children }: { children: ReactNode }) {
  const [isSyncing, setIsSyncing] = useState(false);
  const [progress, setProgress] = useState<BatchProgress | null>(null);
  const [metadata, setMetadata] = useState<SyncMetadata | null>(null);

  const refreshStatus = useCallback(async () => {
    try {
      const status = await invoke<SyncMetadata>('get_sync_status');
      setMetadata(status);
      setIsSyncing(status.sync_status === 'syncing');
    } catch (error) {
      console.error('Failed to fetch sync status:', error);
    }
  }, []);

  // 共通イベント（batch-progress）をリッスン
  useEffect(() => {
    const unlisten = listen<BatchProgress>(
      BATCH_PROGRESS_EVENT,
      async (event) => {
        const data = event.payload;

        // メール同期のイベントのみ処理
        if (data.task_name !== TASK_NAMES.GMAIL_SYNC) {
          return;
        }

        setProgress(data);

        if (data.is_complete) {
          setIsSyncing(false);
          refreshStatus();
          const visible = await isAppWindowVisible();
          if (visible) {
            if (data.error) {
              toastError('Gmail同期に失敗しました', data.error);
            } else {
              const desc =
                data.success_count > 0
                  ? `新たに${data.success_count}件のメールを取り込みました`
                  : '新規メッセージはありませんでした';
              toastSuccess('Gmail同期が完了しました', desc);
            }
          } else {
            if (data.error) {
              try {
                await notify('Gmail同期失敗', data.error);
              } catch (error) {
                console.error(
                  'Failed to send Gmail sync failure notification:',
                  error
                );
              }
            } else {
              const body =
                data.success_count > 0
                  ? `新たに${data.success_count}件のメールを取り込みました`
                  : '新規メッセージはありませんでした';
              try {
                await notify('Gmail同期完了', body);
              } catch (error) {
                console.error(
                  'Failed to send Gmail sync completion notification:',
                  error
                );
              }
            }
          }
        }
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [refreshStatus]);

  useEffect(() => {
    const initializeSync = async () => {
      try {
        const status = await invoke<SyncMetadata>('get_sync_status');

        if (status.sync_status === 'syncing') {
          console.warn(
            "Detected stuck 'syncing' state on app startup, resetting to 'idle'"
          );

          try {
            await invoke('reset_sync_status');
          } catch (resetError) {
            console.error('Failed to reset sync status:', resetError);
          }
        }

        await refreshStatus();
      } catch (error) {
        console.error('Failed to initialize sync state:', error);
      }
    };

    initializeSync();
  }, [refreshStatus]);

  const startSync = async () => {
    try {
      setIsSyncing(true);
      setProgress(null);
      await invoke('start_sync');
    } catch (error) {
      setIsSyncing(false);
      throw error;
    }
  };

  const cancelSync = async () => {
    try {
      await invoke('cancel_sync');
    } catch (error) {
      console.error('Failed to cancel sync:', error);
      throw error;
    }
  };

  const updateBatchSize = async (size: number) => {
    try {
      await invoke('update_batch_size', { batchSize: size });
      await refreshStatus();
    } catch (error) {
      console.error('Failed to update batch size:', error);
      throw error;
    }
  };

  const updateMaxIterations = async (maxIterations: number) => {
    try {
      await invoke('update_max_iterations', { maxIterations });
      await refreshStatus();
    } catch (error) {
      console.error('Failed to update max iterations:', error);
      throw error;
    }
  };

  const updateMaxResultsPerPage = async (maxResultsPerPage: number) => {
    try {
      await invoke('update_max_results_per_page', { maxResultsPerPage });
      await refreshStatus();
    } catch (error) {
      console.error('Failed to update max results per page:', error);
      throw error;
    }
  };

  const updateTimeoutMinutes = async (timeoutMinutes: number) => {
    try {
      await invoke('update_timeout_minutes', { timeoutMinutes });
      await refreshStatus();
    } catch (error) {
      console.error('Failed to update timeout minutes:', error);
      throw error;
    }
  };

  return (
    <SyncContext.Provider
      value={{
        isSyncing,
        progress,
        metadata,
        startSync,
        cancelSync,
        refreshStatus,
        updateBatchSize,
        updateMaxIterations,
        updateMaxResultsPerPage,
        updateTimeoutMinutes,
      }}
    >
      {children}
    </SyncContext.Provider>
  );
}
