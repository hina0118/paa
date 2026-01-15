import { useEffect } from "react";
import { DatabaseManager } from "@/lib/database";
import { EmailList } from "@/components/emails/email-list";
import { Sidebar } from "@/components/layout/sidebar";
import { Dashboard } from "@/components/screens/dashboard";
import { Sync } from "@/components/screens/sync";
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
