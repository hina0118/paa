import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Delivery } from './delivery';
import { buildTrackingUrl } from './delivery-utils';

const mockSelect = vi.fn();

vi.mock('@/lib/database', () => ({
  DatabaseManager: {
    getInstance: () => ({
      getDatabase: () => Promise.resolve({ select: mockSelect }),
    }),
  },
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(),
}));

// ---------------------------------------------------------------------------
// buildTrackingUrl unit tests
// ---------------------------------------------------------------------------

describe('buildTrackingUrl', () => {
  it('returns null when carrier is null', () => {
    expect(buildTrackingUrl(null, '123456789')).toBeNull();
  });

  it('returns null when trackingNumber is null', () => {
    expect(buildTrackingUrl('佐川急便', null)).toBeNull();
  });

  it('returns null when both are null', () => {
    expect(buildTrackingUrl(null, null)).toBeNull();
  });

  it('returns null for unknown carrier', () => {
    expect(buildTrackingUrl('未知の業者', '123456789')).toBeNull();
  });

  it('returns Sagawa URL for carrier containing 佐川', () => {
    const url = buildTrackingUrl('佐川急便', '123456789');
    expect(url).toBe(
      'https://k2k.sagawa-exp.co.jp/p/web/okurijosearch.do?okurijoNo=123456789'
    );
  });

  it('returns Japan Post URL for carrier containing 日本郵便', () => {
    const url = buildTrackingUrl('日本郵便', 'AA1234567890JP');
    expect(url).toBe(
      'https://trackings.post.japanpost.jp/services/srv/search/?requestNo=AA1234567890JP'
    );
  });

  it('returns Japan Post URL for carrier containing ゆうパケット', () => {
    const url = buildTrackingUrl('ゆうパケット', '987654321');
    expect(url).toBe(
      'https://trackings.post.japanpost.jp/services/srv/search/?requestNo=987654321'
    );
  });

  it('returns Japan Post URL for carrier containing ゆうパック', () => {
    const url = buildTrackingUrl('ゆうパック', '987654321');
    expect(url).toBe(
      'https://trackings.post.japanpost.jp/services/srv/search/?requestNo=987654321'
    );
  });

  it('returns Yamato URL for carrier containing ヤマト', () => {
    const url = buildTrackingUrl('ヤマト運輸', '1234567890123');
    expect(url).toBe(
      'https://jizen.kuronekoyamato.co.jp/jizen/servlet/crjz.b.NQ0010?id=1234567890123'
    );
  });

  it('returns Yamato URL for carrier containing クロネコ', () => {
    const url = buildTrackingUrl('クロネコヤマト', '1234567890123');
    expect(url).toBe(
      'https://jizen.kuronekoyamato.co.jp/jizen/servlet/crjz.b.NQ0010?id=1234567890123'
    );
  });

  it('encodes tracking number with special characters', () => {
    const url = buildTrackingUrl('佐川急便', 'ABC 123');
    expect(url).toBe(
      'https://k2k.sagawa-exp.co.jp/p/web/okurijosearch.do?okurijoNo=ABC%20123'
    );
  });

  it('trims whitespace from tracking number', () => {
    const url = buildTrackingUrl('ヤマト運輸', '  123  ');
    expect(url).toBe(
      'https://jizen.kuronekoyamato.co.jp/jizen/servlet/crjz.b.NQ0010?id=123'
    );
  });
});

// ---------------------------------------------------------------------------
// Delivery component filter tests
// ---------------------------------------------------------------------------

const makeRow = (id: number, carrier: string | null, status: string) => ({
  id,
  order_id: id,
  tracking_number: `TRK-${id}`,
  carrier,
  delivery_status: status,
  estimated_delivery: null,
  actual_delivery: null,
  last_checked_at: null,
  order_number: `ORD-${id}`,
  shop_domain: 'shop.example.com',
  order_date: '2024-01-01',
});

describe('Delivery component filters', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockSelect.mockResolvedValue([
      makeRow(1, '佐川急便', 'shipped'),
      makeRow(2, 'ヤマト運輸', 'delivered'),
      makeRow(3, '佐川急便', 'delivered'),
    ]);
  });

  it('renders the page heading', async () => {
    render(<Delivery />);
    expect(
      await screen.findByRole('heading', { name: '配送状況' })
    ).toBeInTheDocument();
  });

  it('shows all rows initially', async () => {
    render(<Delivery />);
    // Wait for data load (total count displayed in description)
    await screen.findByText(/3件の配送レコード/);
  });

  it('filters by carrier', async () => {
    const user = userEvent.setup();
    render(<Delivery />);
    await screen.findByText(/3件の配送レコード/);

    // Click the 佐川急便 carrier filter button
    const sagawaButton = screen.getByRole('button', { name: '佐川急便' });
    await user.click(sagawaButton);

    // Footer should show 2/3 rows
    expect(screen.getByText(/^2 \/ 3 件表示$/)).toBeInTheDocument();
  });

  it('filters by status', async () => {
    const user = userEvent.setup();
    render(<Delivery />);
    await screen.findByText(/3件の配送レコード/);

    // Click the 配達完了 (delivered) status filter button
    const deliveredButton = screen.getByRole('button', { name: '配達完了' });
    await user.click(deliveredButton);

    // Footer should show 2/3 rows
    expect(screen.getByText(/^2 \/ 3 件表示$/)).toBeInTheDocument();
  });

  it('combines carrier and status filters', async () => {
    const user = userEvent.setup();
    render(<Delivery />);
    await screen.findByText(/3件の配送レコード/);

    await user.click(screen.getByRole('button', { name: '佐川急便' }));
    await user.click(screen.getByRole('button', { name: '配達完了' }));

    // Only row 3 (佐川 + delivered) should remain
    expect(screen.getByText(/^1 \/ 3 件表示$/)).toBeInTheDocument();
  });

  it('resets to all rows when すべて is clicked', async () => {
    const user = userEvent.setup();
    render(<Delivery />);
    await screen.findByText(/3件の配送レコード/);

    await user.click(screen.getByRole('button', { name: '佐川急便' }));
    expect(screen.getByText(/^2 \/ 3 件表示$/)).toBeInTheDocument();

    // Click すべて for carrier filters
    const carrierContainer = screen
      .getByText('配送業者:')
      .closest('div') as HTMLElement;
    await user.click(
      within(carrierContainer).getByRole('button', { name: 'すべて' })
    );

    expect(screen.getByText(/^3 \/ 3 件表示$/)).toBeInTheDocument();
  });

  it('shows 該当するレコードがありません when no rows match', async () => {
    const user = userEvent.setup();
    // ヤマト運輸 + 発送済み → 0件
    render(<Delivery />);
    await screen.findByText(/3件の配送レコード/);

    await user.click(screen.getByRole('button', { name: 'ヤマト運輸' }));
    await user.click(screen.getByRole('button', { name: '発送済み' }));

    expect(
      screen.getByText('該当するレコードがありません')
    ).toBeInTheDocument();
  });
});
