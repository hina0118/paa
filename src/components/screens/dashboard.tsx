import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useParse } from '@/contexts/use-parse';
import { useSync } from '@/contexts/use-sync';
import { formatDateTime } from '@/lib/utils';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../ui/card';
import { Button } from '../ui/button';

interface EmailStats {
  total_emails: number;
  with_body_plain: number;
  with_body_html: number;
  without_body: number;
  avg_plain_length: number;
  avg_html_length: number;
}

interface OrderStats {
  total_orders: number;
  total_items: number;
  total_amount: number;
}

interface DeliveryStats {
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

interface ProductMasterStats {
  product_master_count: number;
  distinct_items_with_normalized: number;
  items_with_parsed: number;
}

interface MiscStats {
  shop_settings_count: number;
  shop_settings_enabled_count: number;
  images_count: number;
}

// プログレスバーの最大値（文字数）
// テキスト形式: 一般的なメールの平均的な長さを基準に5000文字
// HTML形式: HTMLタグを含むため、テキストの約4倍の20000文字
const PROGRESS_MAX_PLAIN = 5000;
const PROGRESS_MAX_HTML = 20000;

export function Dashboard() {
  const [stats, setStats] = useState<EmailStats | null>(null);
  const [orderStats, setOrderStats] = useState<OrderStats | null>(null);
  const [deliveryStats, setDeliveryStats] = useState<DeliveryStats | null>(
    null
  );
  const [productMasterStats, setProductMasterStats] =
    useState<ProductMasterStats | null>(null);
  const [miscStats, setMiscStats] = useState<MiscStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { metadata: parseMetadata, refreshStatus: refreshParseStatus } =
    useParse();
  const { metadata: syncMetadata, refreshStatus: refreshSyncStatus } =
    useSync();

  const loadStats = async () => {
    try {
      setLoading(true);
      setError(null);
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
      setStats(emailResult);
      setOrderStats(orderResult);
      setDeliveryStats(deliveryResult);
      setProductMasterStats(productMasterResult);
      setMiscStats(miscResult);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      console.error('Failed to load dashboard stats:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadStats();
    refreshParseStatus();
    refreshSyncStatus();
  }, [refreshParseStatus, refreshSyncStatus]);

  const formatNumber = (num: number) => {
    return new Intl.NumberFormat('ja-JP').format(num);
  };

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 文字';
    return `${formatNumber(Math.round(bytes))} 文字`;
  };

  const formatCurrency = (amount: number) => {
    return `¥${formatNumber(amount)}`;
  };

  const calculatePercentage = (part: number, total: number) => {
    if (total === 0) return '0';
    return ((part / total) * 100).toFixed(1);
  };

  return (
    <div className="container mx-auto py-10 space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-3xl font-bold">ダッシュボード</h1>
        <Button onClick={loadStats} disabled={loading}>
          {loading ? '読み込み中...' : '更新'}
        </Button>
      </div>

      {error && (
        <Card className="border-red-500">
          <CardHeader>
            <CardTitle className="text-red-500">エラー</CardTitle>
          </CardHeader>
          <CardContent>
            <p>{error}</p>
          </CardContent>
        </Card>
      )}

      {stats && (
        <>
          {/* 注文・商品サマリ */}
          {orderStats && (
            <div className="grid gap-4 md:grid-cols-3">
              <Card>
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">注文数</CardTitle>
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth="2"
                    className="h-4 w-4 text-muted-foreground"
                  >
                    <path d="M6 2L3 6v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6l-3-4z" />
                    <line x1="3" y1="6" x2="21" y2="6" />
                  </svg>
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold">
                    {formatNumber(orderStats.total_orders)}
                  </div>
                  <p className="text-xs text-muted-foreground">
                    パース済み注文
                  </p>
                </CardContent>
              </Card>
              <Card>
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">商品数</CardTitle>
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth="2"
                    className="h-4 w-4 text-muted-foreground"
                  >
                    <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
                  </svg>
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold">
                    {formatNumber(orderStats.total_items)}
                  </div>
                  <p className="text-xs text-muted-foreground">
                    登録商品アイテム
                  </p>
                </CardContent>
              </Card>
              <Card>
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium">
                    合計金額
                  </CardTitle>
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth="2"
                    className="h-4 w-4 text-muted-foreground"
                  >
                    <line x1="12" y1="1" x2="12" y2="23" />
                    <path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />
                  </svg>
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold">
                    {formatCurrency(orderStats.total_amount)}
                  </div>
                  <p className="text-xs text-muted-foreground">
                    商品合計（税込想定）
                  </p>
                </CardContent>
              </Card>
            </div>
          )}

          {/* 配送状況 */}
          {deliveryStats && (
            <Card>
              <CardHeader>
                <CardTitle>配送状況</CardTitle>
                <CardDescription>
                  注文ごとの最新配送ステータス別件数
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-5">
                  {[
                    {
                      key: 'delivered' as const,
                      label: '配達済み',
                      count: deliveryStats.delivered,
                    },
                    {
                      key: 'shipped' as const,
                      label: '発送済み',
                      count: deliveryStats.shipped,
                    },
                    {
                      key: 'in_transit' as const,
                      label: '配送中',
                      count: deliveryStats.in_transit,
                    },
                    {
                      key: 'out_for_delivery' as const,
                      label: '配達中',
                      count: deliveryStats.out_for_delivery,
                    },
                    {
                      key: 'preparing' as const,
                      label: '準備中',
                      count: deliveryStats.preparing,
                    },
                    {
                      key: 'not_shipped' as const,
                      label: '未発送',
                      count: deliveryStats.not_shipped,
                    },
                    {
                      key: 'failed' as const,
                      label: '配送失敗',
                      count: deliveryStats.failed,
                    },
                    {
                      key: 'returned' as const,
                      label: '返品',
                      count: deliveryStats.returned,
                    },
                    {
                      key: 'cancelled' as const,
                      label: 'キャンセル',
                      count: deliveryStats.cancelled,
                    },
                  ].map(({ label, count }) => (
                    <div
                      key={label}
                      className="flex items-center justify-between rounded border px-3 py-2"
                    >
                      <span className="text-sm text-muted-foreground">
                        {label}
                      </span>
                      <span className="text-sm font-semibold">
                        {formatNumber(count)}
                      </span>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          )}

          {/* 商品名解析（AI）進捗 */}
          {productMasterStats && (
            <Card>
              <CardHeader>
                <CardTitle>商品名解析 (AI)</CardTitle>
                <CardDescription>
                  Gemini API による商品名からのメーカー情報抽出の進捗
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  <div>
                    <div className="flex items-center justify-between text-sm">
                      <span>解析済み / 対象</span>
                      <span className="font-semibold">
                        {formatNumber(productMasterStats.items_with_parsed)} /{' '}
                        {formatNumber(
                          productMasterStats.distinct_items_with_normalized
                        )}{' '}
                        件
                      </span>
                    </div>
                    <div className="mt-2 h-2 w-full bg-secondary rounded-full overflow-hidden">
                      <div
                        className="h-full bg-emerald-500 transition-all"
                        style={{
                          width: `${
                            productMasterStats.distinct_items_with_normalized >
                            0
                              ? Math.min(
                                  100,
                                  (productMasterStats.items_with_parsed /
                                    productMasterStats.distinct_items_with_normalized) *
                                    100
                                )
                              : 0
                          }%`,
                        }}
                      />
                    </div>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    product_master キャッシュ:{' '}
                    {formatNumber(productMasterStats.product_master_count)} 件
                  </p>
                </div>
              </CardContent>
            </Card>
          )}

          {/* 店舗設定・画像 */}
          {miscStats && (
            <div className="grid gap-4 md:grid-cols-2">
              <Card>
                <CardHeader>
                  <CardTitle>店舗設定</CardTitle>
                  <CardDescription>
                    メール取得対象の送信元アドレス設定
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="flex items-center gap-6">
                    <div>
                      <span className="text-2xl font-bold">
                        {formatNumber(miscStats.shop_settings_count)}
                      </span>
                      <span className="text-sm text-muted-foreground ml-1">
                        件
                      </span>
                      <p className="text-xs text-muted-foreground">登録済み</p>
                    </div>
                    <div>
                      <span className="text-2xl font-bold">
                        {formatNumber(miscStats.shop_settings_enabled_count)}
                      </span>
                      <span className="text-sm text-muted-foreground ml-1">
                        件
                      </span>
                      <p className="text-xs text-muted-foreground">有効</p>
                    </div>
                  </div>
                </CardContent>
              </Card>
              <Card>
                <CardHeader>
                  <CardTitle>商品画像</CardTitle>
                  <CardDescription>登録済み商品画像の数</CardDescription>
                </CardHeader>
                <CardContent>
                  <div>
                    <span className="text-2xl font-bold">
                      {formatNumber(miscStats.images_count)}
                    </span>
                    <span className="text-sm text-muted-foreground ml-1">
                      件
                    </span>
                    <p className="text-xs text-muted-foreground mt-1">
                      item_name_normalized 単位
                    </p>
                  </div>
                </CardContent>
              </Card>
            </div>
          )}

          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">
                  総メール数
                </CardTitle>
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth="2"
                  className="h-4 w-4 text-muted-foreground"
                >
                  <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
                </svg>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">
                  {formatNumber(stats.total_emails)}
                </div>
                <p className="text-xs text-muted-foreground">
                  取り込み済みメール
                </p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">
                  テキスト本文あり
                </CardTitle>
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth="2"
                  className="h-4 w-4 text-muted-foreground"
                >
                  <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
                  <circle cx="9" cy="7" r="4" />
                  <path d="M22 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75" />
                </svg>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">
                  {formatNumber(stats.with_body_plain)}
                </div>
                <p className="text-xs text-muted-foreground">
                  {calculatePercentage(
                    stats.with_body_plain,
                    stats.total_emails
                  )}
                  % のメール
                </p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">
                  HTML本文あり
                </CardTitle>
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth="2"
                  className="h-4 w-4 text-muted-foreground"
                >
                  <rect width="20" height="14" x="2" y="5" rx="2" />
                  <path d="M2 10h20" />
                </svg>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">
                  {formatNumber(stats.with_body_html)}
                </div>
                <p className="text-xs text-muted-foreground">
                  {calculatePercentage(
                    stats.with_body_html,
                    stats.total_emails
                  )}
                  % のメール
                </p>
              </CardContent>
            </Card>

            <Card className={stats.without_body > 0 ? 'border-amber-500' : ''}>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">本文なし</CardTitle>
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth="2"
                  className={`h-4 w-4 ${stats.without_body > 0 ? 'text-amber-500' : 'text-muted-foreground'}`}
                >
                  <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z" />
                  <path d="M12 9v4" />
                  <path d="M12 17h.01" />
                </svg>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">
                  {formatNumber(stats.without_body)}
                </div>
                <p className="text-xs text-muted-foreground">
                  {stats.without_body > 0 ? '要確認' : '問題なし'}
                </p>
              </CardContent>
            </Card>
          </div>

          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
            <Card>
              <CardHeader>
                <CardTitle>同期状況</CardTitle>
                <CardDescription>Gmail からのメール取得状態</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  <div>
                    <div className="flex items-center justify-between">
                      <span className="text-sm">ステータス</span>
                      <span
                        className={`px-2 py-1 rounded text-xs font-semibold ${
                          syncMetadata?.sync_status === 'syncing'
                            ? 'bg-blue-100 text-blue-800'
                            : syncMetadata?.sync_status === 'idle'
                              ? 'bg-green-100 text-green-800'
                              : syncMetadata?.sync_status === 'paused'
                                ? 'bg-yellow-100 text-yellow-800'
                                : syncMetadata?.sync_status === 'error'
                                  ? 'bg-red-100 text-red-800'
                                  : 'bg-gray-100 text-gray-800'
                        }`}
                      >
                        {syncMetadata?.sync_status === 'syncing'
                          ? '同期中'
                          : syncMetadata?.sync_status === 'idle'
                            ? '待機中'
                            : syncMetadata?.sync_status === 'paused'
                              ? '一時停止'
                              : syncMetadata?.sync_status === 'error'
                                ? 'エラー'
                                : '不明'}
                      </span>
                    </div>
                  </div>
                  <div>
                    <div className="flex items-center justify-between">
                      <span className="text-sm">総取得件数</span>
                      <span className="text-lg font-bold">
                        {formatNumber(syncMetadata?.total_synced_count ?? 0)}
                      </span>
                    </div>
                  </div>
                  {syncMetadata?.last_sync_completed_at && (
                    <p className="text-xs text-muted-foreground">
                      最終同期:{' '}
                      {formatDateTime(syncMetadata.last_sync_completed_at)}
                    </p>
                  )}
                  {syncMetadata?.last_error_message && (
                    <p className="text-xs text-red-600 dark:text-red-400">
                      エラー: {syncMetadata.last_error_message}
                    </p>
                  )}
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>平均本文長</CardTitle>
                <CardDescription>メール本文の平均文字数</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium">テキスト形式</span>
                    <span className="text-sm text-muted-foreground">
                      {formatBytes(stats.avg_plain_length)}
                    </span>
                  </div>
                  <div className="mt-2 h-2 w-full bg-secondary rounded-full overflow-hidden">
                    <div
                      className="h-full bg-blue-500 transition-all"
                      style={{
                        width: `${Math.min(100, (stats.avg_plain_length / PROGRESS_MAX_PLAIN) * 100)}%`,
                      }}
                    />
                  </div>
                </div>
                <div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium">HTML形式</span>
                    <span className="text-sm text-muted-foreground">
                      {formatBytes(stats.avg_html_length)}
                    </span>
                  </div>
                  <div className="mt-2 h-2 w-full bg-secondary rounded-full overflow-hidden">
                    <div
                      className="h-full bg-green-500 transition-all"
                      style={{
                        width: `${Math.min(100, (stats.avg_html_length / PROGRESS_MAX_HTML) * 100)}%`,
                      }}
                    />
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>データ品質</CardTitle>
                <CardDescription>
                  取り込まれたメールデータの状態
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <span className="text-sm">本文データ完全性</span>
                    <span className="text-sm font-bold">
                      {calculatePercentage(
                        stats.total_emails - stats.without_body,
                        stats.total_emails
                      )}
                      %
                    </span>
                  </div>
                  <div className="h-2 w-full bg-secondary rounded-full overflow-hidden">
                    <div
                      className={`h-full transition-all ${
                        stats.without_body === 0
                          ? 'bg-green-500'
                          : stats.without_body < stats.total_emails * 0.1
                            ? 'bg-amber-500'
                            : 'bg-red-500'
                      }`}
                      style={{
                        width: `${calculatePercentage(
                          stats.total_emails - stats.without_body,
                          stats.total_emails
                        )}%`,
                      }}
                    />
                  </div>
                  {stats.without_body > 0 && (
                    <p className="text-xs text-amber-600 dark:text-amber-400">
                      {formatNumber(stats.without_body)}{' '}
                      件のメールに本文データがありません。
                      再同期をお勧めします。
                    </p>
                  )}
                  {stats.without_body === 0 && (
                    <p className="text-xs text-green-600 dark:text-green-400">
                      全てのメールに本文データがあります。
                    </p>
                  )}
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>パース状況</CardTitle>
                <CardDescription>メールからの注文情報抽出状態</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  <div>
                    <div className="flex items-center justify-between">
                      <span className="text-sm">ステータス</span>
                      <span
                        className={`px-2 py-1 rounded text-xs font-semibold ${
                          parseMetadata?.parse_status === 'running'
                            ? 'bg-blue-100 text-blue-800'
                            : parseMetadata?.parse_status === 'completed'
                              ? 'bg-green-100 text-green-800'
                              : parseMetadata?.parse_status === 'error'
                                ? 'bg-red-100 text-red-800'
                                : 'bg-gray-100 text-gray-800'
                        }`}
                      >
                        {parseMetadata?.parse_status === 'running'
                          ? 'パース中'
                          : parseMetadata?.parse_status === 'completed'
                            ? '完了'
                            : parseMetadata?.parse_status === 'error'
                              ? 'エラー'
                              : '待機中'}
                      </span>
                    </div>
                  </div>
                  <div>
                    <div className="flex items-center justify-between">
                      <span className="text-sm">総パース件数</span>
                      <span className="text-lg font-bold">
                        {formatNumber(parseMetadata?.total_parsed_count || 0)}
                      </span>
                    </div>
                  </div>
                  {parseMetadata?.last_parse_completed_at && (
                    <p className="text-xs text-muted-foreground">
                      最終完了:{' '}
                      {formatDateTime(parseMetadata.last_parse_completed_at)}
                    </p>
                  )}
                  {parseMetadata?.last_error_message && (
                    <p className="text-xs text-red-600 dark:text-red-400">
                      エラー: {parseMetadata.last_error_message}
                    </p>
                  )}
                </div>
              </CardContent>
            </Card>
          </div>
        </>
      )}

      {!stats && !loading && !error && (
        <Card>
          <CardContent className="flex items-center justify-center py-10">
            <p className="text-muted-foreground">データを読み込んでいます...</p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
