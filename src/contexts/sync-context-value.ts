import { createContext } from 'react';
import type { BatchProgress } from './batch-progress-types';

/**
 * 同期進捗（後方互換性のため残す）
 * @deprecated 新しいコードでは BatchProgress を使用してください
 */
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

/**
 * BatchProgress から SyncProgress への変換ヘルパー（後方互換性用）
 */
export function batchProgressToSyncProgress(bp: BatchProgress): SyncProgress {
  return {
    batch_number: bp.batch_number,
    batch_size: bp.batch_size,
    total_synced: bp.processed_count,
    newly_saved: bp.success_count,
    status_message: bp.status_message,
    is_complete: bp.is_complete,
    error: bp.error,
  };
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
  /** @deprecated 新しいコードでは batchProgress を使用してください */
  progress: SyncProgress | null;
  /** 共通の進捗型 */
  batchProgress: BatchProgress | null;
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
