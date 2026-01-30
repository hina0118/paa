import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { OrderItemCard } from './order-item-card';
import type { OrderItemRow } from '@/lib/types';

vi.mock('@/hooks/useImageUrl', () => ({
  useImageUrl: () => () => null,
}));

const mockItem: OrderItemRow = {
  id: 1,
  orderId: 10,
  itemName: 'テスト商品',
  itemNameNormalized: null,
  price: 3000,
  quantity: 1,
  category: 'フィギュア',
  brand: 'メーカーA',
  createdAt: '2024-01-15T00:00:00',
  shopDomain: 'shop.example.com',
  orderNumber: 'ORD-001',
  orderDate: '2024-01-10',
  fileName: null,
  deliveryStatus: 'delivered',
};

describe('OrderItemCard', () => {
  it('renders item name', () => {
    render(<OrderItemCard item={mockItem} />);
    expect(screen.getByText('テスト商品')).toBeInTheDocument();
  });

  it('renders price formatted', () => {
    render(<OrderItemCard item={mockItem} />);
    expect(screen.getByText('3,000円')).toBeInTheDocument();
  });

  it('renders shop domain', () => {
    render(<OrderItemCard item={mockItem} />);
    expect(screen.getByText('shop.example.com')).toBeInTheDocument();
  });

  it('renders status badge', () => {
    render(<OrderItemCard item={mockItem} />);
    expect(screen.getByText('到着済み')).toBeInTheDocument();
  });

  it('renders brand and category', () => {
    render(<OrderItemCard item={mockItem} />);
    expect(screen.getByText(/メーカーA.*フィギュア/)).toBeInTheDocument();
  });

  it('calls onClick when clicked', () => {
    const onClick = vi.fn();
    render(<OrderItemCard item={mockItem} onClick={onClick} />);
    const card = screen.getByRole('button');
    fireEvent.click(card);
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('calls onClick on Enter key', () => {
    const onClick = vi.fn();
    render(<OrderItemCard item={mockItem} onClick={onClick} />);
    const card = screen.getByRole('button');
    fireEvent.keyDown(card, { key: 'Enter' });
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('calls onClick on Space key', () => {
    const onClick = vi.fn();
    render(<OrderItemCard item={mockItem} onClick={onClick} />);
    const card = screen.getByRole('button');
    fireEvent.keyDown(card, { key: ' ' });
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('has no button role when onClick is not provided', () => {
    render(<OrderItemCard item={mockItem} />);
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('shows placeholder when orderDate is null', () => {
    const itemNoDate = { ...mockItem, orderDate: null };
    render(<OrderItemCard item={itemNoDate} />);
    expect(screen.getByText('-')).toBeInTheDocument();
  });
});
