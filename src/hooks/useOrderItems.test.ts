import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useOrderItems } from './useOrderItems';
import type { OrdersFilterState } from './useOrderFilters';
import type { OrderItemRow } from '@/lib/types';

const mockGetDb = vi.fn();

vi.mock('@/hooks/useDatabase', () => ({
  useDatabase: () => ({
    getDb: mockGetDb,
  }),
}));

const mockDb = {
  select: vi.fn(),
};

const EMPTY_FILTERS: OrdersFilterState = {
  shopDomain: '',
  year: '',
  priceMin: '',
  priceMax: '',
};

function makeMockItem(overrides: Partial<OrderItemRow> = {}): OrderItemRow {
  return {
    id: 1,
    orderId: 1,
    originalOrderNumber: 'ORD-1',
    originalOrderDate: '2024-01-01',
    originalShopName: null,
    originalItemName: 'Test Item',
    originalBrand: '',
    originalPrice: 1000,
    originalQuantity: 1,
    originalCategory: null,
    itemOverrideCategory: null,
    itemName: 'Test Item',
    itemNameNormalized: null,
    price: 1000,
    quantity: 1,
    category: null,
    brand: null,
    createdAt: '2024-01-01',
    shopName: 'Test Shop',
    shopDomain: 'test.com',
    orderNumber: 'ORD-1',
    orderDate: '2024-01-01',
    fileName: null,
    deliveryStatus: null,
    maker: null,
    series: null,
    productName: null,
    scale: null,
    isReissue: null,
    hasOverride: 0,
    ...overrides,
  } as OrderItemRow;
}

describe('useOrderItems', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetDb.mockResolvedValue(mockDb);
    mockDb.select.mockResolvedValue([]);
  });

  it('initializes with loading=true and empty items', () => {
    const { result } = renderHook(() =>
      useOrderItems({ searchDebounced: '', filters: EMPTY_FILTERS })
    );

    expect(result.current.loading).toBe(true);
    expect(result.current.items).toEqual([]);
  });

  it('loads items on mount', async () => {
    const mockItem = makeMockItem();
    mockDb.select.mockResolvedValue([mockItem]);

    const { result } = renderHook(() =>
      useOrderItems({ searchDebounced: '', filters: EMPTY_FILTERS })
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.items).toEqual([mockItem]);
  });

  it('initializes with default sort state', () => {
    const { result } = renderHook(() =>
      useOrderItems({ searchDebounced: '', filters: EMPTY_FILTERS })
    );

    expect(result.current.sort).toEqual({
      sortBy: 'order_date',
      sortOrder: 'desc',
    });
  });

  it('setSort updates sort state', async () => {
    mockDb.select.mockResolvedValue([]);

    const { result } = renderHook(() =>
      useOrderItems({ searchDebounced: '', filters: EMPTY_FILTERS })
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    act(() => {
      result.current.setSort({ sortBy: 'price', sortOrder: 'asc' });
    });

    expect(result.current.sort).toEqual({
      sortBy: 'price',
      sortOrder: 'asc',
    });
  });

  it('reloads items when sort changes', async () => {
    mockDb.select.mockResolvedValue([]);

    const { result } = renderHook(() =>
      useOrderItems({ searchDebounced: '', filters: EMPTY_FILTERS })
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const initialCallCount = mockDb.select.mock.calls.length;

    act(() => {
      result.current.setSort({ sortBy: 'price', sortOrder: 'asc' });
    });

    await waitFor(() => {
      expect(mockDb.select.mock.calls.length).toBeGreaterThan(initialCallCount);
    });
  });

  it('reloads items when filters change', async () => {
    mockDb.select.mockResolvedValue([]);

    const { result, rerender } = renderHook(
      ({ filters }: { filters: OrdersFilterState }) =>
        useOrderItems({ searchDebounced: '', filters }),
      { initialProps: { filters: EMPTY_FILTERS } }
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const initialCallCount = mockDb.select.mock.calls.length;

    rerender({
      filters: { ...EMPTY_FILTERS, shopDomain: 'test.com' },
    });

    await waitFor(() => {
      expect(mockDb.select.mock.calls.length).toBeGreaterThan(initialCallCount);
    });
  });

  it('reloads items when searchDebounced changes', async () => {
    mockDb.select.mockResolvedValue([]);

    const { result, rerender } = renderHook(
      ({ search }: { search: string }) =>
        useOrderItems({ searchDebounced: search, filters: EMPTY_FILTERS }),
      { initialProps: { search: '' } }
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const initialCallCount = mockDb.select.mock.calls.length;

    rerender({ search: 'test query' });

    await waitFor(() => {
      expect(mockDb.select.mock.calls.length).toBeGreaterThan(initialCallCount);
    });
  });

  it('initializes drawer state as closed', () => {
    const { result } = renderHook(() =>
      useOrderItems({ searchDebounced: '', filters: EMPTY_FILTERS })
    );

    expect(result.current.selectedItem).toBeNull();
    expect(result.current.drawerOpen).toBe(false);
  });

  it('openDrawer sets selectedItem and opens drawer', () => {
    const { result } = renderHook(() =>
      useOrderItems({ searchDebounced: '', filters: EMPTY_FILTERS })
    );

    const mockItem = makeMockItem();

    act(() => {
      result.current.openDrawer(mockItem);
    });

    expect(result.current.selectedItem).toEqual(mockItem);
    expect(result.current.drawerOpen).toBe(true);
  });

  it('setDrawerOpen controls drawer open state', () => {
    const { result } = renderHook(() =>
      useOrderItems({ searchDebounced: '', filters: EMPTY_FILTERS })
    );

    const mockItem = makeMockItem();

    act(() => {
      result.current.openDrawer(mockItem);
    });

    expect(result.current.drawerOpen).toBe(true);

    act(() => {
      result.current.setDrawerOpen(false);
    });

    expect(result.current.drawerOpen).toBe(false);
  });

  it('handles loadItems failure gracefully', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    mockDb.select.mockRejectedValue(new Error('DB error'));

    const { result } = renderHook(() =>
      useOrderItems({ searchDebounced: '', filters: EMPTY_FILTERS })
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.items).toEqual([]);
    expect(consoleSpy).toHaveBeenCalledWith(
      'Failed to load order items:',
      expect.any(Error)
    );

    consoleSpy.mockRestore();
  });

  it('handleImageUpdated reloads items and updates selectedItem', async () => {
    const originalItem = makeMockItem({ id: 1, fileName: null });
    const updatedItem = makeMockItem({ id: 1, fileName: 'new-image.jpg' });

    // Initial load returns original item
    mockDb.select.mockResolvedValue([originalItem]);

    const { result } = renderHook(() =>
      useOrderItems({ searchDebounced: '', filters: EMPTY_FILTERS })
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    // Open drawer with item
    act(() => {
      result.current.openDrawer(originalItem);
    });

    // Now mock returns updated item
    mockDb.select.mockResolvedValue([updatedItem]);

    // Call handleImageUpdated
    await act(async () => {
      await result.current.handleImageUpdated();
    });

    expect(result.current.selectedItem).toEqual(updatedItem);
    expect(result.current.items).toEqual([updatedItem]);
  });

  it('discards stale request results (requestId pattern)', async () => {
    const item1 = makeMockItem({ id: 1, itemName: 'First' });
    const item2 = makeMockItem({ id: 2, itemName: 'Second' });

    // First call: resolve slowly
    // Second call: resolve immediately with different data
    let resolveFirst: ((value: unknown[]) => void) | null = null;
    let callCount = 0;

    mockDb.select.mockImplementation(() => {
      callCount++;
      if (callCount === 1) {
        return new Promise<unknown[]>((resolve) => {
          resolveFirst = resolve;
        });
      }
      return Promise.resolve([item2]);
    });

    const { result, rerender } = renderHook(
      ({ search }: { search: string }) =>
        useOrderItems({ searchDebounced: search, filters: EMPTY_FILTERS }),
      { initialProps: { search: '' } }
    );

    // Trigger second load before first completes
    rerender({ search: 'query' });

    // Wait for second request to complete
    await waitFor(() => {
      expect(result.current.items).toEqual([item2]);
    });

    // Now resolve the first (stale) request
    resolveFirst?.([item1]);

    // Items should still show second result (stale result is discarded)
    // Give time for any potential state update
    await act(async () => {
      await new Promise((r) => setTimeout(r, 50));
    });

    expect(result.current.items).toEqual([item2]);
  });
});
