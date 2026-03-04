import { useEffect } from 'react';
import { LayoutDashboard } from 'lucide-react';
import { useDashboardStats } from '@/hooks/useDashboardStats';
import { Card, CardContent } from '../ui/card';
import { Button } from '../ui/button';
import { PageHeader } from '../ui/page-header';
import { Skeleton } from '../ui/skeleton';
import { OrderStatsSection } from '../dashboard/order-stats-section';
import { DeliveryStatsSection } from '../dashboard/delivery-stats-section';
import { ProductMasterSection } from '../dashboard/product-master-section';
import {
  ShopSettingsCard,
  ProductImagesCard,
} from '../dashboard/misc-stats-section';

export function Dashboard() {
  const {
    emailStats,
    orderStats,
    deliveryStats,
    productMasterStats,
    miscStats,
    loading,
    loadError,
    loadStats,
  } = useDashboardStats();

  useEffect(() => {
    loadStats();
  }, [loadStats]);

  return (
    <div className="container mx-auto pt-0 pb-10 px-6 space-y-6">
      <PageHeader title="ダッシュボード" icon={LayoutDashboard}>
        <Button onClick={loadStats} disabled={loading}>
          {loading ? '読み込み中...' : '更新'}
        </Button>
      </PageHeader>

      {emailStats && (
        <>
          {orderStats && <OrderStatsSection orderStats={orderStats} />}
          {deliveryStats && (
            <DeliveryStatsSection deliveryStats={deliveryStats} />
          )}
          <div className="grid gap-4 md:grid-cols-3">
            {productMasterStats && (
              <ProductMasterSection productMasterStats={productMasterStats} />
            )}
            {miscStats && <ShopSettingsCard miscStats={miscStats} />}
            {miscStats && <ProductImagesCard miscStats={miscStats} />}
          </div>
        </>
      )}

      {!emailStats && !loading && (
        <Card>
          <CardContent className="flex items-center justify-center py-10">
            <p className="text-muted-foreground">
              {loadError
                ? 'データの読み込みに失敗しました。上の「更新」ボタンで再試行してください。'
                : 'データがありません。上の「更新」ボタンで読み込んでください。'}
            </p>
          </CardContent>
        </Card>
      )}

      {loading && !emailStats && (
        <div className="space-y-4">
          <div className="grid gap-4 md:grid-cols-3">
            {[...Array(3)].map((_, i) => (
              <Card key={i}>
                <div className="p-6 space-y-3">
                  <Skeleton className="h-4 w-24" />
                  <Skeleton className="h-8 w-32" />
                  <Skeleton className="h-3 w-20" />
                </div>
              </Card>
            ))}
          </div>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
            {[...Array(4)].map((_, i) => (
              <Card key={i}>
                <div className="p-6 space-y-3">
                  <Skeleton className="h-4 w-20" />
                  <Skeleton className="h-6 w-16" />
                  <Skeleton className="h-3 w-24" />
                </div>
              </Card>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
