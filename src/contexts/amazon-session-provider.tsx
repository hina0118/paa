import type { ReactNode } from 'react';
import { AmazonSessionContext } from './amazon-session-context-value';
import { TASK_NAMES } from './batch-progress-types';
import {
  WebviewSessionProvider,
  type WebviewSessionConfig,
} from './webview-session-provider';

const AMAZON_SESSION_CONFIG: WebviewSessionConfig = {
  taskName: TASK_NAMES.AMAZON_ORDER_FETCH,
  label: 'Amazon注文詳細取得',
  progressEvent: 'amazon:fetch_progress',
  completeEvent: 'amazon:fetch_complete',
  openCommand: 'open_amazon_login_window',
  startCommand: 'start_amazon_order_fetch',
  cancelCommand: 'cancel_amazon_order_fetch',
};

export function AmazonSessionProvider({ children }: { children: ReactNode }) {
  return (
    <WebviewSessionProvider
      config={AMAZON_SESSION_CONFIG}
      context={AmazonSessionContext}
    >
      {children}
    </WebviewSessionProvider>
  );
}
