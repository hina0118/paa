import { LayoutDashboard, ShoppingCart, RefreshCw, Settings } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useNavigation } from "@/contexts/navigation-context";

type NavigationItem = {
  name: string;
  icon: React.ComponentType<{ className?: string }>;
  id: "dashboard" | "orders" | "sync" | "settings";
};

const navigationItems: NavigationItem[] = [
  { name: "Dashboard", icon: LayoutDashboard, id: "dashboard" },
  { name: "Orders", icon: ShoppingCart, id: "orders" },
  { name: "Sync", icon: RefreshCw, id: "sync" },
  { name: "Settings", icon: Settings, id: "settings" },
];

export function Sidebar() {
  const { currentScreen, setCurrentScreen } = useNavigation();

  return (
    <aside className="w-64 border-r bg-muted/40 flex flex-col h-screen">
      <div className="p-6 border-b">
        <h2 className="text-2xl font-bold">PAA</h2>
      </div>
      <nav className="flex-1 p-4">
        <ul className="space-y-2">
          {navigationItems.map((item) => {
            const Icon = item.icon;
            const isActive = currentScreen === item.id;
            return (
              <li key={item.id}>
                <Button
                  variant={isActive ? "secondary" : "ghost"}
                  className="w-full justify-start"
                  onClick={() => setCurrentScreen(item.id)}
                >
                  <Icon className="mr-2 h-4 w-4" />
                  {item.name}
                </Button>
              </li>
            );
          })}
        </ul>
      </nav>
    </aside>
  );
}
