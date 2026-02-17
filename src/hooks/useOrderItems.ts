import { useState, useCallback, useEffect, useRef } from 'react';
import { useDatabase } from '@/hooks/useDatabase';
import { loadOrderItems } from '@/lib/orders-queries';
import { parseNumericFilter } from '@/lib/utils';
import { toastError, formatError } from '@/lib/toast';
import type { OrderItemRow } from '@/lib/types';
import type { OrdersFilterState } from '@/hooks/useOrderFilters';

/** ソート状態の型定義 */
export type OrdersSortState = {
  sortBy: 'order_date' | 'price';
  sortOrder: 'asc' | 'desc';
};

const INITIAL_SORT: OrdersSortState = {
  sortBy: 'order_date',
  sortOrder: 'desc',
};

type UseOrderItemsParams = {
  searchDebounced: string;
  filters: OrdersFilterState;
};

/**
 * Ordersスクリーンのデータ取得・ソート・ドロワー状態を管理するフック
 *
 * ## 責務
 * - 商品データの取得（requestId パターンによる競合状態防止）
 * - ソート状態（sortBy, sortOrder）の管理
 * - 選択中アイテム・ドロワーの開閉状態の管理
 * - 画像更新後のデータ再読み込み + selectedItem の同期
 *
 * ## requestId パターン
 * `loadOrderItems()` は tauri-plugin-sql 経由の非同期IPC呼び出しのため
 * AbortController ではキャンセルできない。requestId を使って
 * 古いリクエストの結果が新しいリクエストの結果を上書きするのを防ぐ。
 */
export function useOrderItems({
  searchDebounced,
  filters,
}: UseOrderItemsParams) {
  const { getDb } = useDatabase();
  const [items, setItems] = useState<OrderItemRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [sort, setSort] = useState<OrdersSortState>(INITIAL_SORT);
  const [selectedItem, setSelectedItem] = useState<OrderItemRow | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const loadItemsRequestId = useRef(0);

  // フィルタを個別フィールドに展開（useEffect の依存配列で使用）
  const { shopDomain, year, priceMin, priceMax } = filters;

  const loadItems = useCallback(async (): Promise<
    OrderItemRow[] | undefined
  > => {
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
        sortBy: sort.sortBy,
        sortOrder: sort.sortOrder,
      });
      if (requestId === loadItemsRequestId.current) {
        setItems(rows);
        return rows;
      }
      return undefined;
    } catch (err) {
      if (requestId === loadItemsRequestId.current) {
        toastError(`商品一覧の読み込みに失敗しました: ${formatError(err)}`);
        console.error('Failed to load order items:', err);
        setItems([]);
      }
      return undefined;
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
    sort.sortBy,
    sort.sortOrder,
  ]);

  useEffect(() => {
    loadItems();
  }, [loadItems]);

  const openDrawer = useCallback((item: OrderItemRow) => {
    setSelectedItem(item);
    setDrawerOpen(true);
  }, []);

  const handleImageUpdated = useCallback(async () => {
    const newItems = await loadItems();
    if (newItems && selectedItem) {
      const updated = newItems.find((i) => i.id === selectedItem.id);
      if (updated) setSelectedItem(updated);
    }
  }, [loadItems, selectedItem]);

  return {
    items,
    loading,
    sort,
    setSort,
    selectedItem,
    drawerOpen,
    openDrawer,
    setDrawerOpen,
    handleImageUpdated,
  };
}
