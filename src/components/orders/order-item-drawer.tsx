import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
import { StatusBadge } from './status-badge';
import { useImageUrl } from '@/hooks/useImageUrl';
import type { OrderItemRow } from '@/lib/types';
import { formatDate, formatPrice } from '@/lib/utils';

type OrderItemDrawerProps = {
  item: OrderItemRow | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

export function OrderItemDrawer({
  item,
  open,
  onOpenChange,
}: OrderItemDrawerProps) {
  const getImageUrl = useImageUrl();
  const imageSrc = item ? getImageUrl(item.fileName) : null;

  if (!item) return null;

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" className="sm:max-w-md overflow-y-auto">
        <SheetHeader>
          <SheetTitle className="text-left pr-8">{item.itemName}</SheetTitle>
        </SheetHeader>
        <div className="mt-6 space-y-6">
          <div className="aspect-square max-h-64 bg-muted/50 rounded-lg overflow-hidden flex items-center justify-center">
            {imageSrc ? (
              <img
                src={imageSrc}
                alt={item.itemName}
                className="w-full h-full object-cover"
              />
            ) : (
              <span className="text-muted-foreground text-sm">画像なし</span>
            )}
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
  );
}
