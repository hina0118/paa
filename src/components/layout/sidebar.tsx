import {
  ShoppingCart,
  Layers,
  Key,
  Settings,
  Database,
  ScrollText,
  Store,
  Archive,
  ChevronDown,
  BookOpen,
  Truck,
} from 'lucide-react';
import { useNavigation } from '@/contexts/use-navigation';
import type { Screen } from '@/contexts/navigation-context-value';
import type { ComponentType } from 'react';
import { useState } from 'react';
import { cn } from '@/lib/utils';
import { ThemeToggle } from '@/components/ui/theme-toggle';

/** サイドバーナビゲーションで表示する画面（Screen のサブセット） */
type NavigationScreen = Extract<
  Screen,
  | 'orders'
  | 'deliveries'
  | 'batch'
  | 'logs'
  | 'shop-settings'
  | 'backup'
  | 'api-keys'
  | 'settings'
  | 'product-master'
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
  | 'table-tracking-check-logs'
>;

type TableItem = {
  name: string;
  id: TableScreen;
};

const navigationItems: NavigationItem[] = [
  { name: '商品一覧', icon: ShoppingCart, id: 'orders' },
  { name: '配送状況', icon: Truck, id: 'deliveries' },
  { name: 'バッチ処理', icon: Layers, id: 'batch' },
  { name: 'ログ', icon: ScrollText, id: 'logs' },
  { name: '店舗設定', icon: Store, id: 'shop-settings' },
  { name: 'データのバックアップ', icon: Archive, id: 'backup' },
  { name: 'APIキー設定', icon: Key, id: 'api-keys' },
  { name: '設定', icon: Settings, id: 'settings' },
  { name: '商品マスタ編集', icon: BookOpen, id: 'product-master' },
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
  { name: '配送確認ログ', id: 'table-tracking-check-logs' },
];

export function Sidebar() {
  const { currentScreen, setCurrentScreen } = useNavigation();
  const [isTableSectionOpen, setIsTableSectionOpen] = useState(false);

  return (
    <aside className="w-56 border-r bg-background flex flex-col h-full">
      <div className="h-14 flex items-center gap-2 px-4 border-b">
        <div className="h-7 w-7 rounded-lg bg-primary flex items-center justify-center shrink-0">
          <span className="text-xs font-bold text-primary-foreground">P</span>
        </div>
        <h2 className="font-semibold text-sm tracking-wide">PAA Dashboard</h2>
      </div>

      <nav className="flex-1 p-3 overflow-y-auto space-y-6">
        <div>
          <ul className="space-y-0.5">
            {navigationItems.map((item) => {
              const Icon = item.icon;
              const isActive = currentScreen === item.id;
              return (
                <li key={item.id}>
                  <button
                    className={cn(
                      'group relative flex w-full items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-all duration-150',
                      isActive
                        ? 'bg-primary/10 text-primary'
                        : 'text-muted-foreground hover:bg-muted/60 hover:text-foreground'
                    )}
                    aria-current={isActive ? 'page' : undefined}
                    data-testid={item.id}
                    onClick={() => setCurrentScreen(item.id)}
                  >
                    {isActive && (
                      <span
                        className="absolute left-0 top-1/2 -translate-y-1/2 h-5 w-1 rounded-r-full bg-primary"
                        aria-hidden="true"
                      />
                    )}
                    <Icon
                      className={cn(
                        'h-4 w-4 shrink-0',
                        isActive
                          ? 'text-primary'
                          : 'text-muted-foreground group-hover:text-foreground'
                      )}
                    />
                    {item.name}
                  </button>
                </li>
              );
            })}
          </ul>
        </div>

        <div>
          <p className="mb-1 px-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground/70">
            データ
          </p>
          <button
            id="tables-section-label"
            className="group flex w-full items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted/60 hover:text-foreground transition-all duration-150"
            data-testid="tables-section-toggle"
            aria-expanded={isTableSectionOpen}
            aria-controls="tables-section-content"
            onClick={() => setIsTableSectionOpen(!isTableSectionOpen)}
          >
            <Database className="h-4 w-4 shrink-0 text-muted-foreground group-hover:text-foreground" />
            テーブル
            <ChevronDown
              className={cn(
                'ml-auto h-3.5 w-3.5 text-muted-foreground transition-transform duration-200',
                isTableSectionOpen && 'rotate-180'
              )}
            />
          </button>

          <div
            id="tables-section-content"
            role="region"
            aria-labelledby="tables-section-label"
            className={cn(
              'overflow-hidden transition-all duration-200',
              isTableSectionOpen
                ? 'max-h-[600px] opacity-100'
                : 'max-h-0 opacity-0 invisible'
            )}
          >
            <ul className="mt-1 ml-4 space-y-0.5 border-l border-border/50 pl-2">
              {tableItems.map((item) => {
                const isActive = currentScreen === item.id;
                return (
                  <li key={item.id}>
                    <button
                      className={cn(
                        'flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-xs font-medium transition-colors',
                        isActive
                          ? 'text-primary bg-primary/5'
                          : 'text-muted-foreground hover:text-foreground hover:bg-muted/40'
                      )}
                      aria-current={isActive ? 'page' : undefined}
                      data-testid={item.id}
                      onClick={() => setCurrentScreen(item.id)}
                    >
                      {item.name}
                    </button>
                  </li>
                );
              })}
            </ul>
          </div>
        </div>
      </nav>
      <div className="p-3 border-t">
        <ThemeToggle />
      </div>
    </aside>
  );
}
