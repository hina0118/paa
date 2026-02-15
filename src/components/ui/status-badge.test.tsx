import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import {
  StatusBadge,
  SYNC_STATUS_CONFIG,
  PARSE_STATUS_CONFIG,
  type StatusConfig,
} from './status-badge';

describe('StatusBadge', () => {
  describe('with SYNC_STATUS_CONFIG', () => {
    it('renders syncing status correctly', () => {
      render(<StatusBadge status="syncing" config={SYNC_STATUS_CONFIG} />);

      expect(screen.getByText('ステータス:')).toBeInTheDocument();
      expect(screen.getByText('同期中')).toBeInTheDocument();

      const badge = screen.getByText('同期中');
      expect(badge).toHaveClass('bg-blue-100');
      expect(badge).toHaveClass('text-blue-800');
    });

    it('renders idle status correctly', () => {
      render(<StatusBadge status="idle" config={SYNC_STATUS_CONFIG} />);

      expect(screen.getByText('待機中')).toBeInTheDocument();

      const badge = screen.getByText('待機中');
      expect(badge).toHaveClass('bg-green-100');
      expect(badge).toHaveClass('text-green-800');
    });

    it('renders paused status correctly', () => {
      render(<StatusBadge status="paused" config={SYNC_STATUS_CONFIG} />);

      expect(screen.getByText('一時停止')).toBeInTheDocument();

      const badge = screen.getByText('一時停止');
      expect(badge).toHaveClass('bg-yellow-100');
      expect(badge).toHaveClass('text-yellow-800');
    });

    it('renders error status correctly', () => {
      render(<StatusBadge status="error" config={SYNC_STATUS_CONFIG} />);

      expect(screen.getByText('エラー')).toBeInTheDocument();

      const badge = screen.getByText('エラー');
      expect(badge).toHaveClass('bg-red-100');
      expect(badge).toHaveClass('text-red-800');
    });
  });

  describe('with PARSE_STATUS_CONFIG', () => {
    it('renders running status correctly', () => {
      render(<StatusBadge status="running" config={PARSE_STATUS_CONFIG} />);

      expect(screen.getByText('処理中')).toBeInTheDocument();

      const badge = screen.getByText('処理中');
      expect(badge).toHaveClass('bg-blue-100');
      expect(badge).toHaveClass('text-blue-800');
    });

    it('renders idle status correctly', () => {
      render(<StatusBadge status="idle" config={PARSE_STATUS_CONFIG} />);

      expect(screen.getByText('待機中')).toBeInTheDocument();

      const badge = screen.getByText('待機中');
      expect(badge).toHaveClass('bg-green-100');
      expect(badge).toHaveClass('text-green-800');
    });

    it('renders completed status correctly', () => {
      render(<StatusBadge status="completed" config={PARSE_STATUS_CONFIG} />);

      expect(screen.getByText('完了')).toBeInTheDocument();

      const badge = screen.getByText('完了');
      expect(badge).toHaveClass('bg-green-100');
      expect(badge).toHaveClass('text-green-800');
    });

    it('renders error status correctly', () => {
      render(<StatusBadge status="error" config={PARSE_STATUS_CONFIG} />);

      expect(screen.getByText('エラー')).toBeInTheDocument();

      const badge = screen.getByText('エラー');
      expect(badge).toHaveClass('bg-red-100');
      expect(badge).toHaveClass('text-red-800');
    });
  });

  describe('default handling', () => {
    it('renders default label and style for undefined status', () => {
      const customConfig: StatusConfig = {
        active: { label: 'アクティブ', className: 'bg-blue-100 text-blue-800' },
      };

      render(<StatusBadge status={undefined} config={customConfig} />);

      expect(screen.getByText('不明')).toBeInTheDocument();

      const badge = screen.getByText('不明');
      expect(badge).toHaveClass('bg-gray-100');
      expect(badge).toHaveClass('text-gray-800');
    });

    it('renders default label and style for unknown status', () => {
      render(<StatusBadge status="unknown" config={SYNC_STATUS_CONFIG} />);

      expect(screen.getByText('不明')).toBeInTheDocument();

      const badge = screen.getByText('不明');
      expect(badge).toHaveClass('bg-gray-100');
      expect(badge).toHaveClass('text-gray-800');
    });

    it('renders default label when status is empty string', () => {
      render(<StatusBadge status="" config={SYNC_STATUS_CONFIG} />);

      expect(screen.getByText('不明')).toBeInTheDocument();
    });
  });

  describe('custom config', () => {
    it('works with custom status configuration', () => {
      const customConfig: StatusConfig = {
        pending: {
          label: '保留中',
          className: 'bg-orange-100 text-orange-800',
        },
        approved: {
          label: '承認済み',
          className: 'bg-purple-100 text-purple-800',
        },
      };

      render(<StatusBadge status="pending" config={customConfig} />);

      expect(screen.getByText('保留中')).toBeInTheDocument();

      const badge = screen.getByText('保留中');
      expect(badge).toHaveClass('bg-orange-100');
      expect(badge).toHaveClass('text-orange-800');
    });
  });

  describe('structure', () => {
    it('has correct structure with label and badge', () => {
      render(<StatusBadge status="idle" config={SYNC_STATUS_CONFIG} />);

      const container = screen.getByText('ステータス:').parentElement;
      expect(container).toHaveClass('flex');
      expect(container).toHaveClass('items-center');
      expect(container).toHaveClass('gap-2');

      const label = screen.getByText('ステータス:');
      expect(label).toHaveClass('text-sm');
      expect(label).toHaveClass('font-medium');

      const badge = screen.getByText('待機中');
      expect(badge).toHaveClass('px-2');
      expect(badge).toHaveClass('py-1');
      expect(badge).toHaveClass('rounded');
      expect(badge).toHaveClass('text-xs');
      expect(badge).toHaveClass('font-semibold');
    });
  });
});
