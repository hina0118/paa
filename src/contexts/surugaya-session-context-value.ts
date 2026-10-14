import { createContext } from 'react';
import type { WebviewSessionContextType } from './webview-session-context-value';

export type SurugayaSessionContextType = WebviewSessionContextType;

export const SurugayaSessionContext = createContext<
  SurugayaSessionContextType | undefined
>(undefined);
