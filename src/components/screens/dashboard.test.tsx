import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Dashboard } from './dashboard';
import { ParseProvider } from '@/contexts/parse-provider';
import { SyncProvider } from '@/contexts/sync-provider';
import { mockInvoke, mockListen } from '@/test/setup';

const renderWithProviders = (ui: React.ReactElement) => {
  return render(
    <SyncProvider>
      <ParseProvider>{ui}</ParseProvider>
    </SyncProvider>
  );
};

const defaultEmailStats = {
  total_emails: 100,
  with_body_plain: 80,
  with_body_html: 90,
  without_body: 10,
  avg_plain_length: 500,
  avg_html_length: 2000,
};

const defaultParseStatus = {
  batch_size: 100,
  parse_status: 'idle',
  last_parse_started_at: null,
  last_parse_completed_at: null,
  last_error_message: null,
  total_parsed_count: 0,
};

const defaultSyncStatus = {
  sync_status: 'idle',
  total_synced_count: 0,
  batch_size: 100,
  max_iterations: 10,
  last_sync_started_at: null,
  last_sync_completed_at: null,
  last_error_message: null,
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

describe('Dashboard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // セットアップのモックを上書き
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_email_stats') {
        return Promise.resolve(defaultEmailStats);
      }
      if (cmd === 'get_parse_status') {
        return Promise.resolve(defaultParseStatus);
      }
      if (cmd === 'get_sync_status') {
        return Promise.resolve(defaultSyncStatus);
      }
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
    expect(div).toHaveClass('py-10');
  });

  it('applies heading styling', () => {
    renderWithProviders(<Dashboard />);
    const heading = screen.getByRole('heading', { name: /ダッシュボード/i });
    expect(heading).toHaveClass('text-3xl');
    expect(heading).toHaveClass('font-bold');
  });

  it('renders without errors', () => {
    expect(() => renderWithProviders(<Dashboard />)).not.toThrow();
  });

  // 統計データの表示テスト
  it('displays email statistics', async () => {
    renderWithProviders(<Dashboard />);

    await waitFor(() => {
      // 総メール数
      expect(screen.getByText('100')).toBeInTheDocument();
      // テキスト本文あり
      expect(screen.getByText('80')).toBeInTheDocument();
      // HTML本文あり
      expect(screen.getByText('90')).toBeInTheDocument();
      // 本文なし
      expect(screen.getByText('10')).toBeInTheDocument();
    });
  });

  // エラー処理のテスト
  it('displays error message when loadStats fails', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_email_stats') {
        return Promise.reject(new Error('Failed to load stats'));
      }
      if (cmd === 'get_parse_status') {
        return Promise.resolve(defaultParseStatus);
      }
      if (cmd === 'get_sync_status') {
        return Promise.resolve(defaultSyncStatus);
      }
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

    renderWithProviders(<Dashboard />);

    await waitFor(() => {
      expect(screen.getByText('Failed to load stats')).toBeInTheDocument();
    });

    expect(consoleSpy).toHaveBeenCalledWith(
      'Failed to load dashboard stats:',
      expect.any(Error)
    );
    consoleSpy.mockRestore();
  });

  it('displays error message for non-Error rejection', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_email_stats') {
        return Promise.reject('String error');
      }
      if (cmd === 'get_parse_status') {
        return Promise.resolve(defaultParseStatus);
      }
      if (cmd === 'get_sync_status') {
        return Promise.resolve(defaultSyncStatus);
      }
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

    renderWithProviders(<Dashboard />);

    await waitFor(() => {
      expect(screen.getByText('String error')).toBeInTheDocument();
    });

    consoleSpy.mockRestore();
  });

  // 更新ボタンのテスト
  it('refreshes stats when refresh button is clicked', async () => {
    const user = userEvent.setup();
    renderWithProviders(<Dashboard />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: '更新' })).toBeInTheDocument();
    });

    const refreshButton = screen.getByRole('button', { name: '更新' });
    await user.click(refreshButton);

    await waitFor(() => {
      // get_email_statsが2回呼ばれる（初期ロード + クリック）
      const calls = mockInvoke.mock.calls.filter(
        (call) => call[0] === 'get_email_stats'
      );
      expect(calls.length).toBeGreaterThanOrEqual(2);
    });
  });

  // 平均本文長の表示（formatBytes関数のテスト）
  it('displays average body length', async () => {
    renderWithProviders(<Dashboard />);

    await waitFor(() => {
      // avg_plain_length: 500
      expect(screen.getByText('500 文字')).toBeInTheDocument();
      // avg_html_length: 2000
      expect(screen.getByText('2,000 文字')).toBeInTheDocument();
    });
  });

  // bytes=0のケース
  it('displays zero bytes correctly', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_email_stats') {
        return Promise.resolve({
          ...defaultEmailStats,
          avg_plain_length: 0,
          avg_html_length: 0,
        });
      }
      if (cmd === 'get_parse_status') {
        return Promise.resolve(defaultParseStatus);
      }
      if (cmd === 'get_sync_status') {
        return Promise.resolve(defaultSyncStatus);
      }
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

    renderWithProviders(<Dashboard />);

    await waitFor(() => {
      const zeroTexts = screen.getAllByText('0 文字');
      expect(zeroTexts.length).toBeGreaterThanOrEqual(2);
    });
  });

  // total=0のケース（calculatePercentage関数のテスト）
  it('displays percentage correctly when total is zero', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_email_stats') {
        return Promise.resolve({
          ...defaultEmailStats,
          total_emails: 0,
          with_body_plain: 0,
          with_body_html: 0,
          without_body: 0,
        });
      }
      if (cmd === 'get_parse_status') {
        return Promise.resolve(defaultParseStatus);
      }
      if (cmd === 'get_sync_status') {
        return Promise.resolve(defaultSyncStatus);
      }
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

    renderWithProviders(<Dashboard />);

    await waitFor(() => {
      // 総メール数0の場合のパーセンテージ表示
      const zeroPercents = screen.getAllByText(/0%/);
      expect(zeroPercents.length).toBeGreaterThanOrEqual(1);
    });
  });

  // 本文なしが0の場合
  it('displays correct message when all emails have body', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_email_stats') {
        return Promise.resolve({
          ...defaultEmailStats,
          without_body: 0,
        });
      }
      if (cmd === 'get_parse_status') {
        return Promise.resolve(defaultParseStatus);
      }
      if (cmd === 'get_sync_status') {
        return Promise.resolve(defaultSyncStatus);
      }
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

    renderWithProviders(<Dashboard />);

    await waitFor(() => {
      expect(
        screen.getByText('全てのメールに本文データがあります。')
      ).toBeInTheDocument();
    });
  });

  // パース状態のテスト - running
  it('displays running parse status', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_email_stats') {
        return Promise.resolve(defaultEmailStats);
      }
      if (cmd === 'get_parse_status') {
        return Promise.resolve({
          ...defaultParseStatus,
          parse_status: 'running',
        });
      }
      if (cmd === 'get_sync_status') {
        return Promise.resolve(defaultSyncStatus);
      }
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

    renderWithProviders(<Dashboard />);

    await waitFor(() => {
      expect(screen.getByText('パース中')).toBeInTheDocument();
    });
  });

  // パース状態のテスト - completed
  it('displays completed parse status', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_email_stats') {
        return Promise.resolve(defaultEmailStats);
      }
      if (cmd === 'get_parse_status') {
        return Promise.resolve({
          ...defaultParseStatus,
          parse_status: 'completed',
        });
      }
      if (cmd === 'get_sync_status') {
        return Promise.resolve(defaultSyncStatus);
      }
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

    renderWithProviders(<Dashboard />);

    await waitFor(() => {
      expect(screen.getByText('完了')).toBeInTheDocument();
    });
  });

  // パース状態のテスト - error
  it('displays error parse status', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_email_stats') {
        return Promise.resolve(defaultEmailStats);
      }
      if (cmd === 'get_parse_status') {
        return Promise.resolve({
          ...defaultParseStatus,
          parse_status: 'error',
          last_error_message: 'Parse error occurred',
        });
      }
      if (cmd === 'get_sync_status') {
        return Promise.resolve(defaultSyncStatus);
      }
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

    renderWithProviders(<Dashboard />);

    await waitFor(() => {
      expect(screen.getByText('エラー')).toBeInTheDocument();
      expect(
        screen.getByText(/エラー:.*Parse error occurred/)
      ).toBeInTheDocument();
    });
  });

  // 最終パース完了日時の表示
  it('displays last parse completion time', async () => {
    const completedAt = '2024-01-15T10:30:00Z';
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_email_stats') {
        return Promise.resolve(defaultEmailStats);
      }
      if (cmd === 'get_parse_status') {
        return Promise.resolve({
          ...defaultParseStatus,
          last_parse_completed_at: completedAt,
        });
      }
      if (cmd === 'get_sync_status') {
        return Promise.resolve(defaultSyncStatus);
      }
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

    renderWithProviders(<Dashboard />);

    await waitFor(() => {
      expect(screen.getByText(/最終完了:/)).toBeInTheDocument();
    });
  });

  // 総パース件数の表示
  it('displays total parsed count', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_email_stats') {
        return Promise.resolve(defaultEmailStats);
      }
      if (cmd === 'get_parse_status') {
        return Promise.resolve({
          ...defaultParseStatus,
          total_parsed_count: 250,
        });
      }
      if (cmd === 'get_sync_status') {
        return Promise.resolve(defaultSyncStatus);
      }
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

    renderWithProviders(<Dashboard />);

    await waitFor(() => {
      expect(screen.getByText('250')).toBeInTheDocument();
    });
  });
});
