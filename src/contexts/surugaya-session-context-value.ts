import { createContext } from 'react';
import type { BatchProgress } from './batch-progress-types';

export type SurugayaSessionContextType = {
  isFetching: boolean;
  progress: BatchProgress | null;
  openLoginWindow: () => Promise<void>;
  startFetch: () => Promise<void>;
  cancelFetch: () => Promise<void>;
};

export const SurugayaSessionContext = createContext<
  SurugayaSessionContextType | undefined
>(undefined);
