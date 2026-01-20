import { useEffect } from "react";
import { DatabaseManager } from "@/lib/database";
import { EmailList } from "@/components/emails/email-list";
import { Sidebar } from "@/components/layout/sidebar";
import { Dashboard } from "@/components/screens/dashboard";
import { Sync } from "@/components/screens/sync";
import { Logs } from "@/components/screens/logs";
import { Settings } from "@/components/screens/settings";
import {
  EmailsTable,
  OrdersTable,
  ItemsTable,
  ImagesTable,
  DeliveriesTable,
  HtmlsTable,
  OrderEmailsTable,
  OrderHtmlsTable,
} from "@/components/screens/tables";
import { NavigationProvider, useNavigation } from "@/contexts/navigation-context";
import { SyncProvider } from "@/contexts/sync-context";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

function AppContent() {
  const { currentScreen } = useNavigation();

  const renderScreen = () => {
    switch (currentScreen) {
      case "dashboard":
        return <Dashboard />;
      case "orders":
        return <EmailList />;
      case "sync":
        return <Sync />;
      case "logs":
        return <Logs />;
      case "settings":
        return <Settings />;
      case "table-emails":
        return <EmailsTable />;
      case "table-orders":
        return <OrdersTable />;
      case "table-items":
        return <ItemsTable />;
      case "table-images":
        return <ImagesTable />;
      case "table-deliveries":
        return <DeliveriesTable />;
      case "table-htmls":
        return <HtmlsTable />;
      case "table-order-emails":
        return <OrderEmailsTable />;
      case "table-order-htmls":
        return <OrderHtmlsTable />;
      default:
        return <EmailList />;
    }
  };

  return (
    <div className="flex h-screen overflow-hidden">
      <Sidebar />
      <main className="flex-1 overflow-auto">
        {renderScreen()}
      </main>
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
        await db.select("SELECT 1");
        console.log("Database initialized with migrations");
      } catch (error) {
        console.error("Failed to initialize database:", error);
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

        await invoke("save_window_settings", {
          width: size.width,
          height: size.height,
          x: position.x,
          y: position.y,
          maximized,
        });
      } catch (error) {
        console.error("Failed to save window settings:", error);
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
    listen("notification-action", async () => {
      console.log("Notification clicked - showing window");
      const window = getCurrentWindow();
      await window.show();
      await window.setFocus();
    })
      .then((unlisten) => {
        unlistenNotification = unlisten;
      })
      .catch((error) => {
        console.error("Failed to set up notification action listener:", error);
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
        <AppContent />
      </SyncProvider>
    </NavigationProvider>
  );
}

export default App;
