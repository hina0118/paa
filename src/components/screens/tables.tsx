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

export function OrderHtmlsTable() {
  return <TableViewer tableName="order_htmls" title="Order-HTMLs テーブル" />;
}
