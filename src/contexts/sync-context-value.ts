import { createContext } from 'react';

export interface SyncProgress {
  batch_number: number;
  batch_size: number;
  total_synced: number;
  /** INSERT または ON CONFLICT DO UPDATE で保存された件数（新規のみではない） */
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

export interface SyncContextType {
  isSyncing: boolean;
  progress: SyncProgress | null;
  metadata: SyncMetadata | null;
  startSync: () => Promise<void>;
  cancelSync: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  updateBatchSize: (size: number) => Promise<void>;
  updateMaxIterations: (maxIterations: number) => Promise<void>;
}

export const SyncContext = createContext<SyncContextType | undefined>(
  undefined
);
