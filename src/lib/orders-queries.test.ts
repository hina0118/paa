import { describe, it, expect, vi } from 'vitest';
import { loadOrderItems, getOrderItemFilterOptions } from './orders-queries';

describe('loadOrderItems', () => {
  it('returns items from db', async () => {
    const mockRows = [
      {
        id: 1,
        orderId: 10,
        itemName: '商品A',
        itemNameNormalized: null,
        price: 1000,
        quantity: 1,
        category: null,
        brand: null,
        createdAt: '2024-01-01',
        shopName: 'ホビーサーチ',
        shopDomain: '1999.co.jp',
        orderNumber: 'ORD-1',
        orderDate: '2024-01-01',
        fileName: null,
        deliveryStatus: 'delivered',
      },
    ];
    const mockDb = {
      select: vi.fn().mockResolvedValue(mockRows),
    };
    const result = await loadOrderItems(mockDb as never);
    expect(result).toEqual(mockRows);
    expect(mockDb.select).toHaveBeenCalled();
  });

  it('applies search filter with FTS5 and prefix match', async () => {
    const mockDb = { select: vi.fn().mockResolvedValue([]) };
    await loadOrderItems(mockDb as never, { search: '商品' });
    const [sql, args] = (mockDb.select as ReturnType<typeof vi.fn>).mock
      .calls[0];
    expect(sql).toContain('items_fts');
    expect(sql).toContain('MATCH');
    expect(sql).toContain("ESCAPE '\\'");
    expect(args).toContain('"商品"');
    expect(args).toContain('商品%');
  });

  it('applies shopDomain filter', async () => {
    const mockDb = { select: vi.fn().mockResolvedValue([]) };
    await loadOrderItems(mockDb as never, { shopDomain: 'shop.example.com' });
    const [, args] = (mockDb.select as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(args).toContain('shop.example.com');
    expect(args.filter((a: unknown) => a === 'shop.example.com')).toHaveLength(
      1
    );
  });

  it('applies year filter', async () => {
    const mockDb = { select: vi.fn().mockResolvedValue([]) };
    await loadOrderItems(mockDb as never, { year: 2024 });
    const [, args] = (mockDb.select as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(args).toContain('2024');
  });

  it('applies price range filter', async () => {
    const mockDb = { select: vi.fn().mockResolvedValue([]) };
    await loadOrderItems(mockDb as never, {
      priceMin: 100,
      priceMax: 5000,
    });
    const [, args] = (mockDb.select as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(args).toContain(100);
    expect(args).toContain(5000);
  });

  it('applies sortBy price and sortOrder asc', async () => {
    const mockDb = { select: vi.fn().mockResolvedValue([]) };
    await loadOrderItems(mockDb as never, {
      sortBy: 'price',
      sortOrder: 'asc',
    });
    const [sql] = (mockDb.select as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(sql).toContain('i.price');
    expect(sql).toContain('ASC');
  });

  it('uses DESC when sortOrder is invalid', async () => {
    const mockDb = { select: vi.fn().mockResolvedValue([]) };
    await loadOrderItems(mockDb as never, {
      sortOrder: 'invalid' as 'asc',
    });
    const [sql] = (mockDb.select as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(sql).toContain('DESC');
  });
});

describe('getOrderItemFilterOptions', () => {
  it('returns shop domains and years', async () => {
    const mockDb = {
      select: vi
        .fn()
        .mockResolvedValueOnce([
          { shop_display: 'ホビーサーチ' },
          { shop_display: 'shop2.com' },
        ])
        .mockResolvedValueOnce([{ yr: '2024' }, { yr: '2023' }]),
    };
    const result = await getOrderItemFilterOptions(mockDb as never);
    expect(result.shopDomains).toEqual(['ホビーサーチ', 'shop2.com']);
    expect(result.years).toEqual([2024, 2023]);
  });

  it('filters out invalid years', async () => {
    const mockDb = {
      select: vi
        .fn()
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce([{ yr: '2024' }, { yr: 'invalid' }]),
    };
    const result = await getOrderItemFilterOptions(mockDb as never);
    expect(result.years).toEqual([2024]);
  });
});
