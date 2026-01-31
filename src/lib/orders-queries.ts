import type { OrderItemRow } from '@/lib/types';

type LoadParams = {
  search?: string;
  shopDomain?: string;
  year?: number;
  priceMin?: number;
  priceMax?: number;
  sortBy?: 'order_date' | 'price';
  sortOrder?: 'asc' | 'desc';
};

export async function loadOrderItems(
  db: { select: <T>(sql: string, args?: unknown[]) => Promise<T[]> },
  params: LoadParams = {}
): Promise<OrderItemRow[]> {
  const {
    search = '',
    shopDomain,
    year,
    priceMin,
    priceMax,
    sortBy = 'order_date',
    sortOrder = 'desc',
  } = params;

  const conditions: string[] = ['1=1'];
  const args: unknown[] = [];

  if (search.trim()) {
    conditions.push(
      '(i.item_name LIKE ? OR i.brand LIKE ? OR o.order_number LIKE ? OR o.shop_domain LIKE ?)'
    );
    const pattern = `%${search.trim()}%`;
    args.push(pattern, pattern, pattern, pattern);
  }
  if (shopDomain) {
    conditions.push('o.shop_domain = ?');
    args.push(shopDomain);
  }
  if (year) {
    conditions.push("strftime('%Y', o.order_date) = ?");
    args.push(String(year));
  }
  if (priceMin != null) {
    conditions.push('i.price >= ?');
    args.push(priceMin);
  }
  if (priceMax != null) {
    conditions.push('i.price <= ?');
    args.push(priceMax);
  }

  const orderCol =
    sortBy === 'price' ? 'i.price' : 'COALESCE(o.order_date, o.created_at)';
  const orderDir =
    sortOrder === 'asc' || sortOrder === 'desc'
      ? sortOrder.toUpperCase()
      : 'DESC';

  const sql = `
    WITH latest_delivery AS (
      SELECT order_id, delivery_status
      FROM (
        SELECT order_id, delivery_status,
               ROW_NUMBER() OVER (PARTITION BY order_id ORDER BY updated_at DESC) AS rn
        FROM deliveries
      ) t
      WHERE rn = 1
    )
    SELECT
      i.id,
      i.order_id AS orderId,
      i.item_name AS itemName,
      i.item_name_normalized AS itemNameNormalized,
      i.price,
      i.quantity,
      i.category,
      i.brand,
      i.created_at AS createdAt,
      o.shop_domain AS shopDomain,
      o.order_number AS orderNumber,
      o.order_date AS orderDate,
      img.file_name AS fileName,
      ld.delivery_status AS deliveryStatus
    FROM items i
    JOIN orders o ON i.order_id = o.id
    LEFT JOIN latest_delivery ld ON ld.order_id = o.id
    LEFT JOIN images img ON img.item_id = i.id
    WHERE ${conditions.join(' AND ')}
    ORDER BY ${orderCol} ${orderDir}
  `;

  const rows = await db.select<OrderItemRow>(sql, args);
  return rows;
}

export async function getOrderItemFilterOptions(db: {
  select: <T>(sql: string, args?: unknown[]) => Promise<T[]>;
}): Promise<{ shopDomains: string[]; years: number[] }> {
  const [shops, years] = await Promise.all([
    db.select<{ shop_domain: string }>(
      'SELECT DISTINCT shop_domain FROM orders WHERE shop_domain IS NOT NULL ORDER BY shop_domain'
    ),
    db.select<{ yr: string }>(
      "SELECT DISTINCT strftime('%Y', order_date) AS yr FROM orders WHERE order_date IS NOT NULL AND trim(strftime('%Y', order_date)) != '' ORDER BY yr DESC"
    ),
  ]);
  return {
    shopDomains: shops.map((r) => r.shop_domain),
    years: years.map((r) => parseInt(r.yr, 10)).filter((n) => !isNaN(n)),
  };
}
