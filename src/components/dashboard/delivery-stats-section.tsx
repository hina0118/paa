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
  { key: 'not_shipped' as const, label: '未発送' },
];

export function DeliveryStatsSection({ deliveryStats }: Props) {
  return (
    <Card className="relative overflow-hidden">
      <div className="absolute inset-x-0 top-0 h-1 bg-gradient-to-r from-violet-500 to-emerald-500" />
      <CardHeader className="pt-4">
        <CardTitle>配送状況</CardTitle>
        <CardDescription>注文ごとの最新配送ステータス別件数</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-5">
            {DELIVERY_ITEMS.map(({ key, label }) => (
              <div
                key={key}
                className="flex items-center justify-between rounded border px-3 py-2"
              >
                <span className="text-sm text-muted-foreground">{label}</span>
                <span className="text-sm font-semibold">
                  {formatNumber(deliveryStats[key])}
                </span>
              </div>
            ))}
          </div>
          <div
            className={`flex items-center justify-between rounded border px-3 py-2 ${
              deliveryStats.not_shipped_over_1_year > 0
                ? 'border-amber-500/50 bg-amber-500/5'
                : ''
            }`}
          >
            <span className="text-sm text-muted-foreground">1年以上未発送</span>
            <span
              className={`text-sm font-semibold ${
                deliveryStats.not_shipped_over_1_year > 0
                  ? 'text-amber-600 dark:text-amber-400'
                  : ''
              }`}
            >
              {formatNumber(deliveryStats.not_shipped_over_1_year)} 件
            </span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
