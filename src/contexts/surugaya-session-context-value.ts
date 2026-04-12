import { createContext } from 'react';
import type { BatchProgress } from './batch-progress-types';

export type SurugayaSessionContextType = {
  isFetching: boolean;
  progress: BatchProgress | null;
  openLoginWindow: () => Promise<void>;
  /** 差分取得（html_content IS NULL のみ） */
  startFetch: () => Promise<void>;
  /** 全件再取得（取得済みを含む全 URL を再フェッチ） */
  startRefetchAll: () => Promise<void>;
  cancelFetch: () => Promise<void>;
};

export const SurugayaSessionContext = createContext<
  SurugayaSessionContextType | undefined
>(undefined);
