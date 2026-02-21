import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { LayoutDashboard } from 'lucide-react';
import { PageHeader } from './page-header';

describe('PageHeader', () => {
  it('renders title', () => {
    render(<PageHeader title="Test Title" icon={LayoutDashboard} />);
    expect(screen.getByText('Test Title')).toBeInTheDocument();
  });

  it('renders icon with aria-hidden', () => {
    const { container } = render(
      <PageHeader title="Test" icon={LayoutDashboard} />
    );
    const svg = container.querySelector('svg');
    expect(svg).toHaveAttribute('aria-hidden', 'true');
  });

  it('renders optional description when provided', () => {
    render(
      <PageHeader
        title="Test"
        icon={LayoutDashboard}
        description="A description"
      />
    );
    expect(screen.getByText('A description')).toBeInTheDocument();
  });

  it('does not render description when omitted', () => {
    const { container } = render(
      <PageHeader title="Test" icon={LayoutDashboard} />
    );
    expect(container.querySelector('p')).toBeNull();
  });

  it('renders children in the actions area', () => {
    render(
      <PageHeader title="Test" icon={LayoutDashboard}>
        <button>Action</button>
      </PageHeader>
    );
    expect(screen.getByRole('button', { name: 'Action' })).toBeInTheDocument();
  });

  it('merges custom className', () => {
    const { container } = render(
      <PageHeader
        title="Test"
        icon={LayoutDashboard}
        className="custom-class"
      />
    );
    const root = container.firstChild as HTMLElement;
    expect(root).toHaveClass('custom-class');
    expect(root).toHaveClass('mb-8');
  });
});
