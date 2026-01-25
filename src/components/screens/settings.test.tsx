import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Settings } from './settings';
import { SyncProvider } from '@/contexts/sync-context';
import { ParseProvider } from '@/contexts/parse-context';
import { mockInvoke, mockListen } from '@/test/setup';

const renderWithProviders = (ui: React.ReactElement) => {
  return render(
    <SyncProvider>
      <ParseProvider>{ui}</ParseProvider>
    </SyncProvider>
  );
};

describe('Settings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // セットアップのモックを上書き
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_sync_status') {
        return Promise.resolve({
          batch_size: 50,
          max_iterations: 100,
          sync_status: 'idle',
          last_sync_started_at: null,
          last_sync_completed_at: null,
          total_synced_count: 0,
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

  it('renders settings heading', () => {
    renderWithProviders(<Settings />);
    expect(
      screen.getByRole('heading', { name: /設定/i, level: 1 })
    ).toBeInTheDocument();
  });

  it('renders with correct heading level', () => {
    renderWithProviders(<Settings />);
    const heading = screen.getByRole('heading', { name: /設定/i, level: 1 });
    expect(heading.tagName).toBe('H1');
  });

  it('applies container styling', () => {
    const { container } = renderWithProviders(<Settings />);
    const div = container.querySelector('.container');
    expect(div).toBeInTheDocument();
    expect(div).toHaveClass('mx-auto');
    expect(div).toHaveClass('py-10');
  });

  it('applies heading styling', () => {
    renderWithProviders(<Settings />);
    const heading = screen.getByRole('heading', { name: /設定/i, level: 1 });
    expect(heading).toHaveClass('text-3xl');
    expect(heading).toHaveClass('font-bold');
  });

  it('renders without errors', () => {
    expect(() => renderWithProviders(<Settings />)).not.toThrow();
  });
});
