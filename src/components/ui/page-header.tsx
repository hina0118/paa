import type { ComponentType, ReactNode } from 'react';

import { cn } from '@/lib/utils';

interface PageHeaderProps {
  title: string;
  description?: string;
  icon: ComponentType<{ className?: string }>;
  children?: ReactNode;
  className?: string;
}

export function PageHeader({
  title,
  description,
  icon: Icon,
  children,
  className,
}: PageHeaderProps) {
  return (
    <div
      className={cn(
        'sticky top-0 z-10 bg-background/95 backdrop-blur border-b',
        '-mx-6 px-6 py-4',
        'flex justify-between items-center',
        className
      )}
    >
      <div className="flex items-center gap-3">
        <div className="p-2 rounded-xl bg-primary/10 ring-1 ring-primary/20">
          <Icon className="h-6 w-6 text-primary" aria-hidden="true" />
        </div>
        <div>
          <h1 className="text-2xl font-bold tracking-tight">{title}</h1>
          {description && (
            <p className="text-sm text-muted-foreground mt-0.5">
              {description}
            </p>
          )}
        </div>
      </div>
      {children && <div className="flex items-center gap-2">{children}</div>}
    </div>
  );
}
