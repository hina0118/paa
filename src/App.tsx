import { useEffect } from 'react';
import { DatabaseManager } from '@/lib/database';
import { Orders } from '@/components/screens/orders';
import { Sidebar } from '@/components/layout/sidebar';
import { Dashboard } from '@/components/screens/dashboard';
import { Sync } from '@/components/screens/sync';
import { Parse } from '@/components/screens/parse';
import { Logs } from '@/components/screens/logs';
import { Settings } from '@/components/screens/settings';
import { ShopSettings } from '@/components/screens/shop-settings';
import {
  EmailsTable,
  OrdersTable,
  ItemsTable,
  ImagesTable,
  DeliveriesTable,
  HtmlsTable,
  OrderEmailsTable,
  OrderHtmlsTable,
  ShopSettingsTable,
  SyncMetadataTable,
  WindowSettingsTable,
  ParseMetadataTable,
  ParseSkippedTable,
} from '@/components/screens/tables';
import { NavigationProvider } from '@/contexts/navigation-provider';
import { useNavigation } from '@/contexts/use-navigation';
import { SyncProvider } from '@/contexts/sync-provider';
import { ParseProvider } from '@/contexts/parse-provider';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Toaster } from 'sonner';

function AppContent() {
  const { currentScreen } = useNavigation();

  const renderScreen = () => {
    switch (currentScreen) {
      case 'dashboard':
        return <Dashboard />;
      case 'orders':
        return <Orders />;
      case 'sync':
        return <Sync />;
      case 'parse':
        return <Parse />;
      case 'logs':
        return <Logs />;
      case 'shop-settings':
        return <ShopSettings />;
      case 'settings':
        return <Settings />;
      case 'table-emails':
        return <EmailsTable />;
      case 'table-orders':
        return <OrdersTable />;
      case 'table-items':
        return <ItemsTable />;
      case 'table-images':
        return <ImagesTable />;
      case 'table-deliveries':
        return <DeliveriesTable />;
      case 'table-htmls':
        return <HtmlsTable />;
      case 'table-order-emails':
        return <OrderEmailsTable />;
      case 'table-parse-skipped':
        return <ParseSkippedTable />;
      case 'table-order-htmls':
        return <OrderHtmlsTable />;
      case 'table-shop-settings':
        return <ShopSettingsTable />;
      case 'table-sync-metadata':
        return <SyncMetadataTable />;
      case 'table-window-settings':
        return <WindowSettingsTable />;
      case 'table-parse-metadata':
        return <ParseMetadataTable />;
      default:
        return <Orders />;
    }
  };

  return (
    <div className="flex h-screen overflow-hidden">
      <Sidebar />
      <main className="flex-1 overflow-auto">{renderScreen()}</main>
      <Toaster position="top-right" richColors />
    </div>
  );
}

function App() {
  // 関心ごとに useEffect を分割（可読性・テスト性の向上）

  // 1. DB 初期化・マイグレーション
  useEffect(() => {
    const initDb = async () => {
      try {
        const manager = DatabaseManager.getInstance();
        const db = await manager.getDatabase();
        await db.select('SELECT 1');
      } catch (error) {
        console.error('Failed to initialize database:', error);
      }
    };
    initDb();
  }, []);

  // 2. ウィンドウサイズ・位置変更時の設定保存
  useEffect(() => {
    const saveWindowSettings = async () => {
      try {
        const window = getCurrentWindow();
        const size = await window.innerSize();
        const position = await window.outerPosition();
        const maximized = await window.isMaximized();
        await invoke('save_window_settings', {
          width: size.width,
          height: size.height,
          x: position.x,
          y: position.y,
          maximized,
        });
      } catch (error) {
        console.error('Failed to save window settings:', error);
      }
    };

    let saveTimeout: ReturnType<typeof setTimeout> | undefined;
    const debouncedSave = () => {
      if (saveTimeout) clearTimeout(saveTimeout);
      saveTimeout = setTimeout(saveWindowSettings, 500);
    };

    const isActiveRef = { current: true };
    const windowCleanupRef = { current: null as (() => void) | null };
    const setupWindowListeners = async () => {
      const window = getCurrentWindow();
      const unlistenResize = await window.onResized(debouncedSave);
      const unlistenMove = await window.onMoved(debouncedSave);
      const fn = () => {
        unlistenResize();
        unlistenMove();
      };
      if (!isActiveRef.current) fn();
      else windowCleanupRef.current = fn;
    };
    setupWindowListeners().catch((error) => {
      console.error('Failed to set up window listeners:', error);
    });

    return () => {
      isActiveRef.current = false;
      windowCleanupRef.current?.();
      windowCleanupRef.current = null;
      if (saveTimeout) clearTimeout(saveTimeout);
    };
  }, []);

  // 3. 通知アクション（トレイからウィンドウを前面に）
  useEffect(() => {
    const isActiveRef = { current: true };
    const notificationCleanupRef = { current: null as (() => void) | null };
    listen('notification-action', async () => {
      const window = getCurrentWindow();
      await window.show();
      await window.setFocus();
    })
      .then((unlisten) => {
        if (!isActiveRef.current) unlisten();
        else notificationCleanupRef.current = unlisten;
      })
      .catch((error) => {
        console.error('Failed to set up notification action listener:', error);
      });

    return () => {
      isActiveRef.current = false;
      notificationCleanupRef.current?.();
      notificationCleanupRef.current = null;
    };
  }, []);

  return (
    <NavigationProvider>
      <SyncProvider>
        <ParseProvider>
          <AppContent />
        </ParseProvider>
      </SyncProvider>
    </NavigationProvider>
  );
}

export default App;
