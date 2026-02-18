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

export function MiscStatsSection({ miscStats }: Props) {
  return (
    <div className="grid gap-4 md:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle>店舗設定</CardTitle>
          <CardDescription>メール取得対象の送信元アドレス設定</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-6">
            <div>
              <span className="text-2xl font-bold">
                {formatNumber(miscStats.shop_settings_count)}
              </span>
              <span className="text-sm text-muted-foreground ml-1">件</span>
              <p className="text-xs text-muted-foreground">登録済み</p>
            </div>
            <div>
              <span className="text-2xl font-bold">
                {formatNumber(miscStats.shop_settings_enabled_count)}
              </span>
              <span className="text-sm text-muted-foreground ml-1">件</span>
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
            <span className="text-sm text-muted-foreground ml-1">件</span>
            <p className="text-xs text-muted-foreground mt-1">
              item_name_normalized 単位
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
