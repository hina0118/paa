import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { save, open, confirm } from '@tauri-apps/plugin-dialog';
import { Archive } from 'lucide-react';
import {
  toastSuccess,
  toastError,
  toastWarning,
  formatError,
} from '@/lib/toast';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';

interface ExportResult {
  images_count: number;
  shop_settings_count: number;
  product_master_count: number;
  emails_count: number;
  item_overrides_count: number;
  order_overrides_count: number;
  excluded_items_count: number;
  excluded_orders_count: number;
  image_files_count: number;
  images_skipped: number;
}

interface ImportResult {
  images_inserted: number;
  shop_settings_inserted: number;
  product_master_inserted: number;
  emails_inserted: number;
  item_overrides_inserted: number;
  order_overrides_inserted: number;
  excluded_items_inserted: number;
  excluded_orders_inserted: number;
  image_files_copied: number;
}

/**
 * エクスポート/インポート結果から合計件数と詳細メッセージを生成する
 * @param items - 結果のラベルと件数のタプル配列
 * @returns 合計件数と詳細文字列を含むオブジェクト
 */
function formatBackupResult(items: Array<[string, number]>): {
  total: number;
  details: string;
} {
  const total = items.reduce((sum, [, count]) => sum + count, 0);
  const details = items
    .map(([label, count]) => `${label}: ${count}件`)
    .join('、');
  return { total, details };
}

export function Backup() {
  const [isExporting, setIsExporting] = useState(false);
  const [isImporting, setIsImporting] = useState(false);

  const handleExport = async () => {
    setIsExporting(true);
    try {
      const now = new Date();
      const defaultName = `paa_export_${now.getFullYear()}${String(now.getMonth() + 1).padStart(2, '0')}${String(now.getDate()).padStart(2, '0')}_${String(now.getHours()).padStart(2, '0')}${String(now.getMinutes()).padStart(2, '0')}${String(now.getSeconds()).padStart(2, '0')}.zip`;
      const savePath = await save({
        defaultPath: defaultName,
        filters: [{ name: 'ZIP', extensions: ['zip'] }],
      });
      if (!savePath) {
        return;
      }
      const result = await invoke<ExportResult>('export_metadata', {
        savePath,
      });
      const { total, details } = formatBackupResult([
        ['images', result.images_count],
        ['shop_settings', result.shop_settings_count],
        ['product_master', result.product_master_count],
        ['emails', result.emails_count],
        ['item_overrides', result.item_overrides_count],
        ['order_overrides', result.order_overrides_count],
        ['excluded_items', result.excluded_items_count],
        ['excluded_orders', result.excluded_orders_count],
      ]);
      toastSuccess(
        `バックアップを保存しました（${totalRecords}件のレコード、${result.image_files_count}個の画像ファイル）`
        details
      );
      if (result.images_skipped > 0) {
        toastWarning(
          `${result.images_skipped}件の画像をスキップしました（不正なファイル名、サイズ超過、またはファイルが存在しません）`
        );
      }
    } catch (error) {
      toastError(`エクスポートに失敗しました: ${formatError(error)}`);
    } finally {
      setIsExporting(false);
    }
  };

  const handleImport = async () => {
    const confirmed = await confirm(
      'データを復元します。既存のデータと競合する場合は既存データが維持されます。続行しますか？',
      { title: 'データの復元', kind: 'warning' }
    );
    if (!confirmed) {
      return;
    }
    setIsImporting(true);
    try {
      const zipPath = await open({
        multiple: false,
        directory: false,
        filters: [{ name: 'ZIP', extensions: ['zip'] }],
      });
      if (!zipPath || typeof zipPath !== 'string') {
        return;
      }
      const result = await invoke<ImportResult>('import_metadata', {
        zipPath,
      });
      const { total, details } = formatBackupResult([
        ['images', result.images_inserted],
        ['shop_settings', result.shop_settings_inserted],
        ['product_master', result.product_master_inserted],
        ['emails', result.emails_inserted],
        ['item_overrides', result.item_overrides_inserted],
        ['order_overrides', result.order_overrides_inserted],
        ['excluded_items', result.excluded_items_inserted],
        ['excluded_orders', result.excluded_orders_inserted],
      ]);
      toastSuccess(
        `復元しました（${total}件のレコード、${result.image_files_copied}個の画像ファイル）`,
        details
      );
    } catch (error) {
      toastError(`インポートに失敗しました: ${formatError(error)}`);
    } finally {
      setIsImporting(false);
    }
  };

  return (
    <div className="container mx-auto py-10 px-6 space-y-6">
      <div className="mb-8 space-y-2">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-primary/10">
            <Archive className="h-6 w-6 text-primary" />
          </div>
          <h1 className="text-3xl font-bold tracking-tight">
            データのバックアップ
          </h1>
        </div>
        <p className="text-muted-foreground">
          AI解析済みの商品データや店舗設定、取得済みメールなどのメタデータと画像ファイル（キャッシュ）をバックアップ・復元します。DBをリセットしても重要なデータを維持できます。
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>データのバックアップ</CardTitle>
          <CardDescription>
            メタデータと画像ファイルをZIP形式でエクスポートします。保存先を選択してください。
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Button
            onClick={handleExport}
            disabled={isExporting}
            aria-label="データのバックアップ"
          >
            {isExporting ? 'エクスポート中...' : 'データのバックアップ'}
          </Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>データの復元</CardTitle>
          <CardDescription>
            バックアップZIPからメタデータと画像をインポートします。既存データと競合する場合は既存データが維持されます（INSERT
            OR IGNORE）。
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Button
            onClick={handleImport}
            disabled={isImporting}
            variant="secondary"
            aria-label="データの復元"
          >
            {isImporting ? 'インポート中...' : 'データの復元'}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
