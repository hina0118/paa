import { describe, it, expect, vi } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { BatchSection } from './batch-section';
import { SYNC_STATUS_CONFIG } from './status-badge';
import type { BatchProgress } from '@/contexts/batch-progress-types';

describe('BatchSection', () => {
  const mockProgress: BatchProgress = {
    task_name: 'テストタスク',
    batch_number: 1,
    batch_size: 100,
    total_items: 300,
    processed_count: 150,
    success_count: 145,
    failed_count: 5,
    progress_percent: 50,
    status_message: 'バッチ 1 を処理中...',
    is_complete: false,
  };

  const defaultProps = {
    title: '1. テストセクション',
    controlTitle: 'テスト処理',
    controlDescription: 'テスト処理の説明',
    isRunning: false,
    progress: null,
    onStart: vi.fn(),
    startLabel: '開始',
    runningLabel: '実行中',
    completeMessage: '処理が完了しました',
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('basic rendering', () => {
    it('renders section with title and control card', () => {
      render(<BatchSection {...defaultProps} />);

      expect(screen.getByText('1. テストセクション')).toBeInTheDocument();
      expect(screen.getByText('テスト処理')).toBeInTheDocument();
      expect(screen.getByText('テスト処理の説明')).toBeInTheDocument();
    });

    it('renders start button when not running', () => {
      render(<BatchSection {...defaultProps} />);

      const button = screen.getByRole('button', { name: '開始' });
      expect(button).toBeInTheDocument();
      expect(button).not.toBeDisabled();
    });

    it('renders running button when running', () => {
      render(<BatchSection {...defaultProps} isRunning />);

      const button = screen.getByRole('button', { name: '実行中' });
      expect(button).toBeInTheDocument();
      expect(button).toBeDisabled();
      expect(button).toHaveClass('bg-secondary');
    });
  });

  describe('button interactions', () => {
    it('calls onStart when start button is clicked', async () => {
      const user = userEvent.setup();
      const onStart = vi.fn();

      render(<BatchSection {...defaultProps} onStart={onStart} />);

      await user.click(screen.getByRole('button', { name: '開始' }));

      expect(onStart).toHaveBeenCalledTimes(1);
    });

    it('disables start button when isRunning is true', () => {
      render(<BatchSection {...defaultProps} isRunning />);

      const button = screen.getByRole('button', { name: '実行中' });
      expect(button).toBeDisabled();
    });

    it('disables start button when startDisabled is true', () => {
      render(<BatchSection {...defaultProps} startDisabled />);

      const button = screen.getByRole('button', { name: '開始' });
      expect(button).toBeDisabled();
    });

    it('renders cancel button when running and onCancel is provided', () => {
      const onCancel = vi.fn();

      render(<BatchSection {...defaultProps} isRunning onCancel={onCancel} />);

      expect(
        screen.getByRole('button', { name: 'キャンセル' })
      ).toBeInTheDocument();
    });

    it('does not render cancel button when not running', () => {
      const onCancel = vi.fn();

      render(<BatchSection {...defaultProps} onCancel={onCancel} />);

      expect(
        screen.queryByRole('button', { name: 'キャンセル' })
      ).not.toBeInTheDocument();
    });

    it('does not render cancel button when onCancel is not provided', () => {
      render(<BatchSection {...defaultProps} isRunning />);

      expect(
        screen.queryByRole('button', { name: 'キャンセル' })
      ).not.toBeInTheDocument();
    });

    it('calls onCancel when cancel button is clicked', async () => {
      const user = userEvent.setup();
      const onCancel = vi.fn();

      render(<BatchSection {...defaultProps} isRunning onCancel={onCancel} />);

      await user.click(screen.getByRole('button', { name: 'キャンセル' }));

      expect(onCancel).toHaveBeenCalledTimes(1);
    });

    it('renders custom cancel button label', () => {
      const onCancel = vi.fn();

      render(
        <BatchSection
          {...defaultProps}
          isRunning
          onCancel={onCancel}
          cancelLabel="中止"
        />
      );

      expect(screen.getByRole('button', { name: '中止' })).toBeInTheDocument();
    });

    it('renders cancel button with destructive variant', () => {
      const onCancel = vi.fn();

      render(
        <BatchSection
          {...defaultProps}
          isRunning
          onCancel={onCancel}
          cancelVariant="destructive"
        />
      );

      const cancelButton = screen.getByRole('button', { name: 'キャンセル' });
      expect(cancelButton).toHaveClass('bg-destructive');
    });

    it('renders cancel button with outline variant by default', () => {
      const onCancel = vi.fn();

      render(<BatchSection {...defaultProps} isRunning onCancel={onCancel} />);

      const cancelButton = screen.getByRole('button', { name: 'キャンセル' });
      expect(cancelButton).toHaveClass('border');
    });
  });

  describe('confirm dialog', () => {
    const confirmDialogConfig = {
      title: '確認',
      description: '本当に実行しますか?',
      confirmLabel: '実行する',
    };

    it('shows confirm dialog when start is clicked with confirmDialog config', async () => {
      const user = userEvent.setup();
      const onStart = vi.fn();

      render(
        <BatchSection
          {...defaultProps}
          onStart={onStart}
          confirmDialog={confirmDialogConfig}
        />
      );

      await user.click(screen.getByRole('button', { name: '開始' }));

      // Dialog should be shown
      expect(screen.getByText('確認')).toBeInTheDocument();
      expect(screen.getByText('本当に実行しますか?')).toBeInTheDocument();

      // onStart should not be called yet
      expect(onStart).not.toHaveBeenCalled();
    });

    it('calls onStart when confirm button is clicked in dialog', async () => {
      const user = userEvent.setup();
      const onStart = vi.fn();

      render(
        <BatchSection
          {...defaultProps}
          onStart={onStart}
          confirmDialog={confirmDialogConfig}
        />
      );

      // Open dialog
      await user.click(screen.getByRole('button', { name: '開始' }));

      // Click confirm button in dialog
      const confirmButton = screen.getByRole('button', { name: '実行する' });
      await user.click(confirmButton);

      expect(onStart).toHaveBeenCalledTimes(1);
    });

    it('closes dialog when cancel button is clicked in dialog', async () => {
      const user = userEvent.setup();
      const onStart = vi.fn();

      render(
        <BatchSection
          {...defaultProps}
          onStart={onStart}
          confirmDialog={confirmDialogConfig}
        />
      );

      // Open dialog
      await user.click(screen.getByRole('button', { name: '開始' }));

      // Click cancel button in dialog
      const cancelButton = screen.getAllByRole('button', {
        name: 'キャンセル',
      })[0];
      await user.click(cancelButton);

      // Dialog should be closed (title not visible)
      expect(screen.queryByText('確認')).not.toBeInTheDocument();
      expect(onStart).not.toHaveBeenCalled();
    });

    it('does not show dialog when confirmDialog is not provided', async () => {
      const user = userEvent.setup();
      const onStart = vi.fn();

      render(<BatchSection {...defaultProps} onStart={onStart} />);

      await user.click(screen.getByRole('button', { name: '開始' }));

      // onStart should be called directly
      expect(onStart).toHaveBeenCalledTimes(1);
      // No dialog shown
      expect(screen.queryByText('確認')).not.toBeInTheDocument();
    });
  });

  describe('progress display', () => {
    it('does not show progress card when not running and no progress', () => {
      render(<BatchSection {...defaultProps} />);

      expect(screen.queryByText('進捗')).not.toBeInTheDocument();
    });

    it('shows progress card when running with progress', () => {
      render(
        <BatchSection {...defaultProps} isRunning progress={mockProgress} />
      );

      expect(screen.getByText('進捗')).toBeInTheDocument();
      expect(screen.getByText('150 / 300 件')).toBeInTheDocument();
    });

    it('shows progress card when not running but progress exists', () => {
      render(<BatchSection {...defaultProps} progress={mockProgress} />);

      expect(screen.getByText('進捗')).toBeInTheDocument();
    });

    it('uses custom progressTitle when provided', () => {
      render(
        <BatchSection
          {...defaultProps}
          isRunning
          progress={mockProgress}
          progressTitle="カスタム進捗"
        />
      );

      expect(screen.getByText('カスタム進捗')).toBeInTheDocument();
    });

    it('passes showBatchNumber prop to BatchProgressBar', () => {
      render(
        <BatchSection
          {...defaultProps}
          isRunning
          progress={mockProgress}
          showBatchNumber
          showCounts={false}
        />
      );

      // BatchProgressBar with showBatchNumber shows "バッチ N"
      expect(screen.getByText('バッチ 1')).toBeInTheDocument();
      expect(screen.getByText('150 件処理済み')).toBeInTheDocument();
    });

    it('passes showCounts prop to BatchProgressBar', () => {
      render(
        <BatchSection
          {...defaultProps}
          isRunning
          progress={mockProgress}
          showCounts={false}
        />
      );

      // BatchProgressBar without showCounts doesn't show success/failed labels
      expect(screen.queryByText('成功:')).not.toBeInTheDocument();
      expect(screen.queryByText('失敗:')).not.toBeInTheDocument();
    });
  });

  describe('status badge', () => {
    it('does not show status badge when statusConfig is not provided', () => {
      render(<BatchSection {...defaultProps} />);

      expect(screen.queryByText('ステータス:')).not.toBeInTheDocument();
    });

    it('shows status badge when statusConfig is provided', () => {
      render(
        <BatchSection
          {...defaultProps}
          status="idle"
          statusConfig={SYNC_STATUS_CONFIG}
        />
      );

      expect(screen.getByText('ステータス:')).toBeInTheDocument();
      expect(screen.getByText('待機中')).toBeInTheDocument();
    });

    it('updates status badge based on status prop', () => {
      const { rerender } = render(
        <BatchSection
          {...defaultProps}
          status="idle"
          statusConfig={SYNC_STATUS_CONFIG}
        />
      );

      expect(screen.getByText('待機中')).toBeInTheDocument();

      rerender(
        <BatchSection
          {...defaultProps}
          status="syncing"
          statusConfig={SYNC_STATUS_CONFIG}
        />
      );

      expect(screen.getByText('同期中')).toBeInTheDocument();
    });
  });

  describe('extra content and statistics', () => {
    it('renders extra content in control card', () => {
      const extraContent = <div>追加コンテンツ</div>;

      render(<BatchSection {...defaultProps} extraContent={extraContent} />);

      expect(screen.getByText('追加コンテンツ')).toBeInTheDocument();
    });

    it('renders statistics section', () => {
      const statistics = <div data-testid="statistics">統計情報カード</div>;

      render(<BatchSection {...defaultProps} statistics={statistics} />);

      expect(screen.getByTestId('statistics')).toBeInTheDocument();
      expect(screen.getByText('統計情報カード')).toBeInTheDocument();
    });

    it('does not render statistics when not provided', () => {
      render(<BatchSection {...defaultProps} />);

      expect(screen.queryByText('統計情報カード')).not.toBeInTheDocument();
    });
  });

  describe('complete structure', () => {
    it('renders complete section with all elements', () => {
      const onCancel = vi.fn();
      const extraContent = <div>追加設定</div>;
      const statistics = <div data-testid="stats">統計</div>;

      render(
        <BatchSection
          {...defaultProps}
          isRunning
          progress={mockProgress}
          onCancel={onCancel}
          status="syncing"
          statusConfig={SYNC_STATUS_CONFIG}
          extraContent={extraContent}
          statistics={statistics}
          progressTitle="処理進捗"
        />
      );

      // Section title
      expect(screen.getByText('1. テストセクション')).toBeInTheDocument();

      // Control card
      expect(screen.getByText('テスト処理')).toBeInTheDocument();
      expect(screen.getByText('追加設定')).toBeInTheDocument();
      expect(
        screen.getByRole('button', { name: '実行中' })
      ).toBeInTheDocument();
      expect(
        screen.getByRole('button', { name: 'キャンセル' })
      ).toBeInTheDocument();

      // Status badge
      expect(screen.getByText('ステータス:')).toBeInTheDocument();
      expect(screen.getByText('同期中')).toBeInTheDocument();

      // Progress card
      expect(screen.getByText('処理進捗')).toBeInTheDocument();
      expect(screen.getByText('150 / 300 件')).toBeInTheDocument();

      // Statistics
      expect(screen.getByTestId('stats')).toBeInTheDocument();
    });
  });
});
