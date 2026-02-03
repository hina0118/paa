import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { OrderItemDrawer } from './order-item-drawer';
import type { OrderItemRow } from '@/lib/types';
import { mockInvoke } from '@/test/setup';

const mockGetImageUrl = vi.fn(() => null);
vi.mock('@/hooks/useImageUrl', () => ({
  useImageUrl: () => mockGetImageUrl,
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
  shopName: 'ホビーサーチ',
  shopDomain: '1999.co.jp',
  orderNumber: 'ORD-003',
  orderDate: '2024-02-28',
  fileName: null,
  deliveryStatus: 'in_transit',
};

describe('OrderItemDrawer', () => {
  beforeEach(() => {
    mockGetImageUrl.mockImplementation(() => null);
    vi.clearAllMocks();
  });

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

  it('renders shop name (or domain when no name)', () => {
    render(
      <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
    );
    expect(screen.getByText('ホビーサーチ')).toBeInTheDocument();
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

  it('renders 画像なし when no image', () => {
    render(
      <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
    );
    expect(screen.getByText('画像なし')).toBeInTheDocument();
  });

  it('renders image when useImageUrl returns URL', () => {
    mockGetImageUrl.mockImplementation(
      (fileName: string | null) =>
        (fileName ? 'asset:///drawer-img.jpg' : null) as string | null
    );
    render(
      <OrderItemDrawer
        item={{ ...mockItem, fileName: 'drawer.jpg' }}
        open={true}
        onOpenChange={vi.fn()}
      />
    );
    const img = document.querySelector('img[alt="ドロワー表示テスト"]');
    expect(img).toBeInTheDocument();
    expect(img).toHaveAttribute('src', 'asset:///drawer-img.jpg');
  });

  it('does not render brand/category section when both are null', () => {
    const itemWithoutBrandCategory: OrderItemRow = {
      ...mockItem,
      brand: null,
      category: null,
    };
    render(
      <OrderItemDrawer
        item={itemWithoutBrandCategory}
        open={true}
        onOpenChange={vi.fn()}
      />
    );
    expect(screen.queryByText('メーカー / 作品名')).not.toBeInTheDocument();
  });

  it('opens image search on Enter key when focusing image area', async () => {
    const user = userEvent.setup();
    render(
      <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
    );

    const imageArea = screen.getByTitle('クリックして画像を検索');
    imageArea.focus();
    await user.keyboard('{Enter}');

    await waitFor(() => {
      expect(
        screen.getByRole('heading', { name: '画像を検索' })
      ).toBeInTheDocument();
    });
  });

  it('opens image search on Space key when focusing image area', async () => {
    const user = userEvent.setup();
    render(
      <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
    );

    const imageArea = screen.getByTitle('クリックして画像を検索');
    imageArea.focus();
    await user.keyboard(' ');

    await waitFor(() => {
      expect(
        screen.getByRole('heading', { name: '画像を検索' })
      ).toBeInTheDocument();
    });
  });

  it('opens image search when image area is clicked', async () => {
    const user = userEvent.setup();
    render(
      <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
    );

    const imageArea = screen.getByTitle('クリックして画像を検索');
    await user.click(imageArea);

    expect(
      screen.getByRole('heading', { name: '画像を検索' })
    ).toBeInTheDocument();
  });

  it('opens image search when 画像を検索 button is clicked', async () => {
    const user = userEvent.setup();
    render(
      <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
    );

    // ドロワー内のボタン（ダイアログが閉じているときは1つ、開いているときは複数あるため最初のものを使用）
    const searchButtons = screen.getAllByRole('button', { name: /画像を検索/ });
    await user.click(searchButtons[0]);

    expect(
      screen.getByRole('heading', { name: '画像を検索' })
    ).toBeInTheDocument();
  });

  it('renders shop domain when shopName is null', () => {
    const itemWithDomainOnly: OrderItemRow = {
      ...mockItem,
      shopName: null,
      shopDomain: 'example.com',
    };
    render(
      <OrderItemDrawer
        item={itemWithDomainOnly}
        open={true}
        onOpenChange={vi.fn()}
      />
    );
    expect(screen.getByText('example.com')).toBeInTheDocument();
  });

  it('renders hyphen when both shopName and shopDomain are null', () => {
    const itemWithNoShop: OrderItemRow = {
      ...mockItem,
      shopName: null,
      shopDomain: null,
    };
    render(
      <OrderItemDrawer
        item={itemWithNoShop}
        open={true}
        onOpenChange={vi.fn()}
      />
    );
    expect(screen.getByText('-')).toBeInTheDocument();
  });

  it('renders hyphen for order number when null', () => {
    const itemWithNoOrderNumber: OrderItemRow = {
      ...mockItem,
      orderNumber: null,
    };
    render(
      <OrderItemDrawer
        item={itemWithNoOrderNumber}
        open={true}
        onOpenChange={vi.fn()}
      />
    );
    expect(screen.getByText('-')).toBeInTheDocument();
  });

  it('calls onImageUpdated when image is saved via ImageSearchDialog', async () => {
    const user = userEvent.setup();
    const onImageUpdated = vi.fn();
    const mockResults = [
      {
        url: 'https://example.com/img1.jpg',
        thumbnail_url: null,
        width: null,
        height: null,
        title: null,
        mime_type: null,
      },
    ];
    mockInvoke
      .mockResolvedValueOnce(mockResults)
      .mockResolvedValueOnce(undefined);

    render(
      <OrderItemDrawer
        item={mockItem}
        open={true}
        onOpenChange={vi.fn()}
        onImageUpdated={onImageUpdated}
      />
    );

    // ドロワー内のボタンをクリックしてダイアログを開く
    const drawerButtons = screen.getAllByRole('button', { name: /画像を検索/ });
    await user.click(drawerButtons[0]);

    await waitFor(() => {
      expect(
        screen.getByRole('heading', { name: '画像を検索' })
      ).toBeInTheDocument();
    });

    const dialog = screen.getByRole('dialog');
    await user.click(
      within(dialog).getByRole('button', { name: /画像を検索/ })
    );

    await waitFor(() => {
      expect(screen.getByAltText('検索結果 1')).toBeInTheDocument();
    });

    const img = screen.getByAltText('検索結果 1');
    const firstResultButton = img.closest('button');
    if (firstResultButton) {
      await user.click(firstResultButton);
    }

    await user.click(
      screen.getByRole('button', { name: '選択した画像を保存' })
    );

    await waitFor(() => {
      expect(onImageUpdated).toHaveBeenCalled();
    });
  });
});
