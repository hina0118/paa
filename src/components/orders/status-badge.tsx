import { cn } from '@/lib/utils';
import type { DeliveryStatus } from '@/lib/types';

const STATUS_CONFIG: Record<
  DeliveryStatus,
  { label: string; className: string }
> = {
  not_shipped: {
    label: '発送待ち',
    className: 'bg-muted text-muted-foreground',
  },
  preparing: {
    label: '準備中',
    className: 'bg-yellow-500/20 text-yellow-700 dark:text-yellow-400',
  },
  shipped: {
    label: '発送済み',
    className: 'bg-blue-500/20 text-blue-700 dark:text-blue-400',
  },
  in_transit: {
    label: '配送中',
    className: 'bg-blue-500/20 text-blue-700 dark:text-blue-400',
  },
  out_for_delivery: {
    label: '配達中',
    className: 'bg-blue-500/20 text-blue-700 dark:text-blue-400',
  },
  delivered: {
    label: '到着済み',
    className: 'bg-green-500/20 text-green-700 dark:text-green-400',
  },
  failed: {
    label: '配達失敗',
    className: 'bg-red-500/20 text-red-700 dark:text-red-400',
  },
  returned: {
    label: '返送',
    className: 'bg-orange-500/20 text-orange-700 dark:text-orange-400',
  },
  cancelled: {
    label: 'キャンセル',
    className: 'bg-red-500/20 text-red-700 dark:text-red-400',
  },
};

type StatusBadgeProps = {
  status: DeliveryStatus | null;
  className?: string;
};

export function StatusBadge({ status, className }: StatusBadgeProps) {
  if (!status) return null;
  const config = STATUS_CONFIG[status];
  if (!config) return null;
  return (
    <span
      className={cn(
        'inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium',
        config.className,
        className
      )}
    >
      {config.label}
    </span>
  );
}
