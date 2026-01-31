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

  it('renders 準備中 for preparing', () => {
    render(<StatusBadge status="preparing" />);
    expect(screen.getByText('準備中')).toBeInTheDocument();
  });

  it('renders 配送中 for in_transit', () => {
    render(<StatusBadge status="in_transit" />);
    expect(screen.getByText('配送中')).toBeInTheDocument();
  });

  it('renders 配達中 for out_for_delivery', () => {
    render(<StatusBadge status="out_for_delivery" />);
    expect(screen.getByText('配達中')).toBeInTheDocument();
  });

  it('renders 配達失敗 for failed', () => {
    render(<StatusBadge status="failed" />);
    expect(screen.getByText('配達失敗')).toBeInTheDocument();
  });

  it('renders 返送 for returned', () => {
    render(<StatusBadge status="returned" />);
    expect(screen.getByText('返送')).toBeInTheDocument();
  });

  it('returns null for unknown status', () => {
    const { container } = render(
      <StatusBadge status={'unknown_status' as never} />
    );
    expect(container.firstChild).toBeNull();
  });

  it('applies custom className', () => {
    render(<StatusBadge status="delivered" className="custom-class" />);
    const badge = screen.getByText('到着済み');
    expect(badge).toHaveClass('custom-class');
  });
});
