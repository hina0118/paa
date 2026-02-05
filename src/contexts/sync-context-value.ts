import { createContext } from 'react';
import type { BatchProgress } from './batch-progress-types';

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
  /** 共通の進捗型 */
  progress: BatchProgress | null;
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
