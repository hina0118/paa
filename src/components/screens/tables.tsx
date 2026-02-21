import { TableViewer } from '@/components/tables/table-viewer';

export function EmailsTable() {
  return <TableViewer tableName="emails" title="メールテーブル" />;
}

export function OrdersTable() {
  return <TableViewer tableName="orders" title="注文テーブル" />;
}

export function ItemsTable() {
  return <TableViewer tableName="items" title="商品アイテムテーブル" />;
}

export function ImagesTable() {
  return <TableViewer tableName="images" title="画像テーブル" />;
}

export function DeliveriesTable() {
  return <TableViewer tableName="deliveries" title="配送情報テーブル" />;
}

export function HtmlsTable() {
  return <TableViewer tableName="htmls" title="HTML本文テーブル" />;
}

export function OrderEmailsTable() {
  return <TableViewer tableName="order_emails" title="注文-メールテーブル" />;
}

export function OrderHtmlsTable() {
  return <TableViewer tableName="order_htmls" title="注文-HTMLテーブル" />;
}

export function ShopSettingsTable() {
  return <TableViewer tableName="shop_settings" title="店舗設定テーブル" />;
}

export function ProductMasterTable() {
  return <TableViewer tableName="product_master" title="商品マスタテーブル" />;
}

export function ItemOverridesTable() {
  return (
    <TableViewer tableName="item_overrides" title="アイテム上書きテーブル" />
  );
}

export function OrderOverridesTable() {
  return <TableViewer tableName="order_overrides" title="注文上書きテーブル" />;
}

export function ExcludedItemsTable() {
  return (
    <TableViewer tableName="excluded_items" title="除外アイテムテーブル" />
  );
}

export function ExcludedOrdersTable() {
  return <TableViewer tableName="excluded_orders" title="除外注文テーブル" />;
}
