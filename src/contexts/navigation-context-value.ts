import { createContext } from 'react';

export type Screen =
  | 'news'
  | 'orders'
  | 'deliveries'
  | 'batch'
  | 'logs'
  | 'shop-settings'
  | 'backup'
  | 'api-keys'
  | 'settings'
  | 'product-master'
  | 'exclusion-patterns'
  | `table-${string}`;

export type NavigationContextType = {
  currentScreen: Screen;
  setCurrentScreen: (screen: Screen) => void;
  pendingOcrQuery: string | null;
  setPendingOcrQuery: (query: string | null) => void;
  exclusionFloatOpen: boolean;
  setExclusionFloatOpen: (open: boolean) => void;
};

export const NavigationContext = createContext<
  NavigationContextType | undefined
>(undefined);
