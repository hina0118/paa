import { useContext } from 'react';
import { SurugayaSessionContext } from './surugaya-session-context-value';

export function useSurugayaSession() {
  const ctx = useContext(SurugayaSessionContext);
  if (!ctx) {
    throw new Error(
      'useSurugayaSession must be used within a SurugayaSessionProvider'
    );
  }
  return ctx;
}
