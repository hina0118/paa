import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useDashboardStats } from './useDashboardStats';
import { mockInvoke } from '@/test/setup';

const defaultEmailStats = {
  total_emails: 100,
  with_body_plain: 80,
  with_body_html: 90,
  without_body: 10,
  avg_plain_length: 500,
  avg_html_length: 2000,
};

const defaultOrderStats = {
  total_orders: 50,
  total_items: 120,
  total_amount: 150000,
};

const defaultDeliveryStats = {
  not_shipped: 7,
  preparing: 5,
  shipped: 15,
  in_transit: 3,
  out_for_delivery: 2,
  delivered: 12,
  failed: 1,
  returned: 0,
  cancelled: 2,
};

const defaultProductMasterStats = {
  product_master_count: 25,
  distinct_items_with_normalized: 75,
  items_with_parsed: 20,
};

const defaultMiscStats = {
  shop_settings_count: 8,
  shop_settings_enabled_count: 6,
  images_count: 15,
};

const setupDefaultMocks = () => {
  mockInvoke.mockImplementation((cmd: string) => {
    switch (cmd) {
      case 'get_email_stats':
        return Promise.resolve(defaultEmailStats);
      case 'get_order_stats':
        return Promise.resolve(defaultOrderStats);
      case 'get_delivery_stats':
        return Promise.resolve(defaultDeliveryStats);
      case 'get_product_master_stats':
        return Promise.resolve(defaultProductMasterStats);
      case 'get_misc_stats':
        return Promise.resolve(defaultMiscStats);
      default:
        return Promise.resolve(null);
    }
  });
};

describe('useDashboardStats', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setupDefaultMocks();
  });

  it('初期状態はすべてnull/falseである', () => {
    const { result } = renderHook(() => useDashboardStats());

    expect(result.current.emailStats).toBeNull();
    expect(result.current.orderStats).toBeNull();
    expect(result.current.deliveryStats).toBeNull();
    expect(result.current.productMasterStats).toBeNull();
    expect(result.current.miscStats).toBeNull();
    expect(result.current.loading).toBe(false);
    expect(result.current.loadError).toBe(false);
  });

  it('loadStats呼び出し中はloadingがtrueになる', async () => {
    const { result } = renderHook(() => useDashboardStats());

    act(() => {
      void result.current.loadStats();
    });

    expect(result.current.loading).toBe(true);

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
  });

  it('loadStats成功時にすべての統計データがセットされる', async () => {
    const { result } = renderHook(() => useDashboardStats());

    await act(async () => {
      await result.current.loadStats();
    });

    expect(result.current.emailStats).toEqual(defaultEmailStats);
    expect(result.current.orderStats).toEqual(defaultOrderStats);
    expect(result.current.deliveryStats).toEqual(defaultDeliveryStats);
    expect(result.current.productMasterStats).toEqual(
      defaultProductMasterStats
    );
    expect(result.current.miscStats).toEqual(defaultMiscStats);
    expect(result.current.loading).toBe(false);
    expect(result.current.loadError).toBe(false);
  });

  it('loadStats失敗時はloadErrorがtrueになる', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_email_stats') {
        return Promise.reject(new Error('Failed to load stats'));
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useDashboardStats());

    await act(async () => {
      await result.current.loadStats();
    });

    expect(result.current.loadError).toBe(true);
    expect(result.current.loading).toBe(false);
    expect(consoleSpy).toHaveBeenCalledWith(
      'Failed to load dashboard stats:',
      expect.any(Error)
    );

    consoleSpy.mockRestore();
  });

  it('loadStats再呼び出し時にloadErrorがリセットされる', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    mockInvoke.mockRejectedValueOnce(new Error('error'));

    const { result } = renderHook(() => useDashboardStats());

    await act(async () => {
      await result.current.loadStats();
    });
    expect(result.current.loadError).toBe(true);

    setupDefaultMocks();

    await act(async () => {
      await result.current.loadStats();
    });
    expect(result.current.loadError).toBe(false);

    consoleSpy.mockRestore();
  });

  it('loadStats関数の参照が安定している（useCallback）', () => {
    const { result, rerender } = renderHook(() => useDashboardStats());
    const first = result.current.loadStats;
    rerender();
    expect(result.current.loadStats).toBe(first);
  });

  it('古いリクエスト結果を破棄する（requestIdパターン）', async () => {
    const firstEmailStats = {
      total_emails: 100,
      with_body_plain: 80,
      with_body_html: 90,
      without_body: 10,
      avg_plain_length: 500,
      avg_html_length: 2000,
    };

    const secondEmailStats = {
      total_emails: 200,
      with_body_plain: 160,
      with_body_html: 180,
      without_body: 20,
      avg_plain_length: 600,
      avg_html_length: 2500,
    };

    // First call: resolve slowly
    // Second call: resolve immediately with different data
    let resolveFirst: ((value: unknown) => void) | null = null;
    let callCount = 0;

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_email_stats') {
        callCount++;
        if (callCount === 1) {
          return new Promise((resolve) => {
            resolveFirst = resolve;
          });
        }
        return Promise.resolve(secondEmailStats);
      }
      // Other stats return defaults
      switch (cmd) {
        case 'get_order_stats':
          return Promise.resolve(defaultOrderStats);
        case 'get_delivery_stats':
          return Promise.resolve(defaultDeliveryStats);
        case 'get_product_master_stats':
          return Promise.resolve(defaultProductMasterStats);
        case 'get_misc_stats':
          return Promise.resolve(defaultMiscStats);
        default:
          return Promise.resolve(null);
      }
    });

    const { result } = renderHook(() => useDashboardStats());

    // First call (will be slow)
    act(() => {
      void result.current.loadStats();
    });

    // Second call before first completes
    await act(async () => {
      await result.current.loadStats();
    });

    // Wait for second request to complete
    await waitFor(() => {
      expect(result.current.emailStats).toEqual(secondEmailStats);
    });

    // Now resolve the first (stale) request
    resolveFirst?.(firstEmailStats);

    // Stats should still show second result (stale result is discarded)
    // Give time for any potential state update
    await act(async () => {
      await new Promise((r) => setTimeout(r, 50));
    });

    expect(result.current.emailStats).toEqual(secondEmailStats);
  });
});
