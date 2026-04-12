import { useState, useCallback, useEffect, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { AmazonSessionContext } from './amazon-session-context-value';
import { type BatchProgress, TASK_NAMES } from './batch-progress-types';
import { toastSuccess, toastError } from '@/lib/toast';

interface AmazonFetchProgressPayload {
  current: number;
  total: number;
  url: string;
}

interface AmazonFetchCompletePayload {
  cancelled: boolean;
  error: string | null;
}

function toProgress(
  payload: AmazonFetchProgressPayload,
  isComplete: boolean
): BatchProgress {
  const { current, total, url } = payload;
  const processed = total > 0 ? Math.min(current + 1, total) : 0;
  return {
    task_name: TASK_NAMES.AMAZON_ORDER_FETCH,
    batch_number: processed,
    batch_size: 1,
    total_items: total,
    processed_count: processed,
    success_count: processed,
    failed_count: 0,
    progress_percent: total > 0 ? (processed / total) * 100 : 0,
    status_message: url,
    is_complete: isComplete,
  };
}

export function AmazonSessionProvider({ children }: { children: ReactNode }) {
  const [isFetching, setIsFetching] = useState(false);
  const [progress, setProgress] = useState<BatchProgress | null>(null);

  useEffect(() => {
    const isActive = { current: true };
    const unlisteners: (() => void)[] = [];

    const setup = async () => {
      const unlistenProgress = await listen<AmazonFetchProgressPayload>(
        'amazon:fetch_progress',
        (e) => {
          if (!isActive.current) return;
          setProgress(toProgress(e.payload, false));
        }
      );

      const unlistenComplete = await listen<AmazonFetchCompletePayload>(
        'amazon:fetch_complete',
        (e) => {
          if (!isActive.current) return;
          setIsFetching(false);
          const { cancelled, error } = e.payload;
          if (error) {
            setProgress((prev) =>
              prev ? { ...prev, is_complete: true, error } : null
            );
            toastError('Amazon注文詳細取得に失敗しました', error);
          } else if (cancelled) {
            setProgress((prev) =>
              prev ? { ...prev, is_complete: true } : null
            );
            // キャンセル時はトースト非表示
          } else {
            setProgress((prev) =>
              prev ? { ...prev, is_complete: true } : null
            );
            toastSuccess('Amazon注文詳細取得が完了しました');
          }
        }
      );

      unlisteners.push(unlistenProgress, unlistenComplete);
    };

    setup().catch((e) =>
      console.error('Failed to set up amazon session listeners:', e)
    );

    return () => {
      isActive.current = false;
      unlisteners.forEach((fn) => fn());
    };
  }, []);

  const openLoginWindow = useCallback(async () => {
    await invoke('open_amazon_login_window');
  }, []);

  const startFetch = useCallback(async () => {
    setIsFetching(true);
    setProgress(null);
    try {
      await invoke('start_amazon_order_fetch', { forceRefetch: false });
    } catch (error) {
      setIsFetching(false);
      throw error;
    }
  }, []);

  const startRefetchAll = useCallback(async () => {
    setIsFetching(true);
    setProgress(null);
    try {
      await invoke('start_amazon_order_fetch', { forceRefetch: true });
    } catch (error) {
      setIsFetching(false);
      throw error;
    }
  }, []);

  const cancelFetch = useCallback(async () => {
    try {
      await invoke('cancel_amazon_order_fetch');
    } catch (error) {
      console.error('Failed to cancel amazon order fetch:', error);
      throw error;
    }
  }, []);

  return (
    <AmazonSessionContext.Provider
      value={{
        isFetching,
        progress,
        openLoginWindow,
        startFetch,
        startRefetchAll,
        cancelFetch,
      }}
    >
      {children}
    </AmazonSessionContext.Provider>
  );
}
