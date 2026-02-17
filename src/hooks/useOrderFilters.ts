import { useState, useCallback, useEffect } from 'react';
import { useDatabase } from '@/hooks/useDatabase';
import { getOrderItemFilterOptions } from '@/lib/orders-queries';
import { toastError, formatError } from '@/lib/toast';

/**
 * フィルタ状態の型定義
 *
 * すべて string 型: `<input>` / `<select>` に直接バインドし、
 * number への変換はクエリ実行時に `parseNumericFilter()` で行う
 */
export type OrdersFilterState = {
  shopDomain: string;
  year: string;
  priceMin: string;
  priceMax: string;
};

/** フィルタドロップダウンの選択肢 */
export type OrdersFilterOptions = {
  shopDomains: string[];
  years: number[];
};

const INITIAL_FILTERS: OrdersFilterState = {
  shopDomain: '',
  year: '',
  priceMin: '',
  priceMax: '',
};

const INITIAL_OPTIONS: OrdersFilterOptions = {
  shopDomains: [],
  years: [],
};

/**
 * Ordersスクリーンのフィルタ状態を管理するフック
 *
 * - 4つのフィルタ値（shopDomain, year, priceMin, priceMax）を単一オブジェクトで管理
 * - マウント時にDBからフィルタ選択肢（ショップ一覧、年一覧）をロード
 * - `setFilter('key', value)` で型安全に個別フィールドを更新
 * - `clearFilters()` で全フィルタを空文字にリセット
 */
export function useOrderFilters() {
  const { getDb } = useDatabase();
  const [filters, setFilters] = useState<OrdersFilterState>(INITIAL_FILTERS);
  const [filterOptions, setFilterOptions] =
    useState<OrdersFilterOptions>(INITIAL_OPTIONS);

  const setFilter = useCallback(
    <K extends keyof OrdersFilterState>(
      key: K,
      value: OrdersFilterState[K]
    ) => {
      setFilters((prev) => ({ ...prev, [key]: value }));
    },
    []
  );

  const clearFilters = useCallback(() => {
    setFilters(INITIAL_FILTERS);
  }, []);

  const loadFilterOptions = useCallback(async () => {
    try {
      const db = await getDb();
      const options = await getOrderItemFilterOptions(db);
      setFilterOptions(options);
    } catch (err) {
      toastError(
        `フィルタオプションの読み込みに失敗しました: ${formatError(err)}`
      );
      console.error('Failed to load filter options:', err);
    }
  }, [getDb]);

  useEffect(() => {
    loadFilterOptions();
  }, [loadFilterOptions]);

  return {
    filters,
    setFilter,
    clearFilters,
    filterOptions,
  };
}
