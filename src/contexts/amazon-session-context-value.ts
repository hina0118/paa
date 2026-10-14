import { createContext } from 'react';
import type { WebviewSessionContextType } from './webview-session-context-value';

export type AmazonSessionContextType = WebviewSessionContextType;

export const AmazonSessionContext = createContext<
  AmazonSessionContextType | undefined
>(undefined);
