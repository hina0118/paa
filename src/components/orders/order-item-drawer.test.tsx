import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { OrderItemDrawer } from './order-item-drawer';
import type { OrderItemRow } from '@/lib/types';
import { mockInvoke, mockListen } from '@/test/setup';
import { CLIPBOARD_URL_DETECTED_EVENT } from '@/lib/tauri-events';

const mockGetImageUrl = vi.fn(() => null);
vi.mock('@/hooks/useImageUrl', () => ({
  useImageUrl: () => mockGetImageUrl,
}));

const mockItem: OrderItemRow = {
  id: 3,
  orderId: 12,
  originalOrderNumber: 'ORD-003',
  originalOrderDate: '2024-02-28',
  originalShopName: 'ホビーサーチ',
  originalItemName: 'ドロワー表示テスト',
  originalBrand: '出版社X',
  originalPrice: 5000,
  originalQuantity: 1,
  originalCategory: '書籍',
  itemOverrideCategory: null,
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
  maker: null,
  series: null,
  productName: null,
  scale: null,
  isReissue: null,
  hasOverride: 0,
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

  describe('handleSave - Edit Mode', () => {
    it('enters edit mode when pencil button is clicked', async () => {
      const user = userEvent.setup();
      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      const pencilButton = screen.getByTitle('編集');
      await user.click(pencilButton);

      // 編集モードでは入力フィールドが表示される
      expect(screen.getByLabelText('商品名')).toBeInTheDocument();
      expect(screen.getByLabelText('価格')).toBeInTheDocument();
      expect(screen.getByLabelText('数量')).toBeInTheDocument();
      expect(screen.getByLabelText('メーカー')).toBeInTheDocument();
      expect(screen.getByLabelText('ショップ名')).toBeInTheDocument();
      expect(screen.getByLabelText('注文番号')).toBeInTheDocument();
      expect(screen.getByLabelText('注文日')).toBeInTheDocument();
    });

    it('exits edit mode when cancel button is clicked', async () => {
      const user = userEvent.setup();
      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      // 編集モードに入る
      await user.click(screen.getByTitle('編集'));
      expect(screen.getByLabelText('商品名')).toBeInTheDocument();

      // キャンセルボタンをクリック（複数あるので最初のものを選択）
      const cancelButtons = screen.getAllByRole('button', {
        name: 'キャンセル',
      });
      await user.click(cancelButtons[0]);

      // 編集モードが終了し、表示モードに戻る
      expect(screen.queryByLabelText('商品名')).not.toBeInTheDocument();
    });

    it('initializes form with item data when entering edit mode', async () => {
      const user = userEvent.setup();
      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      await user.click(screen.getByTitle('編集'));

      expect(screen.getByLabelText('商品名')).toHaveValue('ドロワー表示テスト');
      expect(screen.getByLabelText('価格')).toHaveValue(5000);
      expect(screen.getByLabelText('数量')).toHaveValue(1);
      expect(screen.getByLabelText('メーカー')).toHaveValue('出版社X');
      expect(screen.getByLabelText('ショップ名')).toHaveValue('ホビーサーチ');
      expect(screen.getByLabelText('注文番号')).toHaveValue('ORD-003');
      expect(screen.getByLabelText('注文日')).toHaveValue('2024-02-28');
    });

    it('exits edit mode when drawer is closed', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();
      const { rerender } = render(
        <OrderItemDrawer
          item={mockItem}
          open={true}
          onOpenChange={onOpenChange}
        />
      );

      // 編集モードに入る
      await user.click(screen.getByTitle('編集'));
      expect(screen.getByLabelText('商品名')).toBeInTheDocument();

      // ドロワーを閉じる
      rerender(
        <OrderItemDrawer
          item={mockItem}
          open={false}
          onOpenChange={onOpenChange}
        />
      );

      // ドロワーを再度開く
      rerender(
        <OrderItemDrawer
          item={mockItem}
          open={true}
          onOpenChange={onOpenChange}
        />
      );

      // 編集モードは終了している
      expect(screen.queryByLabelText('商品名')).not.toBeInTheDocument();
    });
  });

  describe('handleSave - Item Field Updates', () => {
    it('saves updated item name', async () => {
      const user = userEvent.setup();
      const onDataChanged = vi.fn();
      mockInvoke.mockResolvedValueOnce(undefined);

      render(
        <OrderItemDrawer
          item={mockItem}
          open={true}
          onOpenChange={vi.fn()}
          onDataChanged={onDataChanged}
        />
      );

      await user.click(screen.getByTitle('編集'));
      const itemNameInput = screen.getByLabelText('商品名');
      await user.clear(itemNameInput);
      await user.type(itemNameInput, '新しい商品名');
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('save_item_override', {
          shopDomain: '1999.co.jp',
          orderNumber: 'ORD-003',
          originalItemName: 'ドロワー表示テスト',
          originalBrand: '出版社X',
          itemName: '新しい商品名',
          price: null,
          quantity: null,
          brand: null,
          category: null,
        });
        expect(onDataChanged).toHaveBeenCalled();
      });
    });

    it('saves updated price', async () => {
      const user = userEvent.setup();
      const onDataChanged = vi.fn();
      mockInvoke.mockResolvedValueOnce(undefined);

      render(
        <OrderItemDrawer
          item={mockItem}
          open={true}
          onOpenChange={vi.fn()}
          onDataChanged={onDataChanged}
        />
      );

      await user.click(screen.getByTitle('編集'));
      const priceInput = screen.getByLabelText('価格');
      await user.clear(priceInput);
      await user.type(priceInput, '8000');
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('save_item_override', {
          shopDomain: '1999.co.jp',
          orderNumber: 'ORD-003',
          originalItemName: 'ドロワー表示テスト',
          originalBrand: '出版社X',
          itemName: null,
          price: 8000,
          quantity: null,
          brand: null,
          category: null,
        });
      });
    });

    it('saves updated quantity', async () => {
      const user = userEvent.setup();
      const onDataChanged = vi.fn();
      mockInvoke.mockResolvedValueOnce(undefined);

      render(
        <OrderItemDrawer
          item={mockItem}
          open={true}
          onOpenChange={vi.fn()}
          onDataChanged={onDataChanged}
        />
      );

      await user.click(screen.getByTitle('編集'));
      const quantityInput = screen.getByLabelText('数量');
      await user.clear(quantityInput);
      await user.type(quantityInput, '3');
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('save_item_override', {
          shopDomain: '1999.co.jp',
          orderNumber: 'ORD-003',
          originalItemName: 'ドロワー表示テスト',
          originalBrand: '出版社X',
          itemName: null,
          price: null,
          quantity: 3,
          brand: null,
          category: null,
        });
      });
    });

    it('saves updated brand', async () => {
      const user = userEvent.setup();
      const onDataChanged = vi.fn();
      mockInvoke.mockResolvedValueOnce(undefined);

      render(
        <OrderItemDrawer
          item={mockItem}
          open={true}
          onOpenChange={vi.fn()}
          onDataChanged={onDataChanged}
        />
      );

      await user.click(screen.getByTitle('編集'));
      const brandInput = screen.getByLabelText('メーカー');
      await user.clear(brandInput);
      await user.type(brandInput, '新メーカー');
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('save_item_override', {
          shopDomain: '1999.co.jp',
          orderNumber: 'ORD-003',
          originalItemName: 'ドロワー表示テスト',
          originalBrand: '出版社X',
          itemName: null,
          price: null,
          quantity: null,
          brand: '新メーカー',
          category: null,
        });
      });
    });

    it('saves multiple item fields at once', async () => {
      const user = userEvent.setup();
      const onDataChanged = vi.fn();
      mockInvoke.mockResolvedValueOnce(undefined);

      render(
        <OrderItemDrawer
          item={mockItem}
          open={true}
          onOpenChange={vi.fn()}
          onDataChanged={onDataChanged}
        />
      );

      await user.click(screen.getByTitle('編集'));
      await user.clear(screen.getByLabelText('商品名'));
      await user.type(screen.getByLabelText('商品名'), '新商品');
      await user.clear(screen.getByLabelText('価格'));
      await user.type(screen.getByLabelText('価格'), '6000');
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('save_item_override', {
          shopDomain: '1999.co.jp',
          orderNumber: 'ORD-003',
          originalItemName: 'ドロワー表示テスト',
          originalBrand: '出版社X',
          itemName: '新商品',
          price: 6000,
          quantity: null,
          brand: null,
          category: null,
        });
      });
    });
  });

  describe('handleSave - Order Field Updates', () => {
    it('saves updated shop name', async () => {
      const user = userEvent.setup();
      const onDataChanged = vi.fn();
      mockInvoke.mockResolvedValueOnce(undefined);

      render(
        <OrderItemDrawer
          item={mockItem}
          open={true}
          onOpenChange={vi.fn()}
          onDataChanged={onDataChanged}
        />
      );

      await user.click(screen.getByTitle('編集'));
      const shopNameInput = screen.getByLabelText('ショップ名');
      await user.clear(shopNameInput);
      await user.type(shopNameInput, '新ショップ');
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('save_order_override', {
          shopDomain: '1999.co.jp',
          orderNumber: 'ORD-003',
          newOrderNumber: null,
          orderDate: null,
          shopName: '新ショップ',
        });
      });
    });

    it('saves updated order number', async () => {
      const user = userEvent.setup();
      const onDataChanged = vi.fn();
      mockInvoke.mockResolvedValueOnce(undefined);

      render(
        <OrderItemDrawer
          item={mockItem}
          open={true}
          onOpenChange={vi.fn()}
          onDataChanged={onDataChanged}
        />
      );

      await user.click(screen.getByTitle('編集'));
      const orderNumberInput = screen.getByLabelText('注文番号');
      await user.clear(orderNumberInput);
      await user.type(orderNumberInput, 'NEW-ORD-123');
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('save_order_override', {
          shopDomain: '1999.co.jp',
          orderNumber: 'ORD-003',
          newOrderNumber: 'NEW-ORD-123',
          orderDate: null,
          shopName: null,
        });
      });
    });

    it('saves updated order date', async () => {
      const user = userEvent.setup();
      const onDataChanged = vi.fn();
      mockInvoke.mockResolvedValueOnce(undefined);

      render(
        <OrderItemDrawer
          item={mockItem}
          open={true}
          onOpenChange={vi.fn()}
          onDataChanged={onDataChanged}
        />
      );

      await user.click(screen.getByTitle('編集'));
      const orderDateInput = screen.getByLabelText('注文日');
      await user.clear(orderDateInput);
      await user.type(orderDateInput, '2024-03-15');
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('save_order_override', {
          shopDomain: '1999.co.jp',
          orderNumber: 'ORD-003',
          newOrderNumber: null,
          orderDate: '2024-03-15',
          shopName: null,
        });
      });
    });

    it('saves both item and order updates when both are changed', async () => {
      const user = userEvent.setup();
      const onDataChanged = vi.fn();
      mockInvoke.mockResolvedValue(undefined);

      render(
        <OrderItemDrawer
          item={mockItem}
          open={true}
          onOpenChange={vi.fn()}
          onDataChanged={onDataChanged}
        />
      );

      await user.click(screen.getByTitle('編集'));
      await user.clear(screen.getByLabelText('商品名'));
      await user.type(screen.getByLabelText('商品名'), '変更商品');
      await user.clear(screen.getByLabelText('ショップ名'));
      await user.type(screen.getByLabelText('ショップ名'), '変更ショップ');
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          'save_item_override',
          expect.any(Object)
        );
        expect(mockInvoke).toHaveBeenCalledWith(
          'save_order_override',
          expect.any(Object)
        );
        expect(onDataChanged).toHaveBeenCalled();
      });
    });
  });

  describe('handleSave - Reverting to Original Values', () => {
    it('deletes item override when all item fields are reverted to original', async () => {
      const user = userEvent.setup();
      const itemWithOverride: OrderItemRow = {
        ...mockItem,
        itemName: '編集済み商品名',
        hasOverride: 1,
      };
      mockInvoke.mockResolvedValueOnce(undefined);

      render(
        <OrderItemDrawer
          item={itemWithOverride}
          open={true}
          onOpenChange={vi.fn()}
          onDataChanged={vi.fn()}
        />
      );

      await user.click(screen.getByTitle('編集'));
      const itemNameInput = screen.getByLabelText('商品名');
      await user.clear(itemNameInput);
      await user.type(itemNameInput, 'ドロワー表示テスト'); // 元の値に戻す
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('delete_item_override_by_key', {
          shopDomain: '1999.co.jp',
          orderNumber: 'ORD-003',
          originalItemName: 'ドロワー表示テスト',
          originalBrand: '出版社X',
        });
      });
    });

    it('deletes order override when all order fields are reverted to original', async () => {
      const user = userEvent.setup();
      const itemWithOverride: OrderItemRow = {
        ...mockItem,
        shopName: '変更済みショップ',
        hasOverride: 1,
      };
      mockInvoke.mockResolvedValueOnce(undefined);

      render(
        <OrderItemDrawer
          item={itemWithOverride}
          open={true}
          onOpenChange={vi.fn()}
          onDataChanged={vi.fn()}
        />
      );

      await user.click(screen.getByTitle('編集'));
      const shopNameInput = screen.getByLabelText('ショップ名');
      await user.clear(shopNameInput);
      await user.type(shopNameInput, 'ホビーサーチ'); // 元の値に戻す
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          'delete_order_override_by_key',
          {
            shopDomain: '1999.co.jp',
            orderNumber: 'ORD-003',
          }
        );
      });
    });

    it('maintains category override even when other item fields are reverted', async () => {
      const user = userEvent.setup();
      const itemWithCategoryOverride: OrderItemRow = {
        ...mockItem,
        itemName: '編集済み商品名',
        itemOverrideCategory: 'カスタムカテゴリ',
        hasOverride: 1,
      };
      mockInvoke.mockResolvedValueOnce(undefined);

      render(
        <OrderItemDrawer
          item={itemWithCategoryOverride}
          open={true}
          onOpenChange={vi.fn()}
          onDataChanged={vi.fn()}
        />
      );

      await user.click(screen.getByTitle('編集'));
      const itemNameInput = screen.getByLabelText('商品名');
      await user.clear(itemNameInput);
      await user.type(itemNameInput, 'ドロワー表示テスト'); // 元の値に戻す
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        // category override があるので削除ではなく保存される
        expect(mockInvoke).toHaveBeenCalledWith('save_item_override', {
          shopDomain: '1999.co.jp',
          orderNumber: 'ORD-003',
          originalItemName: 'ドロワー表示テスト',
          originalBrand: '出版社X',
          itemName: null,
          price: null,
          quantity: null,
          brand: null,
          category: 'カスタムカテゴリ',
        });
      });
    });
  });

  describe('handleSave - Validation', () => {
    it('shows error when price is empty', async () => {
      const user = userEvent.setup();
      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      await user.click(screen.getByTitle('編集'));
      const priceInput = screen.getByLabelText('価格');
      await user.clear(priceInput);
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(screen.getByText('価格を入力してください')).toBeInTheDocument();
      });
    });

    it('shows error when price is negative', async () => {
      const user = userEvent.setup();
      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      await user.click(screen.getByTitle('編集'));
      const priceInput = screen.getByLabelText('価格');
      await user.clear(priceInput);
      await user.type(priceInput, '-100');
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(
          screen.getByText('価格は 0 以上で入力してください')
        ).toBeInTheDocument();
      });
    });

    it('shows error when price is not an integer', async () => {
      const user = userEvent.setup();
      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      await user.click(screen.getByTitle('編集'));
      const priceInput = screen.getByLabelText('価格');
      await user.clear(priceInput);
      await user.type(priceInput, '12.5');
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(
          screen.getByText('価格は整数で入力してください')
        ).toBeInTheDocument();
      });
    });

    it('shows error when quantity is empty', async () => {
      const user = userEvent.setup();
      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      await user.click(screen.getByTitle('編集'));
      const quantityInput = screen.getByLabelText('数量');
      await user.clear(quantityInput);
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(screen.getByText('数量を入力してください')).toBeInTheDocument();
      });
    });

    it('shows error when quantity is less than 1', async () => {
      const user = userEvent.setup();
      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      await user.click(screen.getByTitle('編集'));
      const quantityInput = screen.getByLabelText('数量');
      await user.clear(quantityInput);
      await user.type(quantityInput, '0');
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(
          screen.getByText('数量は 1 以上で入力してください')
        ).toBeInTheDocument();
      });
    });

    it('shows error when originalItemName is missing', async () => {
      const user = userEvent.setup();
      const itemWithoutOriginalItemName: OrderItemRow = {
        ...mockItem,
        originalItemName: null as unknown as string,
      };

      render(
        <OrderItemDrawer
          item={itemWithoutOriginalItemName}
          open={true}
          onOpenChange={vi.fn()}
        />
      );

      await user.click(screen.getByTitle('編集'));
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(
          screen.getByText('商品情報が不足しているため保存できません')
        ).toBeInTheDocument();
      });
    });

    it('clears validation error when entering edit mode again', async () => {
      const user = userEvent.setup();
      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      // エラーを発生させる
      await user.click(screen.getByTitle('編集'));
      await user.clear(screen.getByLabelText('価格'));
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(screen.getByText('価格を入力してください')).toBeInTheDocument();
      });

      // 編集モードをキャンセルして再度開く（複数あるので最初のものを選択）
      const cancelButtons = screen.getAllByRole('button', {
        name: 'キャンセル',
      });
      await user.click(cancelButtons[0]);
      await user.click(screen.getByTitle('編集'));

      // エラーがクリアされている
      expect(
        screen.queryByText('価格を入力してください')
      ).not.toBeInTheDocument();
    });

    it('disables save button while saving', async () => {
      const user = userEvent.setup();
      let resolveInvoke: (value: unknown) => void;
      const invokePromise = new Promise((resolve) => {
        resolveInvoke = resolve;
      });
      mockInvoke.mockReturnValueOnce(invokePromise);

      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      await user.click(screen.getByTitle('編集'));
      await user.clear(screen.getByLabelText('商品名'));
      await user.type(screen.getByLabelText('商品名'), '新商品');

      const saveButton = screen.getByRole('button', { name: '保存' });
      await user.click(saveButton);

      await waitFor(() => {
        expect(
          screen.getByRole('button', { name: '保存中...' })
        ).toBeDisabled();
        // Get the cancel button that's in the form (not the X button in header)
        const cancelButtons = screen.getAllByRole('button', {
          name: 'キャンセル',
        });
        // The form cancel button should be disabled (it's the second one, after the X icon button)
        const formCancelButton = cancelButtons.find(
          (btn) => btn.textContent === 'キャンセル'
        );
        expect(formCancelButton).toBeDisabled();
      });

      resolveInvoke(undefined);

      await waitFor(() => {
        expect(
          screen.queryByRole('button', { name: '保存中...' })
        ).not.toBeInTheDocument();
      });
    });
  });

  describe('handleSave - NULL Handling', () => {
    it('handles null shopName correctly when reverting', async () => {
      const user = userEvent.setup();
      const itemWithNullShopName: OrderItemRow = {
        ...mockItem,
        originalShopName: null,
        shopName: '変更済みショップ',
      };
      mockInvoke.mockResolvedValueOnce(undefined);

      render(
        <OrderItemDrawer
          item={itemWithNullShopName}
          open={true}
          onOpenChange={vi.fn()}
          onDataChanged={vi.fn()}
        />
      );

      await user.click(screen.getByTitle('編集'));
      const shopNameInput = screen.getByLabelText('ショップ名');
      await user.clear(shopNameInput);
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith(
          'delete_order_override_by_key',
          {
            shopDomain: '1999.co.jp',
            orderNumber: 'ORD-003',
          }
        );
      });
    });

    it('handles null brand correctly when reverting', async () => {
      const user = userEvent.setup();
      const itemWithNullBrand: OrderItemRow = {
        ...mockItem,
        originalBrand: null,
        brand: '変更済みブランド',
      };
      mockInvoke.mockResolvedValueOnce(undefined);

      render(
        <OrderItemDrawer
          item={itemWithNullBrand}
          open={true}
          onOpenChange={vi.fn()}
          onDataChanged={vi.fn()}
        />
      );

      await user.click(screen.getByTitle('編集'));
      const brandInput = screen.getByLabelText('メーカー');
      await user.clear(brandInput);
      await user.click(screen.getByRole('button', { name: '保存' }));

      await waitFor(() => {
        // Empty string is compared to originalBrand (null becomes ''), so delete is called
        expect(mockInvoke).toHaveBeenCalledWith('save_item_override', {
          shopDomain: '1999.co.jp',
          orderNumber: 'ORD-003',
          originalItemName: 'ドロワー表示テスト',
          originalBrand: null,
          itemName: null,
          price: null,
          quantity: null,
          brand: '',
          category: null,
        });
      });
    });
  });

  describe('handleExclude', () => {
    it('opens exclude confirmation dialog when exclude button is clicked', async () => {
      const user = userEvent.setup();
      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      await user.click(screen.getByTitle('編集'));
      await user.click(screen.getByRole('button', { name: 'この商品を除外' }));

      expect(
        screen.getByRole('heading', { name: '商品を除外' })
      ).toBeInTheDocument();
      expect(
        screen.getByText(
          'この商品を除外しますか？再パース後も表示されなくなります。'
        )
      ).toBeInTheDocument();
    });

    it('closes exclude dialog when cancel is clicked', async () => {
      const user = userEvent.setup();
      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      await user.click(screen.getByTitle('編集'));
      await user.click(screen.getByRole('button', { name: 'この商品を除外' }));

      const dialog = screen.getByRole('dialog');
      await user.click(
        within(dialog).getByRole('button', { name: 'キャンセル' })
      );

      await waitFor(() => {
        expect(
          screen.queryByRole('heading', { name: '商品を除外' })
        ).not.toBeInTheDocument();
      });
    });

    it('excludes item when confirmed', async () => {
      const user = userEvent.setup();
      const onOpenChange = vi.fn();
      const onDataChanged = vi.fn();
      mockInvoke.mockResolvedValueOnce(undefined);

      render(
        <OrderItemDrawer
          item={mockItem}
          open={true}
          onOpenChange={onOpenChange}
          onDataChanged={onDataChanged}
        />
      );

      await user.click(screen.getByTitle('編集'));
      await user.click(screen.getByRole('button', { name: 'この商品を除外' }));

      const dialog = screen.getByRole('dialog');
      await user.click(
        within(dialog).getByRole('button', { name: '除外する' })
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('exclude_item', {
          shopDomain: '1999.co.jp',
          orderNumber: 'ORD-003',
          itemName: 'ドロワー表示テスト',
          brand: '出版社X',
          reason: null,
        });
        expect(onOpenChange).toHaveBeenCalledWith(false);
        expect(onDataChanged).toHaveBeenCalled();
      });
    });

    it('handles exclusion error gracefully', async () => {
      const user = userEvent.setup();
      const consoleError = vi
        .spyOn(console, 'error')
        .mockImplementation(() => {});
      mockInvoke.mockRejectedValueOnce(new Error('Exclusion failed'));

      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      await user.click(screen.getByTitle('編集'));
      await user.click(screen.getByRole('button', { name: 'この商品を除外' }));

      const dialog = screen.getByRole('dialog');
      await user.click(
        within(dialog).getByRole('button', { name: '除外する' })
      );

      await waitFor(() => {
        expect(consoleError).toHaveBeenCalledWith(
          'Failed to exclude item:',
          expect.any(Error)
        );
      });

      consoleError.mockRestore();
    });

    it('uses original keys for exclusion', async () => {
      const user = userEvent.setup();
      const itemWithOverrides: OrderItemRow = {
        ...mockItem,
        orderNumber: 'MODIFIED-ORDER',
        itemName: 'Modified Item',
        brand: 'Modified Brand',
      };
      mockInvoke.mockResolvedValueOnce(undefined);

      render(
        <OrderItemDrawer
          item={itemWithOverrides}
          open={true}
          onOpenChange={vi.fn()}
          onDataChanged={vi.fn()}
        />
      );

      await user.click(screen.getByTitle('編集'));
      await user.click(screen.getByRole('button', { name: 'この商品を除外' }));

      const dialog = screen.getByRole('dialog');
      await user.click(
        within(dialog).getByRole('button', { name: '除外する' })
      );

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('exclude_item', {
          shopDomain: '1999.co.jp',
          orderNumber: 'ORD-003', // original order number
          itemName: 'ドロワー表示テスト', // original item name
          brand: '出版社X', // original brand
          reason: null,
        });
      });
    });
  });

  describe('Clipboard Event Listener', () => {
    beforeEach(() => {
      // モック実装をリセットして、他のテストの影響を受けないようにする
      mockListen.mockReset();
      mockListen.mockResolvedValue(vi.fn());
    });

    it('sets up clipboard event listener when drawer opens', async () => {
      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalledWith(
          CLIPBOARD_URL_DETECTED_EVENT,
          expect.any(Function)
        );
      });
    });

    it('does not set up listener when drawer is closed', () => {
      render(
        <OrderItemDrawer item={mockItem} open={false} onOpenChange={vi.fn()} />
      );

      expect(mockListen).not.toHaveBeenCalled();
    });

    it('does not set up listener when item is null', () => {
      render(
        <OrderItemDrawer item={null} open={true} onOpenChange={vi.fn()} />
      );

      expect(mockListen).not.toHaveBeenCalled();
    });

    it('opens image search dialog with detected URL when clipboard event fires', async () => {
      const mockUnlisten = vi.fn();
      mockListen.mockImplementation((eventName, callback) => {
        if (eventName === CLIPBOARD_URL_DETECTED_EVENT) {
          // イベントを即座に発火
          setTimeout(() => {
            callback({
              payload: {
                url: 'https://example.com/test-image.jpg',
                kind: 'image_url',
                source: 'clipboard',
                detectedAt: '2024-03-01T00:00:00Z',
              },
            });
          }, 0);
        }
        return Promise.resolve(mockUnlisten);
      });

      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      await waitFor(() => {
        expect(
          screen.getByRole('heading', { name: '画像を検索' })
        ).toBeInTheDocument();
      });

      // 検知したURLが入力欄に自動入力されているか確認
      const urlInput = screen.getByPlaceholderText('画像のURLをここに貼り付け');
      await waitFor(() => {
        expect(urlInput).toHaveValue('https://example.com/test-image.jpg');
      });
    });

    it('ignores non-image_url clipboard events', async () => {
      const mockUnlisten = vi.fn();
      let callbackInvoked = false;
      mockListen.mockImplementation((eventName, callback) => {
        if (eventName === CLIPBOARD_URL_DETECTED_EVENT) {
          setTimeout(() => {
            callbackInvoked = true;
            callback({
              payload: {
                url: 'https://example.com/page',
                kind: 'url', // not image_url
                source: 'clipboard',
                detectedAt: '2024-03-01T00:00:00Z',
              },
            });
          }, 0);
        }
        return Promise.resolve(mockUnlisten);
      });

      render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      // コールバックが呼ばれるまで待つ
      await waitFor(() => {
        expect(callbackInvoked).toBe(true);
      });

      // コールバック実行後も画像検索ダイアログが開かないことを確認
      expect(
        screen.queryByRole('heading', { name: '画像を検索' })
      ).not.toBeInTheDocument();
    });

    it('cleans up listener when drawer closes', async () => {
      const mockUnlisten = vi.fn();
      mockListen.mockResolvedValue(mockUnlisten);

      const { rerender } = render(
        <OrderItemDrawer item={mockItem} open={true} onOpenChange={vi.fn()} />
      );

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalled();
      });

      // ドロワーを閉じる
      rerender(
        <OrderItemDrawer item={mockItem} open={false} onOpenChange={vi.fn()} />
      );

      await waitFor(() => {
        expect(mockUnlisten).toHaveBeenCalled();
      });
    });
  });
});
