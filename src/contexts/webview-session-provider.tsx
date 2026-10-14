import {
  useState,
  useCallback,
  useEffect,
  type Context,
  type ReactNode,
} from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { WebviewSessionContextType } from './webview-session-context-value';
import type { BatchProgress, TaskName } from './batch-progress-types';
import { toastSuccess, toastError } from '@/lib/toast';

/** 店舗ごとの WebView セッション設定（Rust 側 `WebviewSession` と対応） */
export interface WebviewSessionConfig {
  taskName: TaskName;
  /** トースト・ログ表示用の名称（例: `Amazon注文詳細取得`） */
  label: string;
  progressEvent: string;
  completeEvent: string;
  openCommand: string;
  startCommand: string;
  cancelCommand: string;
}

interface FetchProgressPayload {
  current: number;
  total: number;
  url: string;
}

interface FetchCompletePayload {
  cancelled: boolean;
  error: string | null;
}

function toProgress(
  taskName: TaskName,
  payload: FetchProgressPayload,
  isComplete: boolean
): BatchProgress {
  const { current, total, url } = payload;
  // `current` is treated as a 0-based index from the backend; convert to a processed count.
  const processed = total > 0 ? Math.min(current + 1, total) : 0;
  return {
    task_name: taskName,
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

export function WebviewSessionProvider({
  config,
  context,
  children,
}: {
  config: WebviewSessionConfig;
  context: Context<WebviewSessionContextType | undefined>;
  children: ReactNode;
}) {
  const [isFetching, setIsFetching] = useState(false);
  const [progress, setProgress] = useState<BatchProgress | null>(null);

  useEffect(() => {
    const isActive = { current: true };
    const unlisteners: (() => void)[] = [];

    const setup = async () => {
      const unlistenProgress = await listen<FetchProgressPayload>(
        config.progressEvent,
        (e) => {
          if (!isActive.current) return;
          setProgress(toProgress(config.taskName, e.payload, false));
        }
      );

      const unlistenComplete = await listen<FetchCompletePayload>(
        config.completeEvent,
        (e) => {
          if (!isActive.current) return;
          setIsFetching(false);
          const { cancelled, error } = e.payload;
          if (error) {
            setProgress((prev) =>
              prev ? { ...prev, is_complete: true, error } : null
            );
            toastError(`${config.label}に失敗しました`, error);
          } else {
            setProgress((prev) =>
              prev ? { ...prev, is_complete: true } : null
            );
            // キャンセル時はトースト非表示
            if (!cancelled) {
              toastSuccess(`${config.label}が完了しました`);
            }
          }
        }
      );

      unlisteners.push(unlistenProgress, unlistenComplete);
    };

    setup().catch((e) =>
      console.error(`Failed to set up ${config.label} listeners:`, e)
    );

    return () => {
      isActive.current = false;
      unlisteners.forEach((fn) => fn());
    };
  }, [config]);

  const openLoginWindow = useCallback(async () => {
    await invoke(config.openCommand);
  }, [config]);

  const start = useCallback(
    async (forceRefetch: boolean) => {
      setIsFetching(true);
      setProgress(null);
      try {
        await invoke(config.startCommand, { forceRefetch });
      } catch (error) {
        setIsFetching(false);
        throw error;
      }
    },
    [config]
  );

  const startFetch = useCallback(() => start(false), [start]);
  const startRefetchAll = useCallback(() => start(true), [start]);

  const cancelFetch = useCallback(async () => {
    try {
      await invoke(config.cancelCommand);
    } catch (error) {
      console.error(`Failed to cancel ${config.label}:`, error);
      throw error;
    }
  }, [config]);

  const Provider = context.Provider;
  return (
    <Provider
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
    </Provider>
  );
}
