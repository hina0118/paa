import { Package } from 'lucide-react';
import { StatusBadge } from './status-badge';
import { useImageUrl } from '@/hooks/useImageUrl';
import type { OrderItemRow } from '@/lib/types';
import { cn, formatDate, formatPrice } from '@/lib/utils';

type OrderItemRowViewProps = {
  item: OrderItemRow;
  onClick?: () => void;
  className?: string;
};

export function OrderItemRowView({
  item,
  onClick,
  className,
}: OrderItemRowViewProps) {
  const getImageUrl = useImageUrl();
  const imageSrc = getImageUrl(item.fileName);
  const isInteractive = Boolean(onClick);

  return (
    <div
      role={isInteractive ? 'button' : undefined}
      tabIndex={isInteractive ? 0 : undefined}
      onKeyDown={
        isInteractive
          ? (e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onClick?.();
              }
            }
          : undefined
      }
      className={cn(
        'flex items-center gap-4 p-3 border-b',
        isInteractive && 'cursor-pointer transition-colors hover:bg-muted/50',
        className
      )}
      onClick={onClick}
    >
      <div className="w-16 h-16 shrink-0 bg-muted/50 flex items-center justify-center overflow-hidden rounded">
        {imageSrc ? (
          <img
            src={imageSrc}
            alt={item.itemName}
            className="w-full h-full object-cover"
            loading="lazy"
          />
        ) : (
          <Package className="h-8 w-8 text-muted-foreground" />
        )}
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <h3 className="font-medium text-sm truncate">
            {item.productName ?? item.itemName}
          </h3>
          <StatusBadge status={item.deliveryStatus} className="shrink-0" />
        </div>
        {(item.maker || item.series || item.scale) && (
          <p className="text-xs text-muted-foreground truncate">
            {[item.maker, item.series, item.scale].filter(Boolean).join(' / ')}
          </p>
        )}
        {!(item.maker || item.series || item.scale) &&
          (item.brand || item.category) && (
            <p className="text-xs text-muted-foreground truncate">
              {[item.brand, item.category].filter(Boolean).join(' / ')}
            </p>
          )}
      </div>
      <div className="text-right shrink-0">
        <p className="text-sm font-semibold">{formatPrice(item.price)}</p>
        <p className="text-xs text-muted-foreground">
          {item.shopName ?? item.shopDomain ?? '-'} ・{' '}
          {formatDate(item.orderDate)}
        </p>
      </div>
    </div>
  );
}
