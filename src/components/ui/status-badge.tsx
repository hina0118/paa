export interface StatusConfig {
  [key: string]: { label: string; className: string };
}

interface StatusBadgeProps {
  status?: string;
  config: StatusConfig;
}

const DEFAULT_STYLE = { label: '不明', className: 'bg-gray-100 text-gray-800' };

export function StatusBadge({ status, config }: StatusBadgeProps) {
  const { label, className } = (status && config[status]) || DEFAULT_STYLE;
  return (
    <div className="flex items-center gap-2">
      <span className="text-sm font-medium">ステータス:</span>
      <span className={`px-2 py-1 rounded text-xs font-semibold ${className}`}>
        {label}
      </span>
    </div>
  );
}

/** Gmail同期用ステータス設定 */
export const SYNC_STATUS_CONFIG: StatusConfig = {
  syncing: { label: '同期中', className: 'bg-blue-100 text-blue-800' },
  idle: { label: '待機中', className: 'bg-green-100 text-green-800' },
  paused: { label: '一時停止', className: 'bg-yellow-100 text-yellow-800' },
  error: { label: 'エラー', className: 'bg-red-100 text-red-800' },
};

/** メールパース / 商品名解析用ステータス設定 */
export const PARSE_STATUS_CONFIG: StatusConfig = {
  running: { label: '処理中', className: 'bg-blue-100 text-blue-800' },
  idle: { label: '待機中', className: 'bg-green-100 text-green-800' },
  completed: { label: '完了', className: 'bg-green-100 text-green-800' },
  error: { label: 'エラー', className: 'bg-red-100 text-red-800' },
};
