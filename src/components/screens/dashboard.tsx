import { useEffect } from 'react';
import { LayoutDashboard } from 'lucide-react';
import { useParse } from '@/contexts/use-parse';
import { useSync } from '@/contexts/use-sync';
import { useDashboardStats } from '@/hooks/useDashboardStats';
import { Card, CardContent } from '../ui/card';
import { Button } from '../ui/button';
import { OrderStatsSection } from '../dashboard/order-stats-section';
import { DeliveryStatsSection } from '../dashboard/delivery-stats-section';
import { ProductMasterSection } from '../dashboard/product-master-section';
import { MiscStatsSection } from '../dashboard/misc-stats-section';
import { EmailStatsSection } from '../dashboard/email-stats-section';

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
  const { metadata: parseMetadata, refreshStatus: refreshParseStatus } =
    useParse();
  const { metadata: syncMetadata, refreshStatus: refreshSyncStatus } =
    useSync();

  useEffect(() => {
    loadStats();
    refreshParseStatus();
    refreshSyncStatus();
  }, [loadStats, refreshParseStatus, refreshSyncStatus]);

  return (
    <div className="container mx-auto py-10 px-6 space-y-6">
      <div className="mb-8 flex justify-between items-start">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-primary/10">
            <LayoutDashboard className="h-6 w-6 text-primary" />
          </div>
          <h1 className="text-3xl font-bold tracking-tight">ダッシュボード</h1>
        </div>
        <Button onClick={loadStats} disabled={loading}>
          {loading ? '読み込み中...' : '更新'}
        </Button>
      </div>

      {emailStats && (
        <>
          {orderStats && <OrderStatsSection orderStats={orderStats} />}
          {deliveryStats && (
            <DeliveryStatsSection deliveryStats={deliveryStats} />
          )}
          {productMasterStats && (
            <ProductMasterSection productMasterStats={productMasterStats} />
          )}
          {miscStats && <MiscStatsSection miscStats={miscStats} />}
          <EmailStatsSection
            emailStats={emailStats}
            syncMetadata={syncMetadata}
            parseMetadata={parseMetadata}
          />
        </>
      )}

      {!emailStats && !loading && (
        <Card>
          <CardContent className="flex items-center justify-center py-10">
            <p className="text-muted-foreground">
              {loadError
                ? 'データの読み込みに失敗しました。上の「更新」ボタンで再試行してください。'
                : 'データがありません。'}
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
