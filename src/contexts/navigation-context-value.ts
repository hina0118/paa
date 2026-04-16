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
  | 'table-emails'
  | 'table-orders'
  | 'table-items'
  | 'table-images'
  | 'table-deliveries'
  | 'table-htmls'
  | 'table-order-emails'
  | 'table-order-htmls'
  | 'table-shop-settings'
  | 'table-product-master'
  | 'table-item-overrides'
  | 'table-order-overrides'
  | 'table-excluded-items'
  | 'table-excluded-orders'
  | 'table-tracking-check-logs'
  | 'table-news-clips'
  | 'table-item-exclusion-patterns';

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
