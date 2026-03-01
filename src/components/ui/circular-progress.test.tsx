import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { CircularProgress } from './circular-progress';

describe('CircularProgress', () => {
  it('renders progressbar with correct role', () => {
    const { container } = render(<CircularProgress value={50} />);
    const progressbar = container.querySelector('[role="progressbar"]');
    expect(progressbar).toBeInTheDocument();
  });

  it('rounds and sets aria-valuenow to the given value', () => {
    const { container } = render(<CircularProgress value={75} />);
    const progressbar = container.querySelector('[role="progressbar"]');
    expect(progressbar).toHaveAttribute('aria-valuenow', '75');
  });

  it('sets aria-valuemin to 0 and aria-valuemax to 100', () => {
    const { container } = render(<CircularProgress value={50} />);
    const progressbar = container.querySelector('[role="progressbar"]');
    expect(progressbar).toHaveAttribute('aria-valuemin', '0');
    expect(progressbar).toHaveAttribute('aria-valuemax', '100');
  });

  it('clamps value below 0 to 0', () => {
    const { container } = render(<CircularProgress value={-10} />);
    const progressbar = container.querySelector('[role="progressbar"]');
    expect(progressbar).toHaveAttribute('aria-valuenow', '0');
  });

  it('clamps value above 100 to 100', () => {
    const { container } = render(<CircularProgress value={150} />);
    const progressbar = container.querySelector('[role="progressbar"]');
    expect(progressbar).toHaveAttribute('aria-valuenow', '100');
  });

  it('displays the rounded percentage text', () => {
    const { getByText } = render(<CircularProgress value={50} />);
    expect(getByText('50%')).toBeInTheDocument();
  });

  it('rounds fractional values in displayed text and aria-valuenow', () => {
    const { getByText, container } = render(<CircularProgress value={33.7} />);
    expect(getByText('34%')).toBeInTheDocument();
    const progressbar = container.querySelector('[role="progressbar"]');
    expect(progressbar).toHaveAttribute('aria-valuenow', '34');
  });

  it('uses default aria-label "処理進捗"', () => {
    const { container } = render(<CircularProgress value={50} />);
    const progressbar = container.querySelector('[role="progressbar"]');
    expect(progressbar).toHaveAttribute('aria-label', '処理進捗');
  });

  it('accepts a custom aria-label', () => {
    const { container } = render(
      <CircularProgress value={50} aria-label="注文処理進捗" />
    );
    const progressbar = container.querySelector('[role="progressbar"]');
    expect(progressbar).toHaveAttribute('aria-label', '注文処理進捗');
  });

  it('accepts custom className', () => {
    const { container } = render(
      <CircularProgress value={50} className="custom-class" />
    );
    const progressbar = container.querySelector('[role="progressbar"]');
    expect(progressbar).toHaveClass('custom-class');
  });

  it('renders SVG with provided size', () => {
    const { container } = render(<CircularProgress value={50} size={120} />);
    const svg = container.querySelector('svg');
    expect(svg).toHaveAttribute('width', '120');
    expect(svg).toHaveAttribute('height', '120');
  });
});
