import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { Skeleton } from './skeleton';

describe('Skeleton', () => {
  it('renders a div element', () => {
    const { container } = render(<Skeleton />);
    expect(container.firstChild).toBeInTheDocument();
    expect(container.firstChild?.nodeName).toBe('DIV');
  });

  it('applies default classes', () => {
    const { container } = render(<Skeleton />);
    const el = container.firstChild as HTMLElement;
    expect(el).toHaveClass('animate-pulse');
    expect(el).toHaveClass('rounded-md');
    expect(el).toHaveClass('bg-muted');
  });

  it('merges custom className with defaults', () => {
    const { container } = render(<Skeleton className="h-4 w-full" />);
    const el = container.firstChild as HTMLElement;
    expect(el).toHaveClass('animate-pulse');
    expect(el).toHaveClass('h-4');
    expect(el).toHaveClass('w-full');
  });

  it('passes through other HTML attributes', () => {
    const { container } = render(<Skeleton data-testid="loading-skeleton" />);
    const el = container.firstChild as HTMLElement;
    expect(el).toHaveAttribute('data-testid', 'loading-skeleton');
  });
});
