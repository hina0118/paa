import {
  LayoutDashboard,
  ShoppingCart,
  RefreshCw,
  Settings,
  Database,
  ScrollText,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useNavigation } from '@/contexts/navigation-context';
import { useState } from 'react';

type NavigationItem = {
  name: string;
  icon: React.ComponentType<{ className?: string }>;
  id: 'dashboard' | 'orders' | 'sync' | 'settings' | 'logs';
};

type TableItem = {
  name: string;
  id:
    | 'table-emails'
    | 'table-orders'
    | 'table-items'
    | 'table-images'
    | 'table-deliveries'
    | 'table-htmls'
    | 'table-order-emails'
    | 'table-order-htmls';
};

const navigationItems: NavigationItem[] = [
  { name: 'Dashboard', icon: LayoutDashboard, id: 'dashboard' },
  { name: 'Orders', icon: ShoppingCart, id: 'orders' },
  { name: 'Sync', icon: RefreshCw, id: 'sync' },
  { name: 'Logs', icon: ScrollText, id: 'logs' },
  { name: 'Settings', icon: Settings, id: 'settings' },
];

const tableItems: TableItem[] = [
  { name: 'Emails', id: 'table-emails' },
  { name: 'Orders', id: 'table-orders' },
  { name: 'Items', id: 'table-items' },
  { name: 'Images', id: 'table-images' },
  { name: 'Deliveries', id: 'table-deliveries' },
  { name: 'HTMLs', id: 'table-htmls' },
  { name: 'Order-Emails', id: 'table-order-emails' },
  { name: 'Order-HTMLs', id: 'table-order-htmls' },
];

export function Sidebar() {
  const { currentScreen, setCurrentScreen } = useNavigation();
  const [isTableSectionOpen, setIsTableSectionOpen] = useState(false);

  return (
    <aside className="w-64 border-r bg-muted/40 flex flex-col h-screen">
      <div className="p-6 border-b">
        <h2 className="text-2xl font-bold">PAA</h2>
      </div>
      <nav className="flex-1 p-4 overflow-y-auto">
        <ul className="space-y-2">
          {navigationItems.map((item) => {
            const Icon = item.icon;
            const isActive = currentScreen === item.id;
            return (
              <li key={item.id}>
                <Button
                  variant={isActive ? 'secondary' : 'ghost'}
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

        <div className="mt-6">
          <Button
            variant="ghost"
            className="w-full justify-start"
            onClick={() => setIsTableSectionOpen(!isTableSectionOpen)}
          >
            <Database className="mr-2 h-4 w-4" />
            Tables
            <span className="ml-auto">{isTableSectionOpen ? '▼' : '▶'}</span>
          </Button>

          {isTableSectionOpen && (
            <ul className="mt-2 ml-4 space-y-1">
              {tableItems.map((item) => {
                const isActive = currentScreen === item.id;
                return (
                  <li key={item.id}>
                    <Button
                      variant={isActive ? 'secondary' : 'ghost'}
                      className="w-full justify-start text-sm"
                      onClick={() => setCurrentScreen(item.id)}
                    >
                      {item.name}
                    </Button>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </nav>
    </aside>
  );
}
