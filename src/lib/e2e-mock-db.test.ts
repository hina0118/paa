import { describe, it, expect } from 'vitest';
import { createE2EMockDb } from './e2e-mock-db';

describe('createE2EMockDb', () => {
  it('returns object with select and close methods', () => {
    const db = createE2EMockDb();
    expect(db).toHaveProperty('select');
    expect(db).toHaveProperty('close');
    expect(typeof db.select).toBe('function');
    expect(typeof db.close).toBe('function');
  });

  it('close resolves without error', async () => {
    const db = createE2EMockDb();
    await expect(db.close()).resolves.toBeUndefined();
  });

  describe('select - PRAGMA table_info', () => {
    it('returns images schema for images table', async () => {
      const db = createE2EMockDb();
      const result = await db.select<{ name: string }>(
        'PRAGMA table_info(images)'
      );
      // item_id 削除、item_name_normalized がリレーションキーに。4 カラム
      expect(result).toHaveLength(4);
      expect(result.map((r) => r.name)).toEqual([
        'id',
        'item_name_normalized',
        'file_name',
        'created_at',
      ]);
    });

    it('returns orders schema for orders table', async () => {
      const db = createE2EMockDb();
      const result = await db.select<{ name: string }>(
        'PRAGMA table_info(orders)'
      );
      expect(result).toHaveLength(7);
      expect(result.map((r) => r.name)).toContain('order_number');
      expect(result.map((r) => r.name)).toContain('shop_domain');
      expect(result.map((r) => r.name)).toContain('shop_name');
    });

    it('returns shop_settings schema for shop_settings table', async () => {
      const db = createE2EMockDb();
      const result = await db.select<{ name: string }>(
        'PRAGMA table_info(shop_settings)'
      );
      expect(result).toHaveLength(8);
      expect(result.map((r) => r.name)).toContain('shop_name');
      expect(result.map((r) => r.name)).toContain('parser_type');
    });

    it('returns minimal schema for other tables', async () => {
      const db = createE2EMockDb();
      const result = await db.select<{ name: string }>(
        'PRAGMA table_info(emails)'
      );
      expect(result).toHaveLength(1);
      expect(result[0].name).toBe('id');
    });

    it('returns empty array for invalid table name in PRAGMA', async () => {
      const db = createE2EMockDb();
      const result = await db.select(
        'PRAGMA table_info(invalid_sql_injection)'
      );
      expect(result).toEqual([]);
    });

    it('returns empty array when PRAGMA has no match', async () => {
      const db = createE2EMockDb();
      const result = await db.select('PRAGMA table_info');
      expect(result).toEqual([]);
    });
  });

  describe('select - COUNT(*)', () => {
    it('returns shop_settings count (55)', async () => {
      const db = createE2EMockDb();
      const result = await db.select<{ count: number }>(
        'SELECT COUNT(*) as count FROM shop_settings'
      );
      expect(result).toEqual([{ count: 55 }]);
    });

    it('returns orders count (1)', async () => {
      const db = createE2EMockDb();
      const result = await db.select<{ count: number }>(
        'SELECT COUNT(*) as count FROM orders'
      );
      expect(result).toEqual([{ count: 1 }]);
    });

    it('returns count 0 for other tables', async () => {
      const db = createE2EMockDb();
      const result = await db.select<{ count: number }>(
        'SELECT COUNT(*) as count FROM emails'
      );
      expect(result).toEqual([{ count: 0 }]);
    });
  });

  describe('select - SELECT 1', () => {
    it('returns single row', async () => {
      const db = createE2EMockDb();
      const result = await db.select('SELECT 1');
      expect(result).toEqual([{}]);
    });
  });

  describe('select - shop_settings pagination', () => {
    it('returns paginated shop_settings with limit and offset', async () => {
      const db = createE2EMockDb();
      const result = await db.select<{ id: number; shop_name: string }>(
        'SELECT * FROM shop_settings LIMIT ? OFFSET ?',
        [10, 5]
      );
      expect(result).toHaveLength(10);
      expect(result[0].id).toBe(6);
      expect(result[0].shop_name).toBe('ショップ6');
    });

    it('returns shop_settings with limit only', async () => {
      const db = createE2EMockDb();
      const result = await db.select<{ id: number }>(
        'SELECT * FROM shop_settings LIMIT ? OFFSET ?',
        [3, 0]
      );
      expect(result).toHaveLength(3);
    });
  });

  describe('select - orders pagination', () => {
    it('returns orders with LIMIT and OFFSET', async () => {
      const db = createE2EMockDb();
      const result = await db.select<{ id: number; order_number: string }>(
        'SELECT * FROM orders LIMIT ? OFFSET ?',
        [1, 0]
      );
      expect(result).toHaveLength(1);
      expect(result[0].order_number).toBe('ORD-E2E-001');
    });
  });

  describe('select - order items join', () => {
    it('returns order items for items JOIN orders LEFT JOIN images', async () => {
      const db = createE2EMockDb();
      const result = await db.select<{ itemName: string }>(
        'SELECT * FROM items JOIN orders ON ... LEFT JOIN images ON ...'
      );
      expect(result).toHaveLength(1);
      expect(result[0].itemName).toBe('E2Eテスト商品');
    });
  });

  describe('select - SELECT DISTINCT COALESCE(oo.shop_name, o.shop_name, o.shop_domain)', () => {
    it('returns distinct shop display values for filter options', async () => {
      const db = createE2EMockDb();
      const result = await db.select<{ shop_display: string }>(
        'SELECT DISTINCT COALESCE(oo.shop_name, o.shop_name, o.shop_domain) AS shop_display FROM orders o LEFT JOIN order_overrides oo ON oo.shop_domain = o.shop_domain AND oo.order_number COLLATE NOCASE = o.order_number LEFT JOIN excluded_orders eo ON eo.shop_domain = o.shop_domain AND eo.order_number COLLATE NOCASE = o.order_number WHERE eo.id IS NULL ORDER BY shop_display'
      );
      expect(result).toEqual([{ shop_display: 'example.com' }]);
    });
  });

  describe('select - strftime year', () => {
    it('returns year from order_date', async () => {
      const db = createE2EMockDb();
      const result = await db.select<{ yr: string }>(
        "SELECT strftime('%Y', order_date) as yr FROM orders"
      );
      expect(result).toEqual([{ yr: '2024' }]);
    });
  });

  describe('select - unknown query', () => {
    it('returns empty array for unmatched query', async () => {
      const db = createE2EMockDb();
      const result = await db.select('SELECT * FROM unknown_table');
      expect(result).toEqual([]);
    });
  });
});
