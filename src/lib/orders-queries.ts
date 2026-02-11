import type { OrderItemRow } from '@/lib/types';
import {
  buildFts5ItemBrandQuery,
  escapeLikePrefix,
  TRIGRAM_MIN_LENGTH,
} from './search-utils';

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
    const trimmed = search.trim();
    const likePrefix = escapeLikePrefix(trimmed) + '%';
    const likeContains = '%' + escapeLikePrefix(trimmed) + '%';
    const useTrigram = trimmed.length >= TRIGRAM_MIN_LENGTH;
    const ftsQuery = buildFts5ItemBrandQuery(trimmed);

    if (useTrigram && ftsQuery) {
      conditions.push(
        `(
            i.id IN (SELECT rowid FROM items_fts WHERE items_fts MATCH ?)
            OR COALESCE(oo.new_order_number, o.order_number) LIKE ? ESCAPE '\\'
            OR o.shop_domain LIKE ? ESCAPE '\\'
            OR COALESCE(oo.shop_name, o.shop_name) LIKE ? ESCAPE '\\'
            OR COALESCE(io.item_name, i.item_name) LIKE ? ESCAPE '\\'
            OR COALESCE(CASE WHEN io.brand IS NOT NULL THEN io.brand ELSE i.brand END, '') LIKE ? ESCAPE '\\'
          )`
      );
      args.push(
        ftsQuery,
        likePrefix,
        likePrefix,
        likePrefix,
        likeContains,
        likeContains
      );
    } else {
      conditions.push(
        `(
            COALESCE(oo.new_order_number, o.order_number) LIKE ? ESCAPE '\\'
            OR o.shop_domain LIKE ? ESCAPE '\\'
            OR COALESCE(oo.shop_name, o.shop_name) LIKE ? ESCAPE '\\'
            OR COALESCE(io.item_name, i.item_name) LIKE ? ESCAPE '\\'
            OR COALESCE(CASE WHEN io.brand IS NOT NULL THEN io.brand ELSE i.brand END, '') LIKE ? ESCAPE '\\'
          )`
      );
      args.push(likePrefix, likePrefix, likePrefix, likeContains, likeContains);
    }
  }
  if (shopDomain) {
    // UI のフィルタは「表示名（shop_name or shop_domain）」を選ぶため、表示値で一致させる
    conditions.push('COALESCE(oo.shop_name, o.shop_name, o.shop_domain) = ?');
    args.push(shopDomain);
  }
  if (year) {
    conditions.push(
      "strftime('%Y', COALESCE(oo.order_date, o.order_date)) = ?"
    );
    args.push(String(year));
  }
  if (priceMin != null) {
    conditions.push('COALESCE(io.price, i.price) >= ?');
    args.push(priceMin);
  }
  if (priceMax != null) {
    conditions.push('COALESCE(io.price, i.price) <= ?');
    args.push(priceMax);
  }

  const orderCol =
    sortBy === 'price'
      ? 'COALESCE(io.price, i.price)'
      : 'COALESCE(oo.order_date, o.order_date, o.created_at)';
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
      o.order_number AS originalOrderNumber,
      o.order_date AS originalOrderDate,
      o.shop_name AS originalShopName,
      i.item_name AS originalItemName,
      COALESCE(i.brand, '') AS originalBrand,
      i.price AS originalPrice,
      i.quantity AS originalQuantity,
      i.category AS originalCategory,
      COALESCE(io.item_name, i.item_name) AS itemName,
      i.item_name_normalized AS itemNameNormalized,
      COALESCE(io.price, i.price) AS price,
      COALESCE(io.quantity, i.quantity) AS quantity,
      CASE WHEN io.category IS NOT NULL THEN io.category ELSE i.category END AS category,
      CASE WHEN io.brand IS NOT NULL THEN io.brand ELSE i.brand END AS brand,
      i.created_at AS createdAt,
      COALESCE(oo.shop_name, o.shop_name) AS shopName,
      o.shop_domain AS shopDomain,
      COALESCE(oo.new_order_number, o.order_number) AS orderNumber,
      COALESCE(oo.order_date, o.order_date) AS orderDate,
      img.file_name AS fileName,
      ld.delivery_status AS deliveryStatus,
      pm.maker,
      pm.series,
      pm.product_name AS productName,
      pm.scale,
      pm.is_reissue AS isReissue,
      CASE
        WHEN io.item_name IS NOT NULL
          OR io.price IS NOT NULL
          OR io.quantity IS NOT NULL
          OR io.brand IS NOT NULL
          OR io.category IS NOT NULL
          OR oo.new_order_number IS NOT NULL
          OR oo.order_date IS NOT NULL
          OR oo.shop_name IS NOT NULL
        THEN 1 ELSE 0
      END AS hasOverride
    FROM items i
    JOIN orders o ON i.order_id = o.id
    LEFT JOIN latest_delivery ld ON ld.order_id = o.id
    -- item_name_normalized で JOIN: 同じ正規化商品名の複数アイテムが同一画像を共有（意図した動作）
    LEFT JOIN images img ON img.item_name_normalized = i.item_name_normalized
    -- product_master: 正規化できない商品名（NULL）の item は product_master データを表示しない（意図した動作）
    LEFT JOIN product_master pm ON i.item_name_normalized = pm.normalized_name
    -- 手動上書き: 上書きテーブルからビジネスキーで JOIN
    LEFT JOIN item_overrides io ON io.shop_domain = o.shop_domain
        AND io.order_number = o.order_number COLLATE NOCASE
        AND io.original_item_name = i.item_name
        AND io.original_brand = COALESCE(i.brand, '')
    LEFT JOIN order_overrides oo ON oo.shop_domain = o.shop_domain
        AND oo.order_number = o.order_number COLLATE NOCASE
    -- 除外リスト: 論理削除（一致するレコードを非表示）
    LEFT JOIN excluded_items ei ON ei.shop_domain = o.shop_domain
        AND ei.order_number = o.order_number COLLATE NOCASE
        AND ei.item_name = i.item_name
        AND ei.brand = COALESCE(i.brand, '')
    LEFT JOIN excluded_orders eo ON eo.shop_domain = o.shop_domain
        AND eo.order_number = o.order_number COLLATE NOCASE
    WHERE ei.id IS NULL AND eo.id IS NULL
      AND ${conditions.join(' AND ')}
    ORDER BY ${orderCol} ${orderDir}
  `;

  const rows = await db.select<OrderItemRow>(sql, args);
  return rows;
}

export async function getOrderItemFilterOptions(db: {
  select: <T>(sql: string, args?: unknown[]) => Promise<T[]>;
}): Promise<{ shopDomains: string[]; years: number[] }> {
  const [shops, years] = await Promise.all([
    db.select<{ shop_display: string }>(
      `
        SELECT DISTINCT
          COALESCE(oo.shop_name, o.shop_name, o.shop_domain) AS shop_display
        FROM orders o
        LEFT JOIN order_overrides oo
          ON oo.shop_domain = o.shop_domain
         AND oo.order_number = o.order_number COLLATE NOCASE
        LEFT JOIN excluded_orders eo
          ON eo.shop_domain = o.shop_domain
         AND eo.order_number = o.order_number COLLATE NOCASE
        WHERE eo.id IS NULL
          AND (o.shop_domain IS NOT NULL OR o.shop_name IS NOT NULL OR oo.shop_name IS NOT NULL)
        ORDER BY shop_display
      `
    ),
    db.select<{ yr: string }>(
      `
        SELECT DISTINCT strftime('%Y', COALESCE(oo.order_date, o.order_date)) AS yr
        FROM orders o
        LEFT JOIN order_overrides oo
          ON oo.shop_domain = o.shop_domain
         AND oo.order_number = o.order_number COLLATE NOCASE
        LEFT JOIN excluded_orders eo
          ON eo.shop_domain = o.shop_domain
         AND eo.order_number = o.order_number COLLATE NOCASE
        WHERE eo.id IS NULL
          AND COALESCE(oo.order_date, o.order_date) IS NOT NULL
          AND trim(strftime('%Y', COALESCE(oo.order_date, o.order_date))) != ''
        ORDER BY yr DESC
      `
    ),
  ]);
  return {
    shopDomains: shops.map((r) => r.shop_display),
    years: years.map((r) => parseInt(r.yr, 10)).filter((n) => !isNaN(n)),
  };
}
