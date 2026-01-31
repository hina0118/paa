import { useState, useEffect, useCallback, type ReactNode } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import {
  type SyncProgress,
  type SyncMetadata,
  SyncContext,
} from './sync-context-value';

export function SyncProvider({ children }: { children: ReactNode }) {
  const [isSyncing, setIsSyncing] = useState(false);
  const [progress, setProgress] = useState<SyncProgress | null>(null);
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

  useEffect(() => {
    const unlisten = listen<SyncProgress>('sync-progress', (event) => {
      const data = event.payload;
      setProgress(data);

      if (data.is_complete) {
        setIsSyncing(false);
        refreshStatus();
      }
    });

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
      }}
    >
      {children}
    </SyncContext.Provider>
  );
}
