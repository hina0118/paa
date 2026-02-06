import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { save, open, confirm } from '@tauri-apps/plugin-dialog';
import { Archive } from 'lucide-react';
import { toast } from 'sonner';
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
  image_files_count: number;
}

interface ImportResult {
  images_inserted: number;
  shop_settings_inserted: number;
  product_master_inserted: number;
  image_files_copied: number;
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
      toast.success(
        `バックアップを保存しました（images: ${result.images_count}、shop_settings: ${result.shop_settings_count}、product_master: ${result.product_master_count}、画像ファイル: ${result.image_files_count}）`
      );
    } catch (error) {
      toast.error(
        `エクスポートに失敗しました: ${error instanceof Error ? error.message : String(error)}`
      );
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
      toast.success(
        `復元しました（images: ${result.images_inserted}件、shop_settings: ${result.shop_settings_inserted}件、product_master: ${result.product_master_inserted}件、画像ファイル: ${result.image_files_copied}件）`
      );
    } catch (error) {
      toast.error(
        `インポートに失敗しました: ${error instanceof Error ? error.message : String(error)}`
      );
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
          images、shop_settings、product_master
          のメタデータと画像ファイルをバックアップ・復元します。DBをリセットしてもAI解析済みの商品データや画像キャッシュを維持できます。
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>データのバックアップ</CardTitle>
          <CardDescription>
            メタデータ（images、shop_settings、product_master）と画像ファイルをZIP形式でエクスポートします。保存先を選択してください。
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
