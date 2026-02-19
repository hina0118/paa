import {
  LayoutDashboard,
  ShoppingCart,
  Layers,
  Key,
  Settings,
  Database,
  ScrollText,
  Store,
  Archive,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useNavigation } from '@/contexts/use-navigation';
import type { Screen } from '@/contexts/navigation-context-value';
import type { ComponentType } from 'react';
import { useState } from 'react';

/** サイドバーナビゲーションで表示する画面（Screen のサブセット） */
type NavigationScreen = Extract<
  Screen,
  | 'dashboard'
  | 'orders'
  | 'batch'
  | 'logs'
  | 'shop-settings'
  | 'backup'
  | 'api-keys'
  | 'settings'
>;

type NavigationItem = {
  name: string;
  icon: ComponentType<{ className?: string }>;
  id: NavigationScreen;
};

/** テーブル画面（Screen のサブセット） */
type TableScreen = Extract<
  Screen,
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
>;

type TableItem = {
  name: string;
  id: TableScreen;
};

const navigationItems: NavigationItem[] = [
  { name: 'ダッシュボード', icon: LayoutDashboard, id: 'dashboard' },
  { name: '注文一覧', icon: ShoppingCart, id: 'orders' },
  { name: 'バッチ処理', icon: Layers, id: 'batch' },
  { name: 'ログ', icon: ScrollText, id: 'logs' },
  { name: '店舗設定', icon: Store, id: 'shop-settings' },
  { name: 'データのバックアップ', icon: Archive, id: 'backup' },
  { name: 'APIキー設定', icon: Key, id: 'api-keys' },
  { name: '設定', icon: Settings, id: 'settings' },
];

const tableItems: TableItem[] = [
  { name: 'メール', id: 'table-emails' },
  { name: '注文', id: 'table-orders' },
  { name: '商品アイテム', id: 'table-items' },
  { name: '画像', id: 'table-images' },
  { name: '配送情報', id: 'table-deliveries' },
  { name: 'HTML本文', id: 'table-htmls' },
  { name: '注文-メール', id: 'table-order-emails' },
  { name: '注文-HTML', id: 'table-order-htmls' },
  { name: '店舗設定', id: 'table-shop-settings' },
  { name: '商品マスタ', id: 'table-product-master' },
  { name: 'アイテム上書き', id: 'table-item-overrides' },
  { name: '注文上書き', id: 'table-order-overrides' },
  { name: '除外アイテム', id: 'table-excluded-items' },
  { name: '除外注文', id: 'table-excluded-orders' },
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
                  aria-current={isActive ? 'page' : undefined}
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
            テーブル
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
                      aria-current={isActive ? 'page' : undefined}
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
