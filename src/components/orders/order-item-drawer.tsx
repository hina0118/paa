import { useState, useCallback } from 'react';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
import { Button } from '@/components/ui/button';
import { StatusBadge } from './status-badge';
import { ImageSearchDialog } from './image-search-dialog';
import { useImageUrl } from '@/hooks/useImageUrl';
import type { OrderItemRow } from '@/lib/types';
import { formatDate, formatPrice } from '@/lib/utils';
import { Search } from 'lucide-react';

type OrderItemDrawerProps = {
  item: OrderItemRow | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImageUpdated?: () => void;
};

export function OrderItemDrawer({
  item,
  open,
  onOpenChange,
  onImageUpdated,
}: OrderItemDrawerProps) {
  const getImageUrl = useImageUrl();
  const [imageSearchOpen, setImageSearchOpen] = useState(false);
  const [imageKey, setImageKey] = useState(0);

  const imageSrc = item ? getImageUrl(item.fileName) : null;

  const handleImageSaved = useCallback(() => {
    // 画像が保存されたら、親コンポーネントに通知してリストを更新
    onImageUpdated?.();
    // 画像表示を強制的に更新するためにキーを変更
    setImageKey((prev) => prev + 1);
  }, [onImageUpdated]);

  if (!item) return null;

  return (
    <>
      <Sheet open={open} onOpenChange={onOpenChange}>
        <SheetContent side="right" className="sm:max-w-md overflow-y-auto">
          <SheetHeader>
            <SheetTitle className="text-left pr-8">{item.itemName}</SheetTitle>
          </SheetHeader>
          <div className="mt-6 space-y-6">
            <div className="space-y-2">
              <div
                key={imageKey}
                className="aspect-square max-h-64 bg-muted/50 rounded-lg overflow-hidden flex items-center justify-center cursor-pointer hover:opacity-80 transition-opacity"
                onClick={() => setImageSearchOpen(true)}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    setImageSearchOpen(true);
                  }
                }}
                aria-label="画像を検索"
                title="クリックして画像を検索"
              >
                {imageSrc ? (
                  <img
                    src={imageSrc}
                    alt={item.itemName}
                    className="w-full h-full object-cover"
                  />
                ) : (
                  <span className="text-muted-foreground text-sm">
                    画像なし
                  </span>
                )}
              </div>
              <Button
                variant="outline"
                size="sm"
                className="w-full"
                onClick={() => setImageSearchOpen(true)}
              >
                <Search className="mr-2 h-4 w-4" />
                画像を検索
              </Button>
            </div>
            <div>
              <h4 className="text-sm font-medium text-muted-foreground mb-1">
                ステータス
              </h4>
              <StatusBadge status={item.deliveryStatus} />
            </div>
            <div>
              <h4 className="text-sm font-medium text-muted-foreground mb-1">
                価格
              </h4>
              <p className="text-lg font-semibold">{formatPrice(item.price)}</p>
            </div>
            <div>
              <h4 className="text-sm font-medium text-muted-foreground mb-1">
                ショップ
              </h4>
              <p>{item.shopName ?? item.shopDomain ?? '-'}</p>
            </div>
            <div>
              <h4 className="text-sm font-medium text-muted-foreground mb-1">
                注文番号
              </h4>
              <p>{item.orderNumber ?? '-'}</p>
            </div>
            <div>
              <h4 className="text-sm font-medium text-muted-foreground mb-1">
                注文日
              </h4>
              <p>{formatDate(item.orderDate)}</p>
            </div>
            {(item.brand || item.category) && (
              <div>
                <h4 className="text-sm font-medium text-muted-foreground mb-1">
                  メーカー / 作品名
                </h4>
                <p>{[item.brand, item.category].filter(Boolean).join(' / ')}</p>
              </div>
            )}
          </div>
        </SheetContent>
      </Sheet>

      <ImageSearchDialog
        open={imageSearchOpen}
        onOpenChange={setImageSearchOpen}
        itemId={item.id}
        itemName={item.itemName}
        onImageSaved={handleImageSaved}
      />
    </>
  );
}
