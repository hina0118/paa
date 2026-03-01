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
}

export function CircularProgress({
  value,
  size = 80,
  strokeWidth = 6,
  className,
}: CircularProgressProps) {
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const offset =
    circumference - (Math.min(Math.max(value, 0), 100) / 100) * circumference;

  return (
    <div
      role="progressbar"
      aria-valuenow={Math.round(value)}
      aria-valuemin={0}
      aria-valuemax={100}
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
        {Math.round(value)}%
      </span>
    </div>
  );
}
