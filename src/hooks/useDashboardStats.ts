import { useState, useCallback } from 'react';
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
}

export type UseDashboardStatsResult = {
  emailStats: EmailStats | null;
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
 */
export function useDashboardStats(): UseDashboardStatsResult {
  const [emailStats, setEmailStats] = useState<EmailStats | null>(null);
  const [orderStats, setOrderStats] = useState<OrderStats | null>(null);
  const [deliveryStats, setDeliveryStats] = useState<DeliveryStats | null>(
    null
  );
  const [productMasterStats, setProductMasterStats] =
    useState<ProductMasterStats | null>(null);
  const [miscStats, setMiscStats] = useState<MiscStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState(false);

  const loadStats = useCallback(async () => {
    try {
      setLoading(true);
      setLoadError(false);
      const [
        emailResult,
        orderResult,
        deliveryResult,
        productMasterResult,
        miscResult,
      ] = await Promise.all([
        invoke<EmailStats>('get_email_stats'),
        invoke<OrderStats>('get_order_stats'),
        invoke<DeliveryStats>('get_delivery_stats'),
        invoke<ProductMasterStats>('get_product_master_stats'),
        invoke<MiscStats>('get_misc_stats'),
      ]);
      setEmailStats(emailResult);
      setOrderStats(orderResult);
      setDeliveryStats(deliveryResult);
      setProductMasterStats(productMasterResult);
      setMiscStats(miscResult);
    } catch (err) {
      setLoadError(true);
      toastError(`統計の読み込みに失敗しました: ${formatError(err)}`);
      console.error('Failed to load dashboard stats:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    emailStats,
    orderStats,
    deliveryStats,
    productMasterStats,
    miscStats,
    loading,
    loadError,
    loadStats,
  };
}
