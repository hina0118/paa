import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Orders } from './orders';
import { NavigationProvider } from '@/contexts/navigation-context';

const mockGetDb = vi.fn();

vi.mock('@/hooks/useDatabase', () => ({
  useDatabase: () => ({
    getDb: mockGetDb,
  }),
}));

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://${path}`,
  isTauri: () => false,
}));

vi.mock('@tauri-apps/api/path', () => ({
  appDataDir: () => Promise.resolve('/mock/app/data'),
  join: (_a: string, _b: string) => Promise.resolve('/mock/app/data/images'),
}));

const mockDb = {
  select: vi.fn(),
};

const renderOrders = () => {
  return render(
    <NavigationProvider>
      <Orders />
    </NavigationProvider>
  );
};

describe('Orders', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetDb.mockResolvedValue(mockDb);
    mockDb.select.mockImplementation((sql: string) => {
      if (sql.includes('shop_domain')) {
        return Promise.resolve([]);
      }
      if (sql.includes('strftime')) {
        return Promise.resolve([]);
      }
      return Promise.resolve([]);
    });
  });

  it('renders 商品一覧 heading', async () => {
    renderOrders();
    expect(
      screen.getByRole('heading', { name: '商品一覧' })
    ).toBeInTheDocument();
  });

  it('renders search input', () => {
    renderOrders();
    expect(
      screen.getByPlaceholderText('商品名・ショップ名・注文番号で検索')
    ).toBeInTheDocument();
  });

  it('renders filter clear button', () => {
    renderOrders();
    expect(
      screen.getByRole('button', { name: 'フィルタクリア' })
    ).toBeInTheDocument();
  });

  it('renders card and list view toggle buttons', () => {
    renderOrders();
    expect(
      screen.getByRole('button', { name: 'カード表示' })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'リスト表示' })
    ).toBeInTheDocument();
  });

  it('displays item count when loaded', async () => {
    vi.mocked(mockDb.select).mockImplementation((sql: string) => {
      if (sql.includes('shop_domain')) {
        return Promise.resolve([{ shop_domain: 'shop.com' }]);
      }
      if (sql.includes('strftime')) {
        return Promise.resolve([{ yr: '2024' }]);
      }
      return Promise.resolve([
        {
          id: 1,
          orderId: 1,
          itemName: 'Test Item',
          itemNameNormalized: null,
          price: 1000,
          quantity: 1,
          category: null,
          brand: null,
          createdAt: '2024-01-01',
          shopDomain: 'shop.com',
          orderNumber: 'ORD-1',
          orderDate: '2024-01-01',
          fileName: null,
          deliveryStatus: 'delivered',
        },
      ]);
    });
    renderOrders();
    expect(await screen.findByText(/1件の商品/)).toBeInTheDocument();
  });
});
