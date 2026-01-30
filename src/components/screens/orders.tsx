import { useCallback, useEffect, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { ShoppingCart, Search } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { useDatabase } from '@/hooks/useDatabase';
import {
  loadOrderItems,
  getOrderItemFilterOptions,
} from '@/lib/orders-queries';
import { OrderItemCard } from '@/components/orders/order-item-card';
import type { OrderItemRow } from '@/lib/types';

const SEARCH_DEBOUNCE_MS = 300;
const CARD_MIN_WIDTH = 200;
const ROW_HEIGHT = 320;

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
    setLoading(true);
    try {
      const db = await getDb();
      const rows = await loadOrderItems(db, {
        search: searchDebounced || undefined,
        shopDomain: shopDomain || undefined,
        year: year ? parseInt(year, 10) : undefined,
        priceMin: priceMin ? parseInt(priceMin, 10) : undefined,
        priceMax: priceMax ? parseInt(priceMax, 10) : undefined,
      });
      setItems(rows);
    } catch (err) {
      console.error('Failed to load order items:', err);
      setItems([]);
    } finally {
      setLoading(false);
    }
  }, [getDb, searchDebounced, shopDomain, year, priceMin, priceMax]);

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
          columnCount={columnCount}
          onColumnCountChange={setColumnCount}
          onItemClick={() => {}}
        />
      )}
    </div>
  );
}

type OrderItemGridProps = {
  items: OrderItemRow[];
  columnCount: number;
  onColumnCountChange: (n: number) => void;
  onItemClick: (item: OrderItemRow) => void;
};

function OrderItemGrid({
  items,
  columnCount,
  onColumnCountChange,
  onItemClick,
}: OrderItemGridProps) {
  const scrollContainerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = scrollContainerRef.current;
    if (!el) return;
    const updateColumns = () => {
      const w = el.clientWidth;
      onColumnCountChange(
        Math.max(2, Math.min(4, Math.floor(w / CARD_MIN_WIDTH)))
      );
    };
    const observer = new ResizeObserver(updateColumns);
    observer.observe(el);
    updateColumns();
    return () => observer.disconnect();
  }, [onColumnCountChange]);

  const rowCount = Math.ceil(items.length / columnCount);
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scrollContainerRef.current,
    estimateSize: () => ROW_HEIGHT,
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
                display: 'grid',
                gridTemplateColumns: `repeat(${columnCount}, minmax(0, 1fr))`,
                gap: '1rem',
                padding: '0.5rem',
              }}
            >
              {rowItems.map((item) => (
                <OrderItemCard
                  key={item.id}
                  item={item}
                  onClick={() => onItemClick(item)}
                />
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}
