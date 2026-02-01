import { useContext } from 'react';
import { ParseContext } from './parse-context-value';

export function useParse() {
  const context = useContext(ParseContext);
  if (!context) {
    throw new Error('useParse must be used within ParseProvider');
  }
  return context;
}
