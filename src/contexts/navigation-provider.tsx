import { useState, type ReactNode } from 'react';
import { type Screen, NavigationContext } from './navigation-context-value';

export function NavigationProvider({ children }: { children: ReactNode }) {
  const [currentScreen, setCurrentScreen] = useState<Screen>('orders');
  const [pendingOcrQuery, setPendingOcrQuery] = useState<string | null>(null);
  const [exclusionFloatOpen, setExclusionFloatOpen] = useState(false);

  return (
    <NavigationContext.Provider
      value={{
        currentScreen,
        setCurrentScreen,
        pendingOcrQuery,
        setPendingOcrQuery,
        exclusionFloatOpen,
        setExclusionFloatOpen,
      }}
    >
      {children}
    </NavigationContext.Provider>
  );
}
