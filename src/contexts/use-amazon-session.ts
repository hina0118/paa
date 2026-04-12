import { useContext } from 'react';
import { AmazonSessionContext } from './amazon-session-context-value';

export function useAmazonSession() {
  const ctx = useContext(AmazonSessionContext);
  if (!ctx) {
    throw new Error(
      'useAmazonSession must be used within an AmazonSessionProvider'
    );
  }
  return ctx;
}
