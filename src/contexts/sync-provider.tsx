import { useState, useEffect, useCallback, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { type SyncMetadata, SyncContext } from './sync-context-value';
import { type BatchProgress, TASK_NAMES } from './batch-progress-types';
import { useBatchNotification } from '@/hooks/useBatchNotification';
import { useBatchProgressEvent } from '@/hooks/useBatchProgressEvent';

const buildGmailSuccessMessage = (data: BatchProgress) =>
  data.success_count > 0
    ? `新たに${data.success_count}件のメールを取り込みました`
    : '新規メッセージはありませんでした';

export function SyncProvider({ children }: { children: ReactNode }) {
  const [isSyncing, setIsSyncing] = useState(false);
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

  const notifyGmailSync = useBatchNotification(
    'Gmail同期',
    buildGmailSuccessMessage,
    'Gmail sync'
  );

  const handleGmailComplete = useCallback(
    async (data: BatchProgress) => {
      setIsSyncing(false);
      refreshStatus();
      await notifyGmailSync(data);
    },
    [refreshStatus, notifyGmailSync]
  );

  const { progress, setProgress } = useBatchProgressEvent(
    TASK_NAMES.GMAIL_SYNC,
    handleGmailComplete
  );

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

  const startIncrementalSync = async () => {
    try {
      setIsSyncing(true);
      setProgress(null);
      await invoke('start_incremental_sync');
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
        startIncrementalSync,
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
