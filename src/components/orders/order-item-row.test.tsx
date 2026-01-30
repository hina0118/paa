import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { OrderItemRowView } from './order-item-row';
import type { OrderItemRow } from '@/lib/types';

vi.mock('@/hooks/useImageUrl', () => ({
  useImageUrl: () => () => null,
}));

const mockItem: OrderItemRow = {
  id: 2,
  orderId: 11,
  itemName: 'リスト表示テスト',
  itemNameNormalized: null,
  price: 1500,
  quantity: 2,
  category: null,
  brand: null,
  createdAt: '2024-02-01T00:00:00',
  shopDomain: 'another-shop.com',
  orderNumber: 'ORD-002',
  orderDate: null,
  fileName: null,
  deliveryStatus: 'shipped',
};

describe('OrderItemRowView', () => {
  it('renders item name', () => {
    render(<OrderItemRowView item={mockItem} />);
    expect(screen.getByText('リスト表示テスト')).toBeInTheDocument();
  });

  it('renders price formatted', () => {
    render(<OrderItemRowView item={mockItem} />);
    expect(screen.getByText('1,500円')).toBeInTheDocument();
  });

  it('renders shop domain', () => {
    render(<OrderItemRowView item={mockItem} />);
    expect(screen.getByText('another-shop.com')).toBeInTheDocument();
  });

  it('renders status badge', () => {
    render(<OrderItemRowView item={mockItem} />);
    expect(screen.getByText('発送済み')).toBeInTheDocument();
  });

  it('calls onClick when clicked', () => {
    const onClick = vi.fn();
    render(<OrderItemRowView item={mockItem} onClick={onClick} />);
    const row = screen.getByRole('button');
    fireEvent.click(row);
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('has no button role when onClick is not provided', () => {
    render(<OrderItemRowView item={mockItem} />);
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });
});
