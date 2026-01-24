import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  ReactNode,
} from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

export interface ParseProgress {
  batch_number: number;
  total_emails: number;
  parsed_count: number;
  success_count: number;
  failed_count: number;
  status_message: string;
  is_complete: boolean;
  error?: string;
}

export interface ParseMetadata {
  parse_status: 'idle' | 'running' | 'completed' | 'error';
  last_parse_started_at?: string;
  last_parse_completed_at?: string;
  total_parsed_count: number;
  last_error_message?: string;
  batch_size: number;
}

interface ParseContextType {
  isParsing: boolean;
  progress: ParseProgress | null;
  metadata: ParseMetadata | null;
  startParse: (batchSize?: number) => Promise<void>;
  cancelParse: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  updateBatchSize: (size: number) => Promise<void>;
}

const ParseContext = createContext<ParseContextType | undefined>(undefined);

export function ParseProvider({ children }: { children: ReactNode }) {
  const [isParsing, setIsParsing] = useState(false);
  const [progress, setProgress] = useState<ParseProgress | null>(null);
  const [metadata, setMetadata] = useState<ParseMetadata | null>(null);

  // useCallbackでメモ化して、依存配列の問題を回避
  const refreshStatus = useCallback(async () => {
    try {
      const status = await invoke<ParseMetadata>('get_parse_status');
      setMetadata(status);
      setIsParsing(status.parse_status === 'running');
    } catch (error) {
      console.error('Failed to fetch parse status:', error);
    }
  }, []);

  // Listen for parse progress events
  useEffect(() => {
    const unlisten = listen<ParseProgress>('parse-progress', (event) => {
      const data = event.payload;
      setProgress(data);

      if (data.is_complete) {
        setIsParsing(false);
        // Refresh metadata after completion
        refreshStatus();
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [refreshStatus]);

  // Load initial parse status
  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

  const startParse = async (batchSize?: number) => {
    try {
      setIsParsing(true);
      setProgress(null);
      await invoke('start_batch_parse', { batchSize });
    } catch (error) {
      setIsParsing(false);
      throw error;
    }
  };

  const cancelParse = async () => {
    try {
      await invoke('cancel_parse');
    } catch (error) {
      console.error('Failed to cancel parse:', error);
      throw error;
    }
  };

  const updateBatchSize = async (size: number) => {
    try {
      await invoke('update_parse_batch_size', { batchSize: size });
      await refreshStatus();
    } catch (error) {
      console.error('Failed to update batch size:', error);
      throw error;
    }
  };

  return (
    <ParseContext.Provider
      value={{
        isParsing,
        progress,
        metadata,
        startParse,
        cancelParse,
        refreshStatus,
        updateBatchSize,
      }}
    >
      {children}
    </ParseContext.Provider>
  );
}

export function useParse() {
  const context = useContext(ParseContext);
  if (!context) {
    throw new Error('useParse must be used within ParseProvider');
  }
  return context;
}
