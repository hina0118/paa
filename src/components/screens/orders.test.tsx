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
  select: vi.fn().mockResolvedValue([]),
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
    mockDb.select.mockResolvedValue([]);
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
});
