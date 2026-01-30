import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { OrderItemDrawer } from './order-item-drawer';
import type { OrderItemRow } from '@/lib/types';

vi.mock('@/hooks/useImageUrl', () => ({
  useImageUrl: () => () => null,
}));

const mockItem: OrderItemRow = {
  id: 3,
  orderId: 12,
  itemName: 'ドロワー表示テスト',
  itemNameNormalized: null,
  price: 5000,
  quantity: 1,
  category: '書籍',
  brand: '出版社X',
  createdAt: '2024-03-01T00:00:00',
  shopDomain: 'bookshop.example.com',
  orderNumber: 'ORD-003',
  orderDate: '2024-02-28',
  fileName: null,
  deliveryStatus: 'in_transit',
};

describe('OrderItemDrawer', () => {
  it('returns null when item is null', () => {
    const { container } = render(
      <OrderItemDrawer item={null} open={true} onOpenChange={vi.fn()} />
    );
    expect(container.firstChild).toBeNull();
  });

  it('renders item name in drawer title when open', () => {
    render(
      <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
    );
    expect(screen.getByText('ドロワー表示テスト')).toBeInTheDocument();
  });

  it('renders price', () => {
    render(
      <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
    );
    expect(screen.getByText('5,000円')).toBeInTheDocument();
  });

  it('renders shop domain', () => {
    render(
      <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
    );
    expect(screen.getByText('bookshop.example.com')).toBeInTheDocument();
  });

  it('renders order number', () => {
    render(
      <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
    );
    expect(screen.getByText('ORD-003')).toBeInTheDocument();
  });

  it('renders status', () => {
    render(
      <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
    );
    expect(screen.getByText('配送中')).toBeInTheDocument();
  });

  it('renders brand and category', () => {
    render(
      <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
    );
    expect(screen.getByText(/出版社X|書籍/)).toBeInTheDocument();
  });
});
