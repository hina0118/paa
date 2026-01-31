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
} from '@/components/screens/tables';
import {
  NavigationProvider,
  useNavigation,
} from '@/contexts/navigation-context';
import { SyncProvider } from '@/contexts/sync-context';
import { ParseProvider } from '@/contexts/parse-context';
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
  useEffect(() => {
    // アプリ起動時にDBを初期化してマイグレーションを実行
    const initDb = async () => {
      try {
        // フロントエンド(tauri-plugin-sql)のDB接続を初期化してマイグレーション実行
        const manager = DatabaseManager.getInstance();
        const db = await manager.getDatabase();

        // 簡単なクエリでマイグレーション実行を確実にする
        await db.select('SELECT 1');
      } catch (error) {
        console.error('Failed to initialize database:', error);
      }
    };

    initDb();

    // ウィンドウサイズ・位置変更時に設定を保存
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

    // デバウンス処理（頻繁な保存を避ける）
    let saveTimeout: ReturnType<typeof setTimeout> | undefined;
    const debouncedSave = () => {
      if (saveTimeout) {
        clearTimeout(saveTimeout);
      }
      saveTimeout = setTimeout(saveWindowSettings, 500);
    };

    // ウィンドウイベントリスナーを設定
    const setupWindowListeners = async () => {
      const window = getCurrentWindow();
      const unlistenResize = await window.onResized(debouncedSave);
      const unlistenMove = await window.onMoved(debouncedSave);

      return () => {
        unlistenResize();
        unlistenMove();
      };
    };

    let cleanup: (() => void) | undefined;
    setupWindowListeners().then((fn) => {
      cleanup = fn;
    });

    // 通知アクションイベントリスナーを設定
    let unlistenNotification: (() => void) | undefined;
    listen('notification-action', async () => {
      const window = getCurrentWindow();
      await window.show();
      await window.setFocus();
    })
      .then((unlisten) => {
        unlistenNotification = unlisten;
      })
      .catch((error) => {
        console.error('Failed to set up notification action listener:', error);
      });

    return () => {
      if (cleanup) cleanup();
      if (unlistenNotification) unlistenNotification();
      if (saveTimeout) {
        clearTimeout(saveTimeout);
      }
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
