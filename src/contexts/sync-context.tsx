import {
  createContext,
  useCallback,
  useContext,
  useState,
  useEffect,
  ReactNode,
} from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

export interface SyncProgress {
  batch_number: number;
  batch_size: number;
  total_synced: number;
  newly_saved: number;
  status_message: string;
  is_complete: boolean;
  error?: string;
}

export interface SyncMetadata {
  sync_status: 'idle' | 'syncing' | 'paused' | 'error';
  oldest_fetched_date?: string;
  total_synced_count: number;
  batch_size: number;
  last_sync_started_at?: string;
  last_sync_completed_at?: string;
  max_iterations: number;
}

interface SyncContextType {
  isSyncing: boolean;
  progress: SyncProgress | null;
  metadata: SyncMetadata | null;
  startSync: () => Promise<void>;
  cancelSync: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  updateBatchSize: (size: number) => Promise<void>;
  updateMaxIterations: (maxIterations: number) => Promise<void>;
}

const SyncContext = createContext<SyncContextType | undefined>(undefined);

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

  // Listen for sync progress events
  useEffect(() => {
    const unlisten = listen<SyncProgress>('sync-progress', (event) => {
      const data = event.payload;
      setProgress(data);

      if (data.is_complete) {
        setIsSyncing(false);
        // Refresh metadata after completion
        refreshStatus();
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [refreshStatus]);

  // Load initial sync status and reset stuck "syncing" state
  useEffect(() => {
    const initializeSync = async () => {
      try {
        // まず現在の状態を取得
        const status = await invoke<SyncMetadata>('get_sync_status');

        // "syncing"状態で固まっている場合はリセット
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

        // 最新の状態を取得して表示
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
      // Status will update via event listener
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

export function useSync() {
  const context = useContext(SyncContext);
  if (!context) {
    throw new Error('useSync must be used within SyncProvider');
  }
  return context;
}
