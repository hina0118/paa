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
  const total = productMasterStats.distinct_items_with_normalized;
  const parsed = productMasterStats.items_with_parsed;
  const coveragePercent =
    total > 0 ? Math.min(100, Math.round((parsed / total) * 100)) : 0;

  return (
    <Card className="relative overflow-hidden">
      <div className="absolute inset-x-0 top-0 h-1 bg-gradient-to-r from-violet-500 to-emerald-500" />
      <CardHeader className="pt-4">
        <CardTitle>商品名解析</CardTitle>
        <CardDescription>
          Gemini API による商品名からのメーカー情報抽出の進捗
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex items-baseline gap-2">
          <span className="text-2xl font-bold">
            {formatNumber(parsed)} / {formatNumber(total)}
          </span>
          <span className="text-muted-foreground">
            （{coveragePercent}% 網羅）
          </span>
        </div>
      </CardContent>
    </Card>
  );
}
