import { useCallback, useEffect, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { ShoppingCart, Search, LayoutGrid, List } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { useDatabase } from '@/hooks/useDatabase';
import {
  loadOrderItems,
  getOrderItemFilterOptions,
} from '@/lib/orders-queries';
import { parseNumericFilter } from '@/lib/utils';
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
  const { getDb } = useDatabase();
  const [items, setItems] = useState<OrderItemRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchInput, setSearchInput] = useState('');
  const [searchDebounced, setSearchDebounced] = useState('');
  const [shopDomain, setShopDomain] = useState<string>('');
  const [year, setYear] = useState<string>('');
  const [priceMin, setPriceMin] = useState<string>('');
  const [priceMax, setPriceMax] = useState<string>('');
  const [filterOptions, setFilterOptions] = useState<{
    shopDomains: string[];
    years: number[];
  }>({ shopDomains: [], years: [] });
  const [columnCount, setColumnCount] = useState(4);
  const [viewMode, setViewMode] = useState<'card' | 'list'>('card');
  const [sortBy, setSortBy] = useState<'order_date' | 'price'>('order_date');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');
  const [selectedItem, setSelectedItem] = useState<OrderItemRow | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const loadItemsRequestId = useRef(0);

  useEffect(() => {
    const timer = setTimeout(() => {
      setSearchDebounced(searchInput);
    }, SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [searchInput]);

  const loadFilters = useCallback(async () => {
    try {
      const db = await getDb();
      const options = await getOrderItemFilterOptions(db);
      setFilterOptions(options);
    } catch (err) {
      console.error('Failed to load filter options:', err);
    }
  }, [getDb]);

  const loadItems = useCallback(async () => {
    const requestId = (loadItemsRequestId.current += 1);
    setLoading(true);
    try {
      const db = await getDb();
      const rows = await loadOrderItems(db, {
        search: searchDebounced || undefined,
        shopDomain: shopDomain || undefined,
        year: parseNumericFilter(year),
        priceMin: parseNumericFilter(priceMin),
        priceMax: parseNumericFilter(priceMax),
        sortBy,
        sortOrder,
      });
      if (requestId === loadItemsRequestId.current) {
        setItems(rows);
      }
    } catch (err) {
      if (requestId === loadItemsRequestId.current) {
        console.error('Failed to load order items:', err);
        setItems([]);
      }
    } finally {
      if (requestId === loadItemsRequestId.current) {
        setLoading(false);
      }
    }
  }, [
    getDb,
    searchDebounced,
    shopDomain,
    year,
    priceMin,
    priceMax,
    sortBy,
    sortOrder,
  ]);

  useEffect(() => {
    loadFilters();
  }, [loadFilters]);

  useEffect(() => {
    loadItems();
  }, [loadItems]);

  const handleClearFilters = () => {
    setSearchInput('');
    setSearchDebounced('');
    setShopDomain('');
    setYear('');
    setPriceMin('');
    setPriceMax('');
  };

  return (
    <div className="container mx-auto py-10 px-6">
      <div className="mb-8 space-y-2">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-primary/10">
            <ShoppingCart className="h-6 w-6 text-primary" />
          </div>
          <div>
            <h1 className="text-3xl font-bold tracking-tight">商品一覧</h1>
            <p className="text-sm text-muted-foreground mt-1">
              {loading ? '読み込み中...' : `${items.length}件の商品`}
            </p>
          </div>
        </div>
      </div>

      <div className="mb-6 space-y-4">
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
              value={shopDomain}
              onChange={(e) => setShopDomain(e.target.value)}
              className="h-9 rounded-md border border-input bg-transparent px-3 text-sm"
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
              value={year}
              onChange={(e) => setYear(e.target.value)}
              className="h-9 rounded-md border border-input bg-transparent px-3 text-sm"
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
              htmlFor="sort"
              className="text-sm text-muted-foreground whitespace-nowrap"
            >
              並び順:
            </label>
            <select
              id="sort"
              value={`${sortBy}-${sortOrder}`}
              onChange={(e) => {
                const [by, order] = e.target.value.split('-') as [
                  'order_date' | 'price',
                  'asc' | 'desc',
                ];
                setSortBy(by);
                setSortOrder(order);
              }}
              className="h-9 rounded-md border border-input bg-transparent px-3 text-sm"
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
              value={priceMin}
              onChange={(e) => setPriceMin(e.target.value)}
              className="w-24 h-9"
            />
            <span className="text-muted-foreground">〜</span>
            <Input
              id="filter-price-max"
              type="number"
              placeholder="最大"
              value={priceMax}
              onChange={(e) => setPriceMax(e.target.value)}
              className="w-24 h-9"
            />
            <span className="text-sm text-muted-foreground">円</span>
          </div>
        </div>
      </div>

      {loading ? (
        <div className="text-muted-foreground py-12 text-center">
          読み込み中...
        </div>
      ) : items.length === 0 ? (
        <div className="text-muted-foreground py-12 text-center">
          データがありません
        </div>
      ) : (
        <OrderItemGrid
          items={items}
          viewMode={viewMode}
          columnCount={viewMode === 'list' ? 1 : columnCount}
          onColumnCountChange={setColumnCount}
          onItemClick={(item) => {
            setSelectedItem(item);
            setDrawerOpen(true);
          }}
        />
      )}
      <OrderItemDrawer
        item={selectedItem}
        open={drawerOpen}
        onOpenChange={setDrawerOpen}
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
      className="h-[calc(100vh-20rem)] overflow-auto rounded-lg border"
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
