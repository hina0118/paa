export interface StatusConfig {
  [key: string]: { label: string; className: string };
}

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

/** Gmail同期用ステータス設定 */
export const SYNC_STATUS_CONFIG: StatusConfig = {
  syncing: {
    label: '同期中',
    className: 'bg-primary/10 text-primary border border-primary/20',
  },
  idle: {
    label: '待機中',
    className:
      'bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 border border-emerald-500/20',
  },
  paused: {
    label: '一時停止',
    className:
      'bg-amber-500/10 text-amber-700 dark:text-amber-400 border border-amber-500/20',
  },
  error: {
    label: 'エラー',
    className:
      'bg-destructive/10 text-destructive border border-destructive/20',
  },
};

/** メールパース / 商品名解析用ステータス設定 */
export const PARSE_STATUS_CONFIG: StatusConfig = {
  running: {
    label: '処理中',
    className: 'bg-primary/10 text-primary border border-primary/20',
  },
  idle: {
    label: '待機中',
    className:
      'bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 border border-emerald-500/20',
  },
  completed: {
    label: '完了',
    className:
      'bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 border border-emerald-500/20',
  },
  error: {
    label: 'エラー',
    className:
      'bg-destructive/10 text-destructive border border-destructive/20',
  },
};
