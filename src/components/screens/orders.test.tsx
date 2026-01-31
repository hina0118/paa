import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
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

const defaultMockItem = {
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
  deliveryStatus: 'delivered' as const,
};

const renderOrders = () => {
  return render(
    <NavigationProvider>
      <Orders />
    </NavigationProvider>
  );
};

const setMockWithItems = () => {
  vi.mocked(mockDb.select).mockImplementation((sql: string) => {
    if (sql.includes('SELECT DISTINCT shop_domain')) {
      return Promise.resolve([{ shop_domain: 'shop.com' }]);
    }
    if (sql.includes("strftime('%Y'")) {
      return Promise.resolve([{ yr: '2024' }]);
    }
    return Promise.resolve([defaultMockItem]);
  });
};

const setMockEmpty = () => {
  vi.mocked(mockDb.select).mockImplementation((sql: string) => {
    if (sql.includes('SELECT DISTINCT shop_domain')) {
      return Promise.resolve([]);
    }
    if (sql.includes("strftime('%Y'")) {
      return Promise.resolve([]);
    }
    return Promise.resolve([]);
  });
};

describe('Orders', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetDb.mockResolvedValue(mockDb);
    setMockEmpty();
  });

  it('renders 商品一覧 heading', async () => {
    renderOrders();
    expect(
      await screen.findByRole('heading', { name: '商品一覧' })
    ).toBeInTheDocument();
  });

  it('renders search input', async () => {
    renderOrders();
    expect(
      await screen.findByPlaceholderText('商品名・ショップ名・注文番号で検索')
    ).toBeInTheDocument();
  });

  it('renders filter clear button', async () => {
    renderOrders();
    expect(
      await screen.findByRole('button', { name: 'フィルタクリア' })
    ).toBeInTheDocument();
  });

  it('renders card and list view toggle buttons', async () => {
    renderOrders();
    expect(
      await screen.findByRole('button', { name: 'カード表示' })
    ).toBeInTheDocument();
    expect(
      await screen.findByRole('button', { name: 'リスト表示' })
    ).toBeInTheDocument();
  });

  it('displays item count when loaded', async () => {
    setMockWithItems();
    renderOrders();
    await expect(screen.findByText(/1件の商品/)).resolves.toBeInTheDocument();
  });

  it('displays データがありません when no items', async () => {
    setMockEmpty();
    renderOrders();
    await expect(
      screen.findByText('データがありません')
    ).resolves.toBeInTheDocument();
  });

  it('clears filters when フィルタクリア is clicked', async () => {
    const user = userEvent.setup();
    setMockWithItems();
    renderOrders();
    await screen.findByText(/1件の商品/);

    const searchInput =
      screen.getByPlaceholderText('商品名・ショップ名・注文番号で検索');
    await user.type(searchInput, 'query');
    expect(searchInput).toHaveValue('query');

    const clearButton = screen.getByRole('button', { name: 'フィルタクリア' });
    await user.click(clearButton);

    expect(searchInput).toHaveValue('');
  });

  it('switches to list view when リスト表示 is clicked', async () => {
    const user = userEvent.setup();
    setMockWithItems();
    renderOrders();
    await screen.findByText(/1件の商品/);

    const listButton = screen.getByRole('button', { name: 'リスト表示' });
    await user.click(listButton);

    expect(listButton).toHaveAttribute('aria-pressed', 'true');
  });

  it('switches to card view when カード表示 is clicked', async () => {
    const user = userEvent.setup();
    setMockWithItems();
    renderOrders();
    await screen.findByText(/1件の商品/);

    const listButton = screen.getByRole('button', { name: 'リスト表示' });
    await user.click(listButton);

    const cardButton = screen.getByRole('button', { name: 'カード表示' });
    await user.click(cardButton);

    expect(cardButton).toHaveAttribute('aria-pressed', 'true');
  });

  it('shows empty list when loadItems fails', async () => {
    vi.mocked(mockDb.select).mockImplementation(() =>
      Promise.reject(new Error('DB error'))
    );
    renderOrders();

    await expect(
      screen.findByText('データがありません')
    ).resolves.toBeInTheDocument();
  });

  it('changes sort when sort select is changed', async () => {
    const user = userEvent.setup();
    setMockWithItems();
    renderOrders();
    await screen.findByText(/1件の商品/);

    const sortSelect = document.getElementById('sort') as HTMLSelectElement;
    await user.selectOptions(sortSelect, 'price-asc');

    expect(sortSelect).toHaveValue('price-asc');
  });

  it('changes shop filter when shop select is changed', async () => {
    const user = userEvent.setup();
    setMockWithItems();
    renderOrders();
    await screen.findByText(/1件の商品/);

    const shopSelect = document.getElementById(
      'filter-shop'
    ) as HTMLSelectElement;
    await user.selectOptions(shopSelect, 'shop.com');

    expect(shopSelect).toHaveValue('shop.com');
  });

  it('changes price filter when price inputs are filled', async () => {
    const user = userEvent.setup();
    setMockWithItems();
    renderOrders();
    await screen.findByText(/1件の商品/);

    const priceMin = document.getElementById(
      'filter-price-min'
    ) as HTMLInputElement;
    const priceMax = document.getElementById(
      'filter-price-max'
    ) as HTMLInputElement;
    await user.type(priceMin, '100');
    await user.type(priceMax, '5000');

    expect(priceMin).toHaveValue(100);
    expect(priceMax).toHaveValue(5000);
  });

  it('changes year filter when year select is changed', async () => {
    const user = userEvent.setup();
    setMockWithItems();
    renderOrders();
    await screen.findByText(/1件の商品/);

    const yearSelect = document.getElementById(
      'filter-year'
    ) as HTMLSelectElement;
    await user.selectOptions(yearSelect, '2024');

    expect(yearSelect).toHaveValue('2024');
  });

  it('updates search input when user types', async () => {
    const user = userEvent.setup();
    setMockWithItems();
    renderOrders();
    await screen.findByText(/1件の商品/);

    const searchInput =
      screen.getByPlaceholderText('商品名・ショップ名・注文番号で検索');
    await user.type(searchInput, 'test query');

    expect(searchInput).toHaveValue('test query');
  });
});
