import { createContext, useContext, useState, ReactNode } from "react";

type Screen =
  | "dashboard"
  | "orders"
  | "sync"
  | "settings"
  | "table-emails"
  | "table-orders"
  | "table-items"
  | "table-images"
  | "table-deliveries"
  | "table-htmls"
  | "table-order-emails"
  | "table-order-htmls";

type NavigationContextType = {
  currentScreen: Screen;
  setCurrentScreen: (screen: Screen) => void;
};

const NavigationContext = createContext<NavigationContextType | undefined>(
  undefined
);

export function NavigationProvider({ children }: { children: ReactNode }) {
  const [currentScreen, setCurrentScreen] = useState<Screen>("orders");

  return (
    <NavigationContext.Provider value={{ currentScreen, setCurrentScreen }}>
      {children}
    </NavigationContext.Provider>
  );
}

export function useNavigation() {
  const context = useContext(NavigationContext);
  if (context === undefined) {
    throw new Error("useNavigation must be used within a NavigationProvider");
  }
  return context;
}
