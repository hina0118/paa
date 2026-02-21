import { useCallback, useEffect, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { ShoppingCart, Search, LayoutGrid, List } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { PageHeader } from '@/components/ui/page-header';
import { useDebouncedSearch } from '@/hooks/useDebouncedSearch';
import { useOrderFilters } from '@/hooks/useOrderFilters';
import { useOrderItems } from '@/hooks/useOrderItems';
import { OrderItemCard } from '@/components/orders/order-item-card';
import { OrderItemRowView } from '@/components/orders/order-item-row';
import { OrderItemDrawer } from '@/components/orders/order-item-drawer';
import type { OrderItemRow } from '@/lib/types';

const SEARCH_DEBOUNCE_MS = 300;
const CARD_MIN_WIDTH = 200;
// カードは aspect-square のため、列幅に応じて高さが変わる。フォールバック用
const CARD_ROW_HEIGHT_FALLBACK = 450;
// カード本体の高さオフセット（aspect-square 画像以外: Content + Footer + 余白）
const CARD_CONTENT_HEIGHT_OFFSET = 140;
// 行パディング(0.5rem×2=16px) + グリッド行間ギャップ(1rem=16px) → 計32px（style の padding/gap に合わせる）
const CARD_ROW_PADDING_AND_GAP = 16 + 16;
const LIST_ROW_HEIGHT = 80;

export function Orders() {
  // 検索: デバウンス付き
  const { searchInput, searchDebounced, setSearchInput, clearSearch } =
    useDebouncedSearch(SEARCH_DEBOUNCE_MS);

  // フィルタ: shopDomain, year, priceMin, priceMax + ドロップダウン選択肢
  const { filters, setFilter, clearFilters, filterOptions } = useOrderFilters();

  // データ取得 + ソート + ドロワー状態
  const {
    items,
    loading,
    sort,
    setSort,
    selectedItem,
    drawerOpen,
    openDrawer,
    setDrawerOpen,
    handleImageUpdated,
  } = useOrderItems({ searchDebounced, filters });

  // 表示設定（純粋な表示制御のためローカル状態で十分）
  const [viewMode, setViewMode] = useState<'card' | 'list'>('card');
  const [columnCount, setColumnCount] = useState(4);

  const handleClearFilters = () => {
    clearSearch();
    clearFilters();
  };

  return (
    <div className="container mx-auto h-full flex flex-col px-6">
      <PageHeader
        title="商品一覧"
        description={loading ? '読み込み中...' : `${items.length}件の商品`}
        icon={ShoppingCart}
      />

      <div className="mb-6 space-y-4 flex-shrink-0">
        <div className="flex gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="商品名・ショップ名・注文番号で検索"
              value={searchInput}
              onChange={(e) => setSearchInput(e.target.value)}
              className="pl-9"
            />
          </div>
          <Button variant="outline" onClick={handleClearFilters}>
            フィルタクリア
          </Button>
          <div className="flex border rounded-md">
            <Button
              variant={viewMode === 'card' ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setViewMode('card')}
              aria-pressed={viewMode === 'card'}
              aria-label="カード表示"
            >
              <LayoutGrid className="h-4 w-4" />
            </Button>
            <Button
              variant={viewMode === 'list' ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setViewMode('list')}
              aria-pressed={viewMode === 'list'}
              aria-label="リスト表示"
            >
              <List className="h-4 w-4" />
            </Button>
          </div>
        </div>

        <div className="flex flex-wrap gap-4 items-center">
          <div className="flex items-center gap-2">
            <label
              htmlFor="filter-shop"
              className="text-sm text-muted-foreground whitespace-nowrap"
            >
              ショップ:
            </label>
            <select
              id="filter-shop"
              value={filters.shopDomain}
              onChange={(e) => setFilter('shopDomain', e.target.value)}
              className="h-9 rounded-md border border-input bg-background px-3 text-sm transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1"
            >
              <option value="">すべて</option>
              {filterOptions.shopDomains.map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
            </select>
          </div>
          <div className="flex items-center gap-2">
            <label
              htmlFor="filter-year"
              className="text-sm text-muted-foreground whitespace-nowrap"
            >
              購入年:
            </label>
            <select
              id="filter-year"
              value={filters.year}
              onChange={(e) => setFilter('year', e.target.value)}
              className="h-9 rounded-md border border-input bg-background px-3 text-sm transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1"
            >
              <option value="">すべて</option>
              {filterOptions.years.map((y) => (
                <option key={y} value={y}>
                  {y}年
                </option>
              ))}
            </select>
          </div>
          <div className="flex items-center gap-2">
            <label
              htmlFor="filter-delivery-status"
              className="text-sm text-muted-foreground whitespace-nowrap"
            >
              発送状態:
            </label>
            <select
              id="filter-delivery-status"
              value={filters.deliveryStatus}
              onChange={(e) => setFilter('deliveryStatus', e.target.value)}
              className="h-9 rounded-md border border-input bg-background px-3 text-sm transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1"
            >
              <option value="">すべて</option>
              <option value="not_shipped">未発送のみ</option>
              <option value="shipped">発送済みのみ</option>
            </select>
          </div>
          {filters.deliveryStatus === 'not_shipped' && (
            <div className="flex items-center gap-2">
              <label
                htmlFor="filter-elapsed-months"
                className="text-sm text-muted-foreground whitespace-nowrap"
              >
                経過期間:
              </label>
              <select
                id="filter-elapsed-months"
                value={filters.elapsedMonths}
                onChange={(e) => setFilter('elapsedMonths', e.target.value)}
                className="h-9 rounded-md border border-input bg-background px-3 text-sm transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1"
              >
                <option value="3">3ヶ月以上</option>
                <option value="6">6ヶ月以上</option>
                <option value="12">1年以上</option>
                <option value="24">2年以上</option>
              </select>
            </div>
          )}
          <div className="flex items-center gap-2">
            <label
              htmlFor="sort"
              className="text-sm text-muted-foreground whitespace-nowrap"
            >
              並び順:
            </label>
            <select
              id="sort"
              value={`${sort.sortBy}-${sort.sortOrder}`}
              onChange={(e) => {
                const [by, order] = e.target.value.split('-') as [
                  'order_date' | 'price',
                  'asc' | 'desc',
                ];
                setSort({ sortBy: by, sortOrder: order });
              }}
              className="h-9 rounded-md border border-input bg-background px-3 text-sm transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1"
            >
              <option value="order_date-desc">購入日が新しい順</option>
              <option value="order_date-asc">購入日が古い順</option>
              <option value="price-desc">価格が高い順</option>
              <option value="price-asc">価格が安い順</option>
            </select>
          </div>
          <div className="flex items-center gap-2">
            <label
              htmlFor="filter-price-min"
              className="text-sm text-muted-foreground whitespace-nowrap"
            >
              価格:
            </label>
            <Input
              id="filter-price-min"
              type="number"
              placeholder="最小"
              value={filters.priceMin}
              onChange={(e) => setFilter('priceMin', e.target.value)}
              className="w-24 h-9"
            />
            <span className="text-muted-foreground">〜</span>
            <Input
              id="filter-price-max"
              type="number"
              placeholder="最大"
              value={filters.priceMax}
              onChange={(e) => setFilter('priceMax', e.target.value)}
              className="w-24 h-9"
            />
            <span className="text-sm text-muted-foreground">円</span>
          </div>
        </div>
      </div>

      {loading ? (
        <div className="flex-1 flex items-center justify-center text-muted-foreground">
          読み込み中...
        </div>
      ) : items.length === 0 ? (
        <div className="flex-1 flex items-center justify-center text-muted-foreground">
          データがありません
        </div>
      ) : (
        <OrderItemGrid
          items={items}
          viewMode={viewMode}
          columnCount={viewMode === 'list' ? 1 : columnCount}
          onColumnCountChange={setColumnCount}
          onItemClick={openDrawer}
        />
      )}
      <OrderItemDrawer
        item={selectedItem}
        open={drawerOpen}
        onOpenChange={setDrawerOpen}
        onImageUpdated={handleImageUpdated}
        onDataChanged={handleImageUpdated}
      />
    </div>
  );
}

type OrderItemGridProps = {
  items: OrderItemRow[];
  viewMode: 'card' | 'list';
  columnCount: number;
  onColumnCountChange: (n: number) => void;
  onItemClick: (item: OrderItemRow) => void;
};

function OrderItemGrid({
  items,
  viewMode,
  columnCount,
  onColumnCountChange,
  onItemClick,
}: OrderItemGridProps) {
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState(0);

  useEffect(() => {
    const el = scrollContainerRef.current;
    if (!el) return;
    const updateLayout = () => {
      const w = el.clientWidth;
      setContainerWidth(w);
      onColumnCountChange(
        Math.max(2, Math.min(4, Math.floor(w / CARD_MIN_WIDTH)))
      );
    };
    const observer = new ResizeObserver(updateLayout);
    observer.observe(el);
    updateLayout();
    return () => observer.disconnect();
  }, [onColumnCountChange]);

  const getCardRowHeight = useCallback(() => {
    if (containerWidth <= 0 || columnCount <= 0) {
      return CARD_ROW_HEIGHT_FALLBACK;
    }
    const rowPadding = 8 * 2;
    const gap = 16;
    const gapTotal = gap * (columnCount - 1);
    const columnWidth = (containerWidth - rowPadding - gapTotal) / columnCount;
    return columnWidth + CARD_CONTENT_HEIGHT_OFFSET + CARD_ROW_PADDING_AND_GAP;
  }, [containerWidth, columnCount]);

  const rowHeight = viewMode === 'list' ? LIST_ROW_HEIGHT : getCardRowHeight();
  const rowCount = Math.ceil(items.length / columnCount);
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scrollContainerRef.current,
    estimateSize: () => rowHeight,
    overscan: 2,
  });

  return (
    <div
      ref={scrollContainerRef}
      className="flex-1 min-h-0 overflow-auto rounded-lg border"
      style={{ contain: 'strict' }}
    >
      <div
        style={{
          height: virtualizer.getTotalSize(),
          width: '100%',
          position: 'relative',
        }}
      >
        {virtualizer.getVirtualItems().map((virtualRow) => {
          const startIndex = virtualRow.index * columnCount;
          const rowItems = items.slice(startIndex, startIndex + columnCount);
          return (
            <div
              key={virtualRow.key}
              data-index={virtualRow.index}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                transform: `translateY(${virtualRow.start}px)`,
                display: viewMode === 'list' ? 'block' : 'grid',
                gridTemplateColumns:
                  viewMode === 'card'
                    ? `repeat(${columnCount}, minmax(0, 1fr))`
                    : undefined,
                gap: viewMode === 'card' ? '1rem' : undefined,
                padding: viewMode === 'card' ? '0.5rem' : 0,
              }}
            >
              {rowItems.map((item) =>
                viewMode === 'list' ? (
                  <OrderItemRowView
                    key={item.id}
                    item={item}
                    onClick={() => onItemClick(item)}
                  />
                ) : (
                  <OrderItemCard
                    key={item.id}
                    item={item}
                    onClick={() => onItemClick(item)}
                  />
                )
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
