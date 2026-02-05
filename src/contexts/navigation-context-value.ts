import { createContext } from 'react';

export type Screen =
  | 'dashboard'
  | 'orders'
  | 'batch'
  | 'logs'
  | 'shop-settings'
  | 'api-keys'
  | 'settings'
  | 'table-emails'
  | 'table-orders'
  | 'table-items'
  | 'table-images'
  | 'table-deliveries'
  | 'table-htmls'
  | 'table-order-emails'
  | 'table-order-htmls'
  | 'table-shop-settings'
  | 'table-product-master';

export type NavigationContextType = {
  currentScreen: Screen;
  setCurrentScreen: (screen: Screen) => void;
};

export const NavigationContext = createContext<
  NavigationContextType | undefined
>(undefined);
