import { formatNumber } from '@/lib/formatters';
import type { ProductMasterStats } from '@/hooks/useDashboardStats';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../ui/card';

type Props = {
  productMasterStats: ProductMasterStats;
};

export function ProductMasterSection({ productMasterStats }: Props) {
  const progressPercent =
    productMasterStats.distinct_items_with_normalized > 0
      ? Math.min(
          100,
          (productMasterStats.items_with_parsed /
            productMasterStats.distinct_items_with_normalized) *
            100
        )
      : 0;

  return (
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
                style={{ width: `${progressPercent}%` }}
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
  );
}
