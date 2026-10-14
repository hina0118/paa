import type { ReactNode } from 'react';
import { SurugayaSessionContext } from './surugaya-session-context-value';
import { TASK_NAMES } from './batch-progress-types';
import {
  WebviewSessionProvider,
  type WebviewSessionConfig,
} from './webview-session-provider';

const SURUGAYA_SESSION_CONFIG: WebviewSessionConfig = {
  taskName: TASK_NAMES.SURUGAYA_MYPAGE_FETCH,
  label: '駿河屋マイページ取得',
  progressEvent: 'surugaya:fetch_progress',
  completeEvent: 'surugaya:fetch_complete',
  openCommand: 'open_surugaya_login_window',
  startCommand: 'start_surugaya_mypage_fetch',
  cancelCommand: 'cancel_surugaya_mypage_fetch',
};

export function SurugayaSessionProvider({ children }: { children: ReactNode }) {
  return (
    <WebviewSessionProvider
      config={SURUGAYA_SESSION_CONFIG}
      context={SurugayaSessionContext}
    >
      {children}
    </WebviewSessionProvider>
  );
}
