import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { OrderItemRowView } from './order-item-row';
import type { OrderItemRow } from '@/lib/types';

const mockGetImageUrl = vi.hoisted(() =>
  vi.fn(() => (_fn: string | null) => null)
);

vi.mock('@/hooks/useImageUrl', () => ({
  useImageUrl: () => mockGetImageUrl(),
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
  beforeEach(() => {
    mockGetImageUrl.mockReturnValue((_fn: string | null) => null);
  });

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
    expect(screen.getByText(/another-shop\.com/)).toBeInTheDocument();
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

  it('calls onClick when Enter key is pressed', () => {
    const onClick = vi.fn();
    render(<OrderItemRowView item={mockItem} onClick={onClick} />);
    const row = screen.getByRole('button');
    fireEvent.keyDown(row, { key: 'Enter' });
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('calls onClick when Space key is pressed', () => {
    const onClick = vi.fn();
    render(<OrderItemRowView item={mockItem} onClick={onClick} />);
    const row = screen.getByRole('button');
    fireEvent.keyDown(row, { key: ' ' });
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('does not call onClick for other keys', () => {
    const onClick = vi.fn();
    render(<OrderItemRowView item={mockItem} onClick={onClick} />);
    const row = screen.getByRole('button');
    fireEvent.keyDown(row, { key: 'a' });
    expect(onClick).not.toHaveBeenCalled();
  });

  it('renders image when useImageUrl returns URL', () => {
    mockGetImageUrl.mockReturnValue((fn: string | null) =>
      fn ? 'asset:///images/test.jpg' : null
    );

    const itemWithImage: OrderItemRow = { ...mockItem, fileName: 'test.jpg' };
    render(<OrderItemRowView item={itemWithImage} />);

    const img = screen.getByRole('img', { name: 'リスト表示テスト' });
    expect(img).toBeInTheDocument();
    expect(img).toHaveAttribute('src', 'asset:///images/test.jpg');
  });

  it('has no button role when onClick is not provided', () => {
    render(<OrderItemRowView item={mockItem} />);
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('renders brand when provided', () => {
    const itemWithBrand: OrderItemRow = { ...mockItem, brand: 'TestBrand' };
    render(<OrderItemRowView item={itemWithBrand} />);
    expect(screen.getByText('TestBrand')).toBeInTheDocument();
  });

  it('renders category when provided', () => {
    const itemWithCategory: OrderItemRow = {
      ...mockItem,
      category: 'TestCategory',
    };
    render(<OrderItemRowView item={itemWithCategory} />);
    expect(screen.getByText('TestCategory')).toBeInTheDocument();
  });

  it('renders brand and category joined with slash', () => {
    const itemWithBoth: OrderItemRow = {
      ...mockItem,
      brand: 'BrandA',
      category: 'CategoryB',
    };
    render(<OrderItemRowView item={itemWithBoth} />);
    expect(screen.getByText('BrandA / CategoryB')).toBeInTheDocument();
  });
});
