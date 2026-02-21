import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useOrderFilters } from './useOrderFilters';

const mockGetDb = vi.fn();

vi.mock('@/hooks/useDatabase', () => ({
  useDatabase: () => ({
    getDb: mockGetDb,
  }),
}));

const mockDb = {
  select: vi.fn(),
};

describe('useOrderFilters', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetDb.mockResolvedValue(mockDb);
    // Default: return empty filter options
    mockDb.select.mockResolvedValue([]);
  });

  it('initializes with empty filter values', () => {
    const { result } = renderHook(() => useOrderFilters());

    expect(result.current.filters).toEqual({
      shopDomain: '',
      year: '',
      priceMin: '',
      priceMax: '',
      deliveryStatus: '',
      elapsedMonths: '12',
    });
  });

  it('initializes with empty filter options', () => {
    const { result } = renderHook(() => useOrderFilters());

    expect(result.current.filterOptions).toEqual({
      shopDomains: [],
      years: [],
    });
  });

  it('loads filter options from database on mount', async () => {
    mockDb.select.mockImplementation((sql: string) => {
      if (
        sql.includes('COALESCE(oo.shop_name, o.shop_name, o.shop_domain)') ||
        sql.includes('COALESCE(shop_name, shop_domain)')
      ) {
        return Promise.resolve([
          { shop_display: 'shop-a.com' },
          { shop_display: 'shop-b.com' },
        ]);
      }
      if (sql.includes("strftime('%Y'")) {
        return Promise.resolve([{ yr: '2024' }, { yr: '2023' }]);
      }
      return Promise.resolve([]);
    });

    const { result } = renderHook(() => useOrderFilters());

    await waitFor(() => {
      expect(result.current.filterOptions.shopDomains).toEqual([
        'shop-a.com',
        'shop-b.com',
      ]);
    });

    expect(result.current.filterOptions.years).toEqual([2024, 2023]);
  });

  it('setFilter updates individual filter field', () => {
    const { result } = renderHook(() => useOrderFilters());

    act(() => {
      result.current.setFilter('shopDomain', 'test.com');
    });

    expect(result.current.filters.shopDomain).toBe('test.com');
    expect(result.current.filters.year).toBe('');
    expect(result.current.filters.priceMin).toBe('');
    expect(result.current.filters.priceMax).toBe('');
  });

  it('setFilter updates multiple fields independently', () => {
    const { result } = renderHook(() => useOrderFilters());

    act(() => {
      result.current.setFilter('shopDomain', 'test.com');
    });

    act(() => {
      result.current.setFilter('year', '2024');
    });

    act(() => {
      result.current.setFilter('priceMin', '100');
    });

    act(() => {
      result.current.setFilter('priceMax', '5000');
    });

    expect(result.current.filters).toEqual({
      shopDomain: 'test.com',
      year: '2024',
      priceMin: '100',
      priceMax: '5000',
      deliveryStatus: '',
      elapsedMonths: '12',
    });
  });

  it('setFilter updates deliveryStatus without affecting other fields', () => {
    const { result } = renderHook(() => useOrderFilters());

    act(() => {
      result.current.setFilter('deliveryStatus', 'not_shipped');
    });

    expect(result.current.filters.deliveryStatus).toBe('not_shipped');
    expect(result.current.filters.elapsedMonths).toBe('12');
    expect(result.current.filters.shopDomain).toBe('');
  });

  it('setFilter updates elapsedMonths independently', () => {
    const { result } = renderHook(() => useOrderFilters());

    act(() => {
      result.current.setFilter('elapsedMonths', '6');
    });

    expect(result.current.filters.elapsedMonths).toBe('6');
    expect(result.current.filters.deliveryStatus).toBe('');
  });

  it('clearFilters resets all filter values to empty strings', () => {
    const { result } = renderHook(() => useOrderFilters());

    act(() => {
      result.current.setFilter('shopDomain', 'test.com');
      result.current.setFilter('year', '2024');
      result.current.setFilter('priceMin', '100');
      result.current.setFilter('priceMax', '5000');
    });

    act(() => {
      result.current.clearFilters();
    });

    expect(result.current.filters).toEqual({
      shopDomain: '',
      year: '',
      priceMin: '',
      priceMax: '',
      deliveryStatus: '',
      elapsedMonths: '12',
    });
  });

  it('handles filter options load failure gracefully', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    mockDb.select.mockRejectedValue(new Error('DB error'));

    const { result } = renderHook(() => useOrderFilters());

    await waitFor(() => {
      expect(consoleSpy).toHaveBeenCalledWith(
        'Failed to load filter options:',
        expect.any(Error)
      );
    });

    // Filter options remain empty on error
    expect(result.current.filterOptions).toEqual({
      shopDomains: [],
      years: [],
    });

    consoleSpy.mockRestore();
  });
});
