import { Package } from 'lucide-react';
import { Card, CardContent, CardFooter } from '@/components/ui/card';
import { StatusBadge } from './status-badge';
import { useImageUrl } from '@/hooks/useImageUrl';
import type { OrderItemRow } from '@/lib/types';
import { cn, formatDate, formatPrice } from '@/lib/utils';

type OrderItemCardProps = {
  item: OrderItemRow;
  onClick?: () => void;
  className?: string;
};

function handleCardKeyDown(
  e: React.KeyboardEvent,
  onClick: (() => void) | undefined
) {
  if (!onClick) return;
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault();
    onClick();
  }
}

export function OrderItemCard({
  item,
  onClick,
  className,
}: OrderItemCardProps) {
  const getImageUrl = useImageUrl();
  const imageSrc = getImageUrl(item.fileName);

  return (
    <Card
      role={onClick ? 'button' : undefined}
      tabIndex={onClick ? 0 : undefined}
      className={cn(
        onClick && 'cursor-pointer transition-colors hover:bg-muted/50',
        'overflow-hidden',
        className
      )}
      onClick={onClick}
      onKeyDown={onClick ? (e) => handleCardKeyDown(e, onClick) : undefined}
    >
      <div className="aspect-square bg-muted/50 flex items-center justify-center overflow-hidden">
        {imageSrc ? (
          <img
            src={imageSrc}
            alt={item.itemName}
            className="w-full h-full object-cover"
            loading="lazy"
          />
        ) : (
          <Package className="h-16 w-16 text-muted-foreground" />
        )}
      </div>
      <CardContent className="p-3 space-y-1">
        <div className="flex items-start justify-between gap-2">
          <h3 className="font-medium text-sm line-clamp-2 flex-1">
            {item.productName ?? item.itemName}
          </h3>
          <StatusBadge status={item.deliveryStatus} className="shrink-0" />
        </div>
        {(item.maker || item.series || item.scale) && (
          <p className="text-xs text-muted-foreground line-clamp-1">
            {[item.maker, item.series, item.scale].filter(Boolean).join(' / ')}
          </p>
        )}
        {!(item.maker || item.series || item.scale) &&
          (item.brand || item.category) && (
            <p className="text-xs text-muted-foreground line-clamp-1">
              {[item.brand, item.category].filter(Boolean).join(' / ')}
            </p>
          )}
        <p className="text-sm font-semibold">{formatPrice(item.price)}</p>
      </CardContent>
      <CardFooter className="p-3 pt-0 flex flex-wrap gap-x-2 gap-y-0 text-xs text-muted-foreground">
        <span>{item.shopName ?? item.shopDomain ?? '-'}</span>
        <span>・</span>
        <span>{formatDate(item.orderDate)}</span>
      </CardFooter>
    </Card>
  );
}
