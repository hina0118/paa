import type { StatusConfig } from './status-config';

export type { StatusConfig };
export { SYNC_STATUS_CONFIG, PARSE_STATUS_CONFIG } from './status-config';

interface StatusBadgeProps {
  status?: string;
  config: StatusConfig;
}

const DEFAULT_STYLE = {
  label: '不明',
  className: 'bg-muted text-muted-foreground border border-border',
};

export function StatusBadge({ status, config }: StatusBadgeProps) {
  const { label, className } = (status && config[status]) || DEFAULT_STYLE;
  return (
    <div className="flex items-center gap-2">
      <span className="text-sm font-medium">ステータス:</span>
      <span
        className={`px-2 py-0.5 rounded-md text-xs font-medium ${className}`}
      >
        {label}
      </span>
    </div>
  );
}
