import { useState, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toastError, formatError } from '@/lib/toast';

export interface EmailStats {
  total_emails: number;
  with_body_plain: number;
  with_body_html: number;
  without_body: number;
  avg_plain_length: number;
  avg_html_length: number;
}

export interface OrderStats {
  total_orders: number;
  total_items: number;
  distinct_items_with_normalized: number;
  total_amount: number;
}

export interface DeliveryStats {
  not_shipped: number;
  preparing: number;
  shipped: number;
  in_transit: number;
  out_for_delivery: number;
  delivered: number;
  failed: number;
  returned: number;
  cancelled: number;
  not_shipped_over_1_year: number;
}

export interface ProductMasterStats {
  product_master_count: number;
  distinct_items_with_normalized: number;
  items_with_parsed: number;
}

export interface MiscStats {
  shop_settings_count: number;
  shop_settings_enabled_count: number;
  images_count: number;
  distinct_items_with_normalized: number;
}

export type UseDashboardStatsResult = {
  orderStats: OrderStats | null;
  deliveryStats: DeliveryStats | null;
  productMasterStats: ProductMasterStats | null;
  miscStats: MiscStats | null;
  loading: boolean;
  loadError: boolean;
  loadStats: () => Promise<void>;
};

/**
 * Dashboardスクリーンの統計データフェッチを管理するフック
 *
 * ## 責務
 * - 5種類の統計データの並列フェッチ（Promise.all）
 * - loading / loadError 状態の管理
 * - エラー時のトースト通知
 *
 * ## requestId パターン
 * `loadStats()` は tauri invoke 経由の非同期IPC呼び出しのため
 * AbortController ではキャンセルできない。requestId を使って
 * 古いリクエストの結果が新しいリクエストの結果を上書きするのを防ぐ。
 */
export function useDashboardStats(): UseDashboardStatsResult {
  const [orderStats, setOrderStats] = useState<OrderStats | null>(null);
  const [deliveryStats, setDeliveryStats] = useState<DeliveryStats | null>(
    null
  );
  const [productMasterStats, setProductMasterStats] =
    useState<ProductMasterStats | null>(null);
  const [miscStats, setMiscStats] = useState<MiscStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState(false);
  const loadStatsRequestId = useRef(0);

  const loadStats = useCallback(async () => {
    const requestId = (loadStatsRequestId.current += 1);
    try {
      setLoading(true);
      setLoadError(false);
      const [orderResult, deliveryResult, productMasterResult, miscResult] =
        await Promise.all([
          invoke<OrderStats>('get_order_stats'),
          invoke<DeliveryStats>('get_delivery_stats'),
          invoke<ProductMasterStats>('get_product_master_stats'),
          invoke<MiscStats>('get_misc_stats'),
        ]);
      if (requestId === loadStatsRequestId.current) {
        setOrderStats(orderResult);
        setDeliveryStats(deliveryResult);
        setProductMasterStats(productMasterResult);
        setMiscStats(miscResult);
      }
    } catch (err) {
      if (requestId === loadStatsRequestId.current) {
        setLoadError(true);
        toastError(`統計の読み込みに失敗しました: ${formatError(err)}`);
        console.error('Failed to load dashboard stats:', err);
      }
    } finally {
      if (requestId === loadStatsRequestId.current) {
        setLoading(false);
      }
    }
  }, []);

  return {
    orderStats,
    deliveryStats,
    productMasterStats,
    miscStats,
    loading,
    loadError,
    loadStats,
  };
}
