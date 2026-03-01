import { createContext } from 'react';
import type { BatchProgress } from './batch-progress-types';

export type DeliveryCheckContextType = {
  isChecking: boolean;
  progress: BatchProgress | null;
  startDeliveryCheck: () => Promise<void>;
  cancelDeliveryCheck: () => Promise<void>;
};

export const DeliveryCheckContext = createContext<
  DeliveryCheckContextType | undefined
>(undefined);
