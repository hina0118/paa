import { Sun, Moon, Monitor } from 'lucide-react';
import { useTheme } from '@/contexts/use-theme';
import type { Theme } from '@/contexts/theme-context-value';
import { cn } from '@/lib/utils';

type ThemeOption = {
  value: Theme;
  icon: typeof Sun;
  label: string;
};

const options: ThemeOption[] = [
  { value: 'light', icon: Sun, label: 'ライト' },
  { value: 'dark', icon: Moon, label: 'ダーク' },
  { value: 'system', icon: Monitor, label: 'システム' },
];

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();

  return (
    <div
      className="flex items-center gap-1 rounded-lg bg-muted/60 p-1"
      role="group"
      aria-label="テーマ切り替え"
    >
      {options.map(({ value, icon: Icon, label }) => (
        <button
          key={value}
          onClick={() => setTheme(value)}
          aria-pressed={theme === value}
          aria-label={label}
          title={label}
          className={cn(
            'flex flex-1 items-center justify-center rounded-md p-1.5 transition-colors',
            theme === value
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground'
          )}
        >
          <Icon className="h-3.5 w-3.5" />
        </button>
      ))}
    </div>
  );
}
