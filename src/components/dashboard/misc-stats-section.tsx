import { formatNumber } from '@/lib/formatters';
import type { MiscStats } from '@/hooks/useDashboardStats';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../ui/card';

type Props = {
  miscStats: MiscStats;
};

export function ShopSettingsCard({ miscStats }: Props) {
  return (
    <Card className="relative overflow-hidden">
      <div className="absolute inset-x-0 top-0 h-1 bg-gradient-to-r from-violet-500 to-emerald-500" />
      <CardHeader className="pt-4">
        <CardTitle>店舗設定</CardTitle>
        <CardDescription>メール取得対象の送信元アドレス設定</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex items-baseline gap-2">
          <span className="text-2xl font-bold">
            {formatNumber(miscStats.shop_settings_enabled_count)} /{' '}
            {formatNumber(miscStats.shop_settings_count)}
          </span>
          <span className="text-muted-foreground">（有効 / 登録済み）</span>
        </div>
      </CardContent>
    </Card>
  );
}

export function ProductImagesCard({ miscStats }: Props) {
  const coveragePercent =
    miscStats.distinct_items_with_normalized > 0
      ? Math.min(
          100,
          Math.round(
            (miscStats.images_count /
              miscStats.distinct_items_with_normalized) *
              100
          )
        )
      : 0;

  return (
    <Card className="relative overflow-hidden">
      <div className="absolute inset-x-0 top-0 h-1 bg-gradient-to-r from-violet-500 to-emerald-500" />
      <CardHeader className="pt-4">
        <CardTitle>商品画像</CardTitle>
        <CardDescription>
          登録済み商品画像の数（item_name_normalized 単位）
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex items-baseline gap-2">
          <span className="text-2xl font-bold">
            {formatNumber(miscStats.images_count)} /{' '}
            {formatNumber(miscStats.distinct_items_with_normalized)}
          </span>
          <span className="text-muted-foreground">
            （{coveragePercent}% 網羅）
          </span>
        </div>
      </CardContent>
    </Card>
  );
}

export function MiscStatsSection({ miscStats }: Props) {
  return (
    <div className="grid gap-4 md:grid-cols-2">
      <ShopSettingsCard miscStats={miscStats} />
      <ProductImagesCard miscStats={miscStats} />
    </div>
  );
}
