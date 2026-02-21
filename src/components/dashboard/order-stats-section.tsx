import { ShoppingBag, Package, CircleDollarSign } from 'lucide-react';
import { formatNumber, formatCurrency } from '@/lib/formatters';
import type { OrderStats } from '@/hooks/useDashboardStats';
import { Card, CardContent, CardHeader, CardTitle } from '../ui/card';

type Props = {
  orderStats: OrderStats;
};

export function OrderStatsSection({ orderStats }: Props) {
  return (
    <div className="grid gap-4 md:grid-cols-3">
      <Card className="relative overflow-hidden">
        <div className="absolute inset-x-0 top-0 h-1 bg-gradient-to-r from-primary to-violet-500" />
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2 pt-4">
          <CardTitle className="text-sm font-medium">注文数</CardTitle>
          <div className="rounded-lg bg-primary/10 p-2">
            <ShoppingBag className="h-4 w-4 text-primary" />
          </div>
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold tracking-tight">
            {formatNumber(orderStats.total_orders)}
          </div>
          <p className="text-xs text-muted-foreground mt-1">パース済み注文</p>
        </CardContent>
      </Card>
      <Card className="relative overflow-hidden">
        <div className="absolute inset-x-0 top-0 h-1 bg-gradient-to-r from-violet-500 to-emerald-500" />
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2 pt-4">
          <CardTitle className="text-sm font-medium">商品数</CardTitle>
          <div className="rounded-lg bg-violet-500/10 p-2">
            <Package className="h-4 w-4 text-violet-500" />
          </div>
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold tracking-tight">
            {formatNumber(orderStats.total_items)}
          </div>
          <p className="text-xs text-muted-foreground mt-1">登録商品アイテム</p>
        </CardContent>
      </Card>
      <Card className="relative overflow-hidden">
        <div className="absolute inset-x-0 top-0 h-1 bg-gradient-to-r from-emerald-500 to-cyan-500" />
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2 pt-4">
          <CardTitle className="text-sm font-medium">合計金額</CardTitle>
          <div className="rounded-lg bg-emerald-500/10 p-2">
            <CircleDollarSign className="h-4 w-4 text-emerald-600" />
          </div>
        </CardHeader>
        <CardContent>
          <div className="text-3xl font-bold tracking-tight">
            {formatCurrency(orderStats.total_amount)}
          </div>
          <p className="text-xs text-muted-foreground mt-1">商品合計（税込想定）</p>
        </CardContent>
      </Card>
    </div>
  );
}
