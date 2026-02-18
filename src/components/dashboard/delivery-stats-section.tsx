import { formatNumber } from '@/lib/formatters';
import type { DeliveryStats } from '@/hooks/useDashboardStats';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../ui/card';

type Props = {
  deliveryStats: DeliveryStats;
};

const DELIVERY_ITEMS = [
  { key: 'delivered' as const, label: '配達済み' },
  { key: 'shipped' as const, label: '発送済み' },
  { key: 'in_transit' as const, label: '配送中' },
  { key: 'out_for_delivery' as const, label: '配達中' },
  { key: 'preparing' as const, label: '準備中' },
  { key: 'not_shipped' as const, label: '未発送' },
  { key: 'failed' as const, label: '配送失敗' },
  { key: 'returned' as const, label: '返品' },
  { key: 'cancelled' as const, label: 'キャンセル' },
];

export function DeliveryStatsSection({ deliveryStats }: Props) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>配送状況</CardTitle>
        <CardDescription>注文ごとの最新配送ステータス別件数</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-5">
          {DELIVERY_ITEMS.map(({ key, label }) => (
            <div
              key={label}
              className="flex items-center justify-between rounded border px-3 py-2"
            >
              <span className="text-sm text-muted-foreground">{label}</span>
              <span className="text-sm font-semibold">
                {formatNumber(deliveryStats[key])}
              </span>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
