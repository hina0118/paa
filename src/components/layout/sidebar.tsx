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
  Newspaper,
  Ban,
  PanelLeftClose,
  PanelLeft,
} from 'lucide-react';
import { useNavigation } from '@/contexts/use-navigation';
import type { Screen } from '@/contexts/navigation-context-value';
import type { ComponentType } from 'react';
import { useState, useEffect } from 'react';
import { cn } from '@/lib/utils';
import { ThemeToggle } from '@/components/ui/theme-toggle';
import {
  type TableDefinition,
  getTableDefinitions,
  tableNameToScreenId,
} from '@/lib/table-definitions';

function getSavedCollapsed(): boolean {
  try {
    return localStorage.getItem('sidebar-collapsed') === 'true';
  } catch {
    return false;
  }
}

const FLOAT_SCREENS = new Set<Screen>(['exclusion-patterns']);

/** サイドバーナビゲーションで表示する画面（Screen のサブセット） */
type NavigationScreen = Extract<
  Screen,
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
>;

type NavigationItem = {
  name: string;
  icon: ComponentType<{ className?: string }>;
  id: NavigationScreen;
};

const navigationItems: NavigationItem[] = [
  { name: 'ニュース', icon: Newspaper, id: 'news' },
  { name: '商品一覧', icon: ShoppingCart, id: 'orders' },
  { name: '配送状況', icon: Truck, id: 'deliveries' },
  { name: 'バッチ処理', icon: Layers, id: 'batch' },
  { name: 'ログ', icon: ScrollText, id: 'logs' },
  { name: '店舗設定', icon: Store, id: 'shop-settings' },
  { name: 'データのバックアップ', icon: Archive, id: 'backup' },
  { name: 'APIキー設定', icon: Key, id: 'api-keys' },
  { name: '設定', icon: Settings, id: 'settings' },
  { name: '商品マスタ編集', icon: BookOpen, id: 'product-master' },
  { name: '除外キーワード', icon: Ban, id: 'exclusion-patterns' },
];

export function Sidebar() {
  const { currentScreen, setCurrentScreen, setExclusionFloatOpen } =
    useNavigation();
  const [isTableSectionOpen, setIsTableSectionOpen] = useState(false);
  const [collapsed, setCollapsed] = useState(getSavedCollapsed);
  const [tableDefs, setTableDefs] = useState<TableDefinition[]>([]);

  useEffect(() => {
    getTableDefinitions().then(setTableDefs).catch(console.error);
  }, []);

  const toggleCollapsed = () => {
    setCollapsed((prev) => {
      const next = !prev;
      try {
        localStorage.setItem('sidebar-collapsed', String(next));
      } catch {
        /* storage unavailable */
      }
      return next;
    });
  };

  return (
    <aside
      className={cn(
        'border-r bg-background flex flex-col h-full transition-all duration-200 shrink-0',
        collapsed ? 'w-12' : 'w-56'
      )}
    >
      <div className="h-14 flex items-center border-b shrink-0 overflow-hidden">
        {collapsed ? (
          <div className="flex flex-col items-center w-full gap-1 py-1">
            <div className="h-7 w-7 rounded-lg bg-primary flex items-center justify-center shrink-0">
              <span className="text-xs font-bold text-primary-foreground">
                P
              </span>
            </div>
            <button
              onClick={toggleCollapsed}
              title="サイドバーを展開"
              className="text-muted-foreground hover:text-foreground transition-colors"
            >
              <PanelLeft className="h-3.5 w-3.5" />
            </button>
          </div>
        ) : (
          <div className="flex items-center gap-2 px-4 w-full">
            <div className="h-7 w-7 rounded-lg bg-primary flex items-center justify-center shrink-0">
              <span className="text-xs font-bold text-primary-foreground">
                P
              </span>
            </div>
            <h2 className="font-semibold text-sm tracking-wide flex-1 truncate">
              PAA Dashboard
            </h2>
            <button
              onClick={toggleCollapsed}
              title="サイドバーを折りたたむ"
              className="text-muted-foreground hover:text-foreground transition-colors shrink-0"
            >
              <PanelLeftClose className="h-4 w-4" />
            </button>
          </div>
        )}
      </div>

      <nav
        className={cn(
          'flex-1 overflow-y-auto',
          collapsed ? 'p-1.5' : 'p-3 space-y-6'
        )}
      >
        <div className={collapsed ? '' : undefined}>
          <ul className="space-y-0.5">
            {navigationItems.map((item) => {
              const Icon = item.icon;
              const isActive = currentScreen === item.id;
              return (
                <li key={item.id}>
                  <button
                    className={cn(
                      'group relative flex w-full items-center rounded-lg transition-all duration-150',
                      collapsed
                        ? 'justify-center p-2'
                        : 'gap-3 px-3 py-2 text-sm font-medium',
                      isActive
                        ? 'bg-primary/10 text-primary'
                        : 'text-muted-foreground hover:bg-muted/60 hover:text-foreground'
                    )}
                    aria-current={isActive ? 'page' : undefined}
                    data-testid={item.id}
                    title={collapsed ? item.name : undefined}
                    onClick={() => {
                      if (FLOAT_SCREENS.has(item.id)) {
                        setExclusionFloatOpen(true);
                      } else {
                        setCurrentScreen(item.id);
                      }
                    }}
                  >
                    {isActive && !collapsed && (
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
                    {!collapsed && item.name}
                  </button>
                </li>
              );
            })}
          </ul>
        </div>

        {!collapsed && (
          <div>
            <p className="mb-1 px-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground/70">
              データ
            </p>
          </div>
        )}

        <div className={collapsed ? 'mt-0.5' : undefined}>
          <button
            id="tables-section-label"
            className={cn(
              'group flex w-full items-center rounded-lg transition-all duration-150 text-muted-foreground hover:bg-muted/60 hover:text-foreground',
              collapsed
                ? 'justify-center p-2'
                : 'gap-3 px-3 py-2 text-sm font-medium'
            )}
            data-testid="tables-section-toggle"
            aria-expanded={!collapsed && isTableSectionOpen}
            aria-controls="tables-section-content"
            title={collapsed ? 'テーブル' : undefined}
            onClick={() => {
              if (collapsed) {
                setCollapsed(false);
                try {
                  localStorage.setItem('sidebar-collapsed', 'false');
                } catch {
                  /* storage unavailable */
                }
                setIsTableSectionOpen(true);
              } else {
                setIsTableSectionOpen(!isTableSectionOpen);
              }
            }}
          >
            <Database className="h-4 w-4 shrink-0 text-muted-foreground group-hover:text-foreground" />
            {!collapsed && (
              <>
                テーブル
                <ChevronDown
                  className={cn(
                    'ml-auto h-3.5 w-3.5 text-muted-foreground transition-transform duration-200',
                    isTableSectionOpen && 'rotate-180'
                  )}
                />
              </>
            )}
          </button>

          {!collapsed && (
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
                {tableDefs.map((def) => {
                  const screenId = tableNameToScreenId(def.name) as Screen;
                  const isActive = currentScreen === screenId;
                  return (
                    <li key={def.name}>
                      <button
                        className={cn(
                          'flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-xs font-medium transition-colors',
                          isActive
                            ? 'text-primary bg-primary/5'
                            : 'text-muted-foreground hover:text-foreground hover:bg-muted/40'
                        )}
                        aria-current={isActive ? 'page' : undefined}
                        data-testid={screenId}
                        onClick={() => setCurrentScreen(screenId)}
                      >
                        {def.label}
                      </button>
                    </li>
                  );
                })}
              </ul>
            </div>
          )}
        </div>
      </nav>
      <div
        className={cn(
          'border-t',
          collapsed ? 'p-1.5 flex justify-center' : 'p-3'
        )}
      >
        <ThemeToggle />
      </div>
    </aside>
  );
}
