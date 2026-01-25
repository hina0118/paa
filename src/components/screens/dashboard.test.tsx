import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Dashboard } from './dashboard';
import { ParseProvider } from '@/contexts/parse-context';
import { mockInvoke, mockListen } from '@/test/setup';

const renderWithProviders = (ui: React.ReactElement) => {
  return render(<ParseProvider>{ui}</ParseProvider>);
};

describe('Dashboard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // セットアップのモックを上書き
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_email_stats') {
        return Promise.resolve({
          total_emails: 100,
          with_body_plain: 80,
          with_body_html: 90,
          without_body: 10,
          avg_plain_length: 500,
          avg_html_length: 2000,
        });
      }
      if (cmd === 'get_parse_status') {
        return Promise.resolve({
          batch_size: 100,
          parse_status: 'idle',
          last_parse_started_at: null,
          last_parse_completed_at: null,
          last_error_message: null,
          total_parsed_count: 0,
        });
      }
      return Promise.resolve(null);
    });
    mockListen.mockResolvedValue(() => {});
  });

  it('renders dashboard heading', () => {
    renderWithProviders(<Dashboard />);
    expect(
      screen.getByRole('heading', { name: /ダッシュボード/i })
    ).toBeInTheDocument();
  });

  it('renders with correct heading level', () => {
    renderWithProviders(<Dashboard />);
    const heading = screen.getByRole('heading', { name: /ダッシュボード/i });
    expect(heading.tagName).toBe('H1');
  });

  it('applies container styling', () => {
    const { container } = renderWithProviders(<Dashboard />);
    const div = container.querySelector('.container');
    expect(div).toBeInTheDocument();
    expect(div).toHaveClass('mx-auto');
    expect(div).toHaveClass('py-10');
  });

  it('applies heading styling', () => {
    renderWithProviders(<Dashboard />);
    const heading = screen.getByRole('heading', { name: /ダッシュボード/i });
    expect(heading).toHaveClass('text-3xl');
    expect(heading).toHaveClass('font-bold');
  });

  it('renders without errors', () => {
    expect(() => renderWithProviders(<Dashboard />)).not.toThrow();
  });
});
