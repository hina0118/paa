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
  return (
    <NavigationProvider>
      <AppContent />
    </NavigationProvider>
  );
}

export default App;
