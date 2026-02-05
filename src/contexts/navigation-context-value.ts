import { createContext } from 'react';

export type Screen =
  | 'dashboard'
  | 'orders'
  | 'sync'
  | 'parse'
  | 'logs'
  | 'shop-settings'
  | 'settings'
  | 'table-emails'
  | 'table-orders'
  | 'table-items'
  | 'table-images'
  | 'table-deliveries'
  | 'table-htmls'
  | 'table-order-emails'
  | 'table-order-htmls'
  | 'table-parse-skipped'
  | 'table-shop-settings'
  | 'table-window-settings'
  | 'table-product-master';

export type NavigationContextType = {
  currentScreen: Screen;
  setCurrentScreen: (screen: Screen) => void;
};

export const NavigationContext = createContext<
  NavigationContextType | undefined
>(undefined);
