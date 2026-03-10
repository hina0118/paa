import { useState, useCallback, useEffect, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { SurugayaSessionContext } from './surugaya-session-context-value';
import { type BatchProgress, TASK_NAMES } from './batch-progress-types';
import { toastSuccess, toastError } from '@/lib/toast';

interface SurugayaFetchProgressPayload {
  current: number;
  total: number;
  url: string;
}

function toProgress(
  payload: SurugayaFetchProgressPayload,
  isComplete: boolean
): BatchProgress {
  const { current, total, url } = payload;
  // `current` is treated as a 0-based index from the backend; convert to a processed count.
  const processed = total > 0 ? Math.min(current + 1, total) : 0;
  return {
    task_name: TASK_NAMES.SURUGAYA_MYPAGE_FETCH,
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

export function SurugayaSessionProvider({ children }: { children: ReactNode }) {
  const [isFetching, setIsFetching] = useState(false);
  const [progress, setProgress] = useState<BatchProgress | null>(null);

  useEffect(() => {
    const isActive = { current: true };
    const unlisteners: (() => void)[] = [];

    const setup = async () => {
      const unlistenProgress = await listen<SurugayaFetchProgressPayload>(
        'surugaya:fetch_progress',
        (e) => {
          if (!isActive.current) return;
          setProgress(toProgress(e.payload, false));
        }
      );

      const unlistenComplete = await listen<string | null>(
        'surugaya:fetch_complete',
        (e) => {
          if (!isActive.current) return;
          setIsFetching(false);
          const errorMsg = e.payload;
          if (errorMsg) {
            setProgress((prev) =>
              prev ? { ...prev, is_complete: true, error: errorMsg } : null
            );
            toastError('駿河屋マイページ取得に失敗しました', errorMsg);
          } else {
            setProgress((prev) =>
              prev ? { ...prev, is_complete: true } : null
            );
            toastSuccess('駿河屋マイページ取得が完了しました');
          }
        }
      );

      unlisteners.push(unlistenProgress, unlistenComplete);
    };

    setup().catch((e) =>
      console.error('Failed to set up surugaya session listeners:', e)
    );

    return () => {
      isActive.current = false;
      unlisteners.forEach((fn) => fn());
    };
  }, []);

  const openLoginWindow = useCallback(async () => {
    await invoke('open_surugaya_login_window');
  }, []);

  const startFetch = useCallback(async () => {
    setIsFetching(true);
    setProgress(null);
    try {
      await invoke('start_surugaya_mypage_fetch');
    } catch (error) {
      setIsFetching(false);
      throw error;
    }
  }, []);

  const cancelFetch = useCallback(async () => {
    try {
      await invoke('cancel_surugaya_mypage_fetch');
    } catch (error) {
      console.error('Failed to cancel surugaya mypage fetch:', error);
      throw error;
    }
  }, []);

  return (
    <SurugayaSessionContext.Provider
      value={{ isFetching, progress, openLoginWindow, startFetch, cancelFetch }}
    >
      {children}
    </SurugayaSessionContext.Provider>
  );
}
