import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { OrderItemCard } from './order-item-card';
import type { OrderItemRow } from '@/lib/types';

const mockGetImageUrl = vi.fn(() => null);
vi.mock('@/hooks/useImageUrl', () => ({
  useImageUrl: () => mockGetImageUrl,
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
  shopName: 'ホビーサーチ',
  shopDomain: '1999.co.jp',
  orderNumber: 'ORD-001',
  orderDate: '2024-01-10',
  fileName: null,
  deliveryStatus: 'delivered',
};

describe('OrderItemCard', () => {
  beforeEach(() => {
    mockGetImageUrl.mockImplementation(() => null);
  });

  it('renders item name', () => {
    render(<OrderItemCard item={mockItem} />);
    expect(screen.getByText('テスト商品')).toBeInTheDocument();
  });

  it('renders price formatted', () => {
    render(<OrderItemCard item={mockItem} />);
    expect(screen.getByText('3,000円')).toBeInTheDocument();
  });

  it('renders shop domain when shopName is null', () => {
    const itemWithDomainOnly = {
      ...mockItem,
      shopName: null,
      shopDomain: 'shop.example.com',
    };
    render(<OrderItemCard item={itemWithDomainOnly} />);
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

  it('does not render brand/category when both are null', () => {
    const itemNoBrandCategory = {
      ...mockItem,
      brand: null,
      category: null,
    };
    render(<OrderItemCard item={itemNoBrandCategory} />);
    expect(screen.queryByText(/メーカーA|フィギュア/)).not.toBeInTheDocument();
  });

  it('does not call onClick on other key press', () => {
    const onClick = vi.fn();
    render(<OrderItemCard item={mockItem} onClick={onClick} />);
    const card = screen.getByRole('button');
    fireEvent.keyDown(card, { key: 'a' });
    expect(onClick).not.toHaveBeenCalled();
  });

  it('renders image when useImageUrl returns URL', () => {
    mockGetImageUrl.mockImplementation(
      (fileName: string | null) =>
        (fileName ? 'asset:///images/test.jpg' : null) as string | null
    );
    render(<OrderItemCard item={{ ...mockItem, fileName: 'test.jpg' }} />);
    const img = document.querySelector('img[alt="テスト商品"]');
    expect(img).toBeInTheDocument();
    expect(img).toHaveAttribute('src', 'asset:///images/test.jpg');
  });
});
