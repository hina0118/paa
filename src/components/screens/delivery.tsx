import { useCallback, useEffect, useState, type ReactNode } from 'react';
import { toastError, formatError } from '@/lib/toast';
import {
  Truck,
  ExternalLink,
  RefreshCw,
  Package,
  Clock,
  CheckCircle2,
} from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { Button } from '@/components/ui/button';
import { PageHeader } from '@/components/ui/page-header';
import { DatabaseManager } from '@/lib/database';
import type { DeliveryStatus } from '@/lib/types';
import { buildTrackingUrl } from './delivery-utils';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type DeliveryRow = {
  id: number;
  orderId: number;
  trackingNumber: string | null;
  carrier: string | null;
  deliveryStatus: DeliveryStatus;
  estimatedDelivery: string | null;
  actualDelivery: string | null;
  lastCheckedAt: string | null;
  orderNumber: string | null;
  shopDomain: string | null;
  orderDate: string | null;
};

type DbRow = {
  id: number;
  order_id: number;
  tracking_number: string | null;
  carrier: string | null;
  delivery_status: string;
  estimated_delivery: string | null;
  actual_delivery: string | null;
  last_checked_at: string | null;
  order_number: string | null;
  shop_domain: string | null;
  order_date: string | null;
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DELIVERY_STATUS_LABELS: Record<
  string,
  { label: string; className: string }
> = {
  not_shipped: {
    label: '未発送',
    className: 'bg-muted text-muted-foreground border border-border',
  },
  preparing: {
    label: '準備中',
    className:
      'bg-amber-500/10 text-amber-700 dark:text-amber-400 border border-amber-500/20',
  },
  shipped: {
    label: '発送済み',
    className:
      'bg-blue-500/10 text-blue-700 dark:text-blue-400 border border-blue-500/20',
  },
  in_transit: {
    label: '輸送中',
    className: 'bg-primary/10 text-primary border border-primary/20',
  },
  out_for_delivery: {
    label: '配達中',
    className:
      'bg-orange-500/10 text-orange-700 dark:text-orange-400 border border-orange-500/20',
  },
  delivered: {
    label: '配達完了',
    className:
      'bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 border border-emerald-500/20',
  },
  failed: {
    label: '配達失敗',
    className:
      'bg-destructive/10 text-destructive border border-destructive/20',
  },
  returned: {
    label: '返送',
    className: 'bg-muted text-muted-foreground border border-border',
  },
  cancelled: {
    label: 'キャンセル',
    className: 'bg-muted text-muted-foreground border border-border',
  },
};

const ALL_FILTER = 'all';

// ---------------------------------------------------------------------------
// DB query
// ---------------------------------------------------------------------------

async function fetchDeliveries(): Promise<DeliveryRow[]> {
  const db = await DatabaseManager.getInstance().getDatabase();
  const rows = await db.select<DbRow[]>(`
    SELECT
      d.id,
      d.order_id,
      d.tracking_number,
      d.carrier,
      d.delivery_status,
      d.estimated_delivery,
      d.actual_delivery,
      d.last_checked_at,
      o.order_number,
      o.shop_domain,
      o.order_date
    FROM deliveries d
    LEFT JOIN orders o ON d.order_id = o.id
    ORDER BY d.updated_at DESC
  `);
  return rows.map((r) => ({
    id: r.id,
    orderId: r.order_id,
    trackingNumber: r.tracking_number,
    carrier: r.carrier,
    deliveryStatus: r.delivery_status as DeliveryStatus,
    estimatedDelivery: r.estimated_delivery,
    actualDelivery: r.actual_delivery,
    lastCheckedAt: r.last_checked_at,
    orderNumber: r.order_number,
    shopDomain: r.shop_domain,
    orderDate: r.order_date,
  }));
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function StatusBadge({ status }: { status: string }) {
  const config = DELIVERY_STATUS_LABELS[status] ?? {
    label: status,
    className: 'bg-muted text-muted-foreground border border-border',
  };
  return (
    <span
      className={`px-2 py-0.5 rounded-md text-xs font-medium ${config.className}`}
    >
      {config.label}
    </span>
  );
}

type SummaryCardProps = {
  label: string;
  count: number;
  icon: ReactNode;
  colorClass: string;
};

function SummaryCard({ label, count, icon, colorClass }: SummaryCardProps) {
  return (
    <div className="rounded-lg border bg-card p-4 flex items-center gap-3">
      <div
        className={`h-10 w-10 rounded-full flex items-center justify-center ${colorClass}`}
      >
        {icon}
      </div>
      <div>
        <p className="text-xs text-muted-foreground">{label}</p>
        <p className="text-2xl font-bold">{count}</p>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main screen
// ---------------------------------------------------------------------------

const PENDING_STATUSES: DeliveryStatus[] = [
  'not_shipped',
  'preparing',
  'shipped',
  'in_transit',
  'out_for_delivery',
];

export function Delivery() {
  const [rows, setRows] = useState<DeliveryRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [carrierFilter, setCarrierFilter] = useState<string>(ALL_FILTER);
  const [statusFilter, setStatusFilter] = useState<string>(ALL_FILTER);
  const [openingId, setOpeningId] = useState<number | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await fetchDeliveries();
      setRows(data);
    } catch (error) {
      console.error('Failed to load deliveries:', error);
      toastError(`配送情報の読み込みに失敗しました: ${formatError(error)}`);
      setRows([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  // --- Derived values ---

  const carriers = [
    ALL_FILTER,
    ...Array.from(new Set(rows.map((r) => r.carrier ?? '不明'))),
  ];
  const statuses = [
    ALL_FILTER,
    ...Array.from(new Set(rows.map((r) => r.deliveryStatus))),
  ];

  const filtered = rows.filter((r) => {
    if (carrierFilter !== ALL_FILTER && (r.carrier ?? '不明') !== carrierFilter)
      return false;
    if (statusFilter !== ALL_FILTER && r.deliveryStatus !== statusFilter)
      return false;
    return true;
  });

  const pendingCount = rows.filter((r) =>
    PENDING_STATUSES.includes(r.deliveryStatus)
  ).length;
  const deliveredCount = rows.filter(
    (r) => r.deliveryStatus === 'delivered'
  ).length;
  const totalCount = rows.length;

  // --- Actions ---

  const handleOpenTracking = async (row: DeliveryRow) => {
    const url = buildTrackingUrl(row.carrier, row.trackingNumber);
    if (!url) return;
    setOpeningId(row.id);
    try {
      await openUrl(url);
    } finally {
      setOpeningId(null);
    }
  };

  // --- Render ---

  return (
    <div className="container mx-auto h-full flex flex-col px-6">
      <PageHeader
        title="配送状況"
        description={
          loading ? '読み込み中...' : `${totalCount}件の配送レコード`
        }
        icon={Truck}
      />

      {/* サマリカード */}
      <div className="grid grid-cols-3 gap-4 mb-6 flex-shrink-0">
        <SummaryCard
          label="追跡対象 (未着)"
          count={pendingCount}
          icon={<Clock className="h-5 w-5 text-blue-600 dark:text-blue-400" />}
          colorClass="bg-blue-500/10"
        />
        <SummaryCard
          label="配達完了"
          count={deliveredCount}
          icon={
            <CheckCircle2 className="h-5 w-5 text-emerald-600 dark:text-emerald-400" />
          }
          colorClass="bg-emerald-500/10"
        />
        <SummaryCard
          label="合計"
          count={totalCount}
          icon={<Package className="h-5 w-5 text-muted-foreground" />}
          colorClass="bg-muted"
        />
      </div>

      {/* フィルタ */}
      <div className="flex flex-wrap gap-3 mb-4 flex-shrink-0">
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground font-medium">
            配送業者:
          </span>
          <div className="flex gap-1">
            {carriers.map((c) => (
              <button
                key={c}
                onClick={() => setCarrierFilter(c)}
                className={`px-3 py-1 rounded-md text-xs font-medium transition-colors ${
                  carrierFilter === c
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-muted text-muted-foreground hover:bg-muted/80'
                }`}
              >
                {c === ALL_FILTER ? 'すべて' : c}
              </button>
            ))}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground font-medium">
            ステータス:
          </span>
          <div className="flex flex-wrap gap-1">
            {statuses.map((s) => (
              <button
                key={s}
                onClick={() => setStatusFilter(s)}
                className={`px-3 py-1 rounded-md text-xs font-medium transition-colors ${
                  statusFilter === s
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-muted text-muted-foreground hover:bg-muted/80'
                }`}
              >
                {s === ALL_FILTER
                  ? 'すべて'
                  : (DELIVERY_STATUS_LABELS[s]?.label ?? s)}
              </button>
            ))}
          </div>
        </div>
        <div className="ml-auto">
          <Button variant="outline" size="sm" onClick={load} disabled={loading}>
            <RefreshCw
              className={`h-4 w-4 mr-1.5 ${loading ? 'animate-spin' : ''}`}
            />
            更新
          </Button>
        </div>
      </div>

      {/* テーブル */}
      <div className="flex-1 overflow-auto rounded-lg border">
        <table className="w-full text-sm">
          <thead className="sticky top-0 bg-muted/80 backdrop-blur-sm border-b z-10">
            <tr>
              <th className="text-left px-4 py-3 font-medium text-muted-foreground whitespace-nowrap">
                追跡番号
              </th>
              <th className="text-left px-4 py-3 font-medium text-muted-foreground whitespace-nowrap">
                配送業者
              </th>
              <th className="text-left px-4 py-3 font-medium text-muted-foreground whitespace-nowrap">
                ステータス
              </th>
              <th className="text-left px-4 py-3 font-medium text-muted-foreground whitespace-nowrap">
                注文番号
              </th>
              <th className="text-left px-4 py-3 font-medium text-muted-foreground whitespace-nowrap">
                ショップ
              </th>
              <th className="text-left px-4 py-3 font-medium text-muted-foreground whitespace-nowrap">
                注文日
              </th>
              <th className="text-left px-4 py-3 font-medium text-muted-foreground whitespace-nowrap">
                最終確認
              </th>
              <th className="px-4 py-3" />
            </tr>
          </thead>
          <tbody>
            {loading ? (
              <tr>
                <td
                  colSpan={8}
                  className="text-center py-12 text-muted-foreground"
                >
                  読み込み中...
                </td>
              </tr>
            ) : filtered.length === 0 ? (
              <tr>
                <td
                  colSpan={8}
                  className="text-center py-12 text-muted-foreground"
                >
                  該当するレコードがありません
                </td>
              </tr>
            ) : (
              filtered.map((row) => {
                const trackingUrl = buildTrackingUrl(
                  row.carrier,
                  row.trackingNumber
                );
                const isOpening = openingId === row.id;
                return (
                  <tr
                    key={row.id}
                    className="border-b last:border-0 hover:bg-muted/30 transition-colors"
                  >
                    <td className="px-4 py-3 font-mono text-xs">
                      {row.trackingNumber ?? (
                        <span className="text-muted-foreground">—</span>
                      )}
                    </td>
                    <td className="px-4 py-3 whitespace-nowrap">
                      {row.carrier ?? (
                        <span className="text-muted-foreground">不明</span>
                      )}
                    </td>
                    <td className="px-4 py-3">
                      <StatusBadge status={row.deliveryStatus} />
                    </td>
                    <td className="px-4 py-3 text-xs">
                      {row.orderNumber ?? (
                        <span className="text-muted-foreground">—</span>
                      )}
                    </td>
                    <td className="px-4 py-3 text-xs text-muted-foreground">
                      {row.shopDomain ?? '—'}
                    </td>
                    <td className="px-4 py-3 text-xs text-muted-foreground whitespace-nowrap">
                      {row.orderDate ? row.orderDate.slice(0, 10) : '—'}
                    </td>
                    <td className="px-4 py-3 text-xs text-muted-foreground whitespace-nowrap">
                      {row.lastCheckedAt ? row.lastCheckedAt.slice(0, 10) : '—'}
                    </td>
                    <td className="px-4 py-3">
                      {trackingUrl ? (
                        <Button
                          variant="outline"
                          size="sm"
                          disabled={isOpening}
                          onClick={() => handleOpenTracking(row)}
                          className="whitespace-nowrap"
                        >
                          <ExternalLink className="h-3.5 w-3.5 mr-1.5" />
                          {isOpening ? '開いています...' : '追跡ページ'}
                        </Button>
                      ) : (
                        <span className="text-xs text-muted-foreground">
                          URL不明
                        </span>
                      )}
                    </td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>

      {/* フッター件数 */}
      {!loading && (
        <p className="mt-2 text-xs text-muted-foreground flex-shrink-0">
          {filtered.length} / {totalCount} 件表示
        </p>
      )}
    </div>
  );
}
