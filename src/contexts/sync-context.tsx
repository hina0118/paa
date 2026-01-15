import { createContext, useContext, useState, useEffect, ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

export interface SyncProgress {
  batchNumber: number;
  batchSize: number;
  totalSynced: number;
  newlySaved: number;
  statusMessage: string;
  isComplete: boolean;
  error?: string;
}

export interface SyncMetadata {
  syncStatus: "idle" | "syncing" | "paused" | "error";
  oldestFetchedDate?: string;
  totalSyncedCount: number;
  batchSize: number;
  lastSyncStartedAt?: string;
  lastSyncCompletedAt?: string;
}

interface SyncContextType {
  isSyncing: boolean;
  progress: SyncProgress | null;
  metadata: SyncMetadata | null;
  startSync: () => Promise<void>;
  cancelSync: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  updateBatchSize: (size: number) => Promise<void>;
}

const SyncContext = createContext<SyncContextType | undefined>(undefined);

export function SyncProvider({ children }: { children: ReactNode }) {
  const [isSyncing, setIsSyncing] = useState(false);
  const [progress, setProgress] = useState<SyncProgress | null>(null);
  const [metadata, setMetadata] = useState<SyncMetadata | null>(null);

  // Listen for sync progress events
  useEffect(() => {
    const unlisten = listen<SyncProgress>("sync-progress", (event) => {
      const data = event.payload;
      setProgress(data);

      if (data.isComplete) {
        setIsSyncing(false);
        // Refresh metadata after completion
        refreshStatus();
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Load initial sync status
  useEffect(() => {
    refreshStatus();
  }, []);

  const refreshStatus = async () => {
    try {
      const status = await invoke<SyncMetadata>("get_sync_status");
      setMetadata(status);
      setIsSyncing(status.syncStatus === "syncing");
    } catch (error) {
      console.error("Failed to fetch sync status:", error);
    }
  };

  const startSync = async () => {
    try {
      setIsSyncing(true);
      setProgress(null);
      await invoke("start_sync");
    } catch (error) {
      setIsSyncing(false);
      throw error;
    }
  };

  const cancelSync = async () => {
    try {
      await invoke("cancel_sync");
      // Status will update via event listener
    } catch (error) {
      console.error("Failed to cancel sync:", error);
      throw error;
    }
  };

  const updateBatchSize = async (size: number) => {
    try {
      await invoke("update_batch_size", { batchSize: size });
      await refreshStatus();
    } catch (error) {
      console.error("Failed to update batch size:", error);
      throw error;
    }
  };

  return (
    <SyncContext.Provider
      value={{ isSyncing, progress, metadata, startSync, cancelSync, refreshStatus, updateBatchSize }}
    >
      {children}
    </SyncContext.Provider>
  );
}

export function useSync() {
  const context = useContext(SyncContext);
  if (!context) {
    throw new Error("useSync must be used within SyncProvider");
  }
  return context;
}
