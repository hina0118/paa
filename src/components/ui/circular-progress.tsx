import { cn } from '@/lib/utils';

interface CircularProgressProps {
  /** 進捗率 0〜100 */
  value: number;
  /** 円のサイズ（px） */
  size?: number;
  /** 線の太さ（px） */
  strokeWidth?: number;
  /** クラス名 */
  className?: string;
  /** progressbar のアクセシブルネーム（未指定時は "処理進捗"。コンテキストに応じて具体的なラベルを指定することを推奨） */
  'aria-label'?: string;
}

export function CircularProgress({
  value,
  size = 80,
  strokeWidth = 6,
  className,
  'aria-label': ariaLabel = '処理進捗',
}: CircularProgressProps) {
  const clampedValue = Math.min(Math.max(value, 0), 100);
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference - (clampedValue / 100) * circumference;

  return (
    <div
      role="progressbar"
      aria-valuenow={Math.round(clampedValue)}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-label={ariaLabel}
      className={cn(
        'relative inline-flex items-center justify-center',
        className
      )}
    >
      <svg width={size} height={size} className="-rotate-90" aria-hidden="true">
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          className="stroke-secondary"
          strokeWidth={strokeWidth}
        />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          className="stroke-primary transition-all duration-300 ease-in-out"
          strokeWidth={strokeWidth}
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          strokeLinecap="round"
        />
      </svg>
      <span className="absolute text-sm font-semibold">
        {Math.round(clampedValue)}%
      </span>
    </div>
  );
}
