import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { StatusBadge } from './status-badge';

describe('StatusBadge', () => {
  it('returns null when status is null', () => {
    const { container } = render(<StatusBadge status={null} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders 発送待ち for not_shipped', () => {
    render(<StatusBadge status="not_shipped" />);
    expect(screen.getByText('発送待ち')).toBeInTheDocument();
  });

  it('renders 発送済み for shipped', () => {
    render(<StatusBadge status="shipped" />);
    expect(screen.getByText('発送済み')).toBeInTheDocument();
  });

  it('renders 到着済み for delivered', () => {
    render(<StatusBadge status="delivered" />);
    expect(screen.getByText('到着済み')).toBeInTheDocument();
  });

  it('renders キャンセル for cancelled', () => {
    render(<StatusBadge status="cancelled" />);
    expect(screen.getByText('キャンセル')).toBeInTheDocument();
  });
});
