import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Settings } from './settings';

describe('Settings', () => {
  it('renders settings heading', () => {
    render(<Settings />);
    expect(
      screen.getByRole('heading', { name: /Settings画面です/i })
    ).toBeInTheDocument();
  });

  it('renders with correct heading level', () => {
    render(<Settings />);
    const heading = screen.getByRole('heading', { name: /Settings画面です/i });
    expect(heading.tagName).toBe('H1');
  });

  it('applies container styling', () => {
    const { container } = render(<Settings />);
    const div = container.querySelector('.container');
    expect(div).toBeInTheDocument();
    expect(div).toHaveClass('mx-auto');
    expect(div).toHaveClass('py-10');
  });

  it('applies heading styling', () => {
    render(<Settings />);
    const heading = screen.getByRole('heading', { name: /Settings画面です/i });
    expect(heading).toHaveClass('text-3xl');
    expect(heading).toHaveClass('font-bold');
  });

  it('renders without errors', () => {
    expect(() => render(<Settings />)).not.toThrow();
  });
});
