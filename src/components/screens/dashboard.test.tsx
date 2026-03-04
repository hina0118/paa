import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Toaster } from 'sonner';
import { Dashboard } from './dashboard';
import { mockInvoke, mockListen } from '@/test/setup';

const renderWithProviders = (ui: React.ReactElement) => {
  return render(
    <>
      {ui}
      <Toaster position="top-right" richColors />
    </>
  );
};

const defaultOrderStats = {
  total_orders: 50,
  total_items: 120,
  distinct_items_with_normalized: 75,
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
  not_shipped_over_1_year: 0,
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
  distinct_items_with_normalized: 75,
};

describe('Dashboard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // セットアップのモックを上書き
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_order_stats') {
        return Promise.resolve(defaultOrderStats);
      }
      if (cmd === 'get_delivery_stats') {
        return Promise.resolve(defaultDeliveryStats);
      }
      if (cmd === 'get_product_master_stats') {
        return Promise.resolve(defaultProductMasterStats);
      }
      if (cmd === 'get_misc_stats') {
        return Promise.resolve(defaultMiscStats);
      }
      return Promise.resolve(null);
    });
    mockListen.mockResolvedValue(() => {});
  });

  it('renders dashboard heading', () => {
    renderWithProviders(<Dashboard />);
    expect(
      screen.getByRole('heading', { name: /ダッシュボード/i })
    ).toBeInTheDocument();
  });

  it('renders with correct heading level', () => {
    renderWithProviders(<Dashboard />);
    const heading = screen.getByRole('heading', { name: /ダッシュボード/i });
    expect(heading.tagName).toBe('H1');
  });

  it('applies container styling', () => {
    const { container } = renderWithProviders(<Dashboard />);
    const div = container.querySelector('.container');
    expect(div).toBeInTheDocument();
    expect(div).toHaveClass('mx-auto');
    expect(div).toHaveClass('pt-0');
    expect(div).toHaveClass('pb-10');
  });

  it('applies heading styling', () => {
    renderWithProviders(<Dashboard />);
    const heading = screen.getByRole('heading', { name: /ダッシュボード/i });
    expect(heading).toHaveClass('text-2xl');
    expect(heading).toHaveClass('font-bold');
  });

  it('renders without errors', () => {
    expect(() => renderWithProviders(<Dashboard />)).not.toThrow();
  });

  // 統計データの表示テスト（注文・配送・商品マスタ・その他）
  it('displays order and delivery statistics', async () => {
    renderWithProviders(<Dashboard />);

    await waitFor(() => {
      // 商品数（distinct_items_with_normalized）
      expect(screen.getByText('75')).toBeInTheDocument();
      // 配送状況
      expect(screen.getByText('配送状況')).toBeInTheDocument();
    });
  });

  // エラー処理のテスト（useDashboardStatsはget_order_stats等4種を呼ぶ。いずれかが失敗するとloadErrorになる）
  it('displays error message when loadStats fails', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_order_stats') {
        return Promise.reject(new Error('Failed to load stats'));
      }
      if (cmd === 'get_delivery_stats')
        return Promise.resolve(defaultDeliveryStats);
      if (cmd === 'get_product_master_stats')
        return Promise.resolve(defaultProductMasterStats);
      if (cmd === 'get_misc_stats') return Promise.resolve(defaultMiscStats);
      return Promise.resolve(null);
    });

    renderWithProviders(<Dashboard />);

    await waitFor(
      () => {
        expect(
          screen.getByText(/データの読み込みに失敗しました/)
        ).toBeInTheDocument();
      },
      { timeout: 3000 }
    );

    expect(consoleSpy).toHaveBeenCalledWith(
      'Failed to load dashboard stats:',
      expect.any(Error)
    );
    consoleSpy.mockRestore();
  });

  it('displays error message for non-Error rejection', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_order_stats') return Promise.reject('String error');
      if (cmd === 'get_delivery_stats')
        return Promise.resolve(defaultDeliveryStats);
      if (cmd === 'get_product_master_stats')
        return Promise.resolve(defaultProductMasterStats);
      if (cmd === 'get_misc_stats') return Promise.resolve(defaultMiscStats);
      return Promise.resolve(null);
    });

    renderWithProviders(<Dashboard />);

    await waitFor(
      () => {
        expect(
          screen.getByText(/データの読み込みに失敗しました/)
        ).toBeInTheDocument();
      },
      { timeout: 3000 }
    );

    consoleSpy.mockRestore();
  });

  // 更新ボタンのテスト
  it('refreshes stats when refresh button is clicked', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Dashboard />);

    // 初期ロード完了を待つ
    await waitFor(() => {
      expect(screen.getByText('75')).toBeInTheDocument();
    });

    const refreshButton = screen.getByRole('button', { name: '更新' });
    await user.click(refreshButton);

    // クリック後も統計が表示され続けること（loadStatsが再実行される）
    await waitFor(() => {
      expect(screen.getByText('75')).toBeInTheDocument();
      expect(screen.getByText('配送状況')).toBeInTheDocument();
    });
  });
});
