import { TableViewer } from '@/components/tables/table-viewer';

export function EmailsTable() {
  return <TableViewer tableName="emails" title="Emails テーブル" />;
}

export function OrdersTable() {
  return <TableViewer tableName="orders" title="Orders テーブル" />;
}

export function ItemsTable() {
  return <TableViewer tableName="items" title="Items テーブル" />;
}

export function ImagesTable() {
  return <TableViewer tableName="images" title="Images テーブル" />;
}

export function DeliveriesTable() {
  return <TableViewer tableName="deliveries" title="Deliveries テーブル" />;
}

export function HtmlsTable() {
  return <TableViewer tableName="htmls" title="HTMLs テーブル" />;
}

export function OrderEmailsTable() {
  return <TableViewer tableName="order_emails" title="Order-Emails テーブル" />;
}

export function ParseSkippedTable() {
  return (
    <TableViewer tableName="parse_skipped" title="Parse Skipped テーブル" />
  );
}

export function OrderHtmlsTable() {
  return <TableViewer tableName="order_htmls" title="Order-HTMLs テーブル" />;
}

export function ShopSettingsTable() {
  return (
    <TableViewer tableName="shop_settings" title="Shop Settings テーブル" />
  );
}

export function SyncMetadataTable() {
  return (
    <TableViewer tableName="sync_metadata" title="Sync Metadata テーブル" />
  );
}

export function WindowSettingsTable() {
  return (
    <TableViewer tableName="window_settings" title="Window Settings テーブル" />
  );
}

export function ParseMetadataTable() {
  return (
    <TableViewer tableName="parse_metadata" title="Parse Metadata テーブル" />
  );
}
