import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { BatchProgressBar } from './batch-progress-bar';
import type { BatchProgress } from '@/contexts/batch-progress-types';

describe('BatchProgressBar', () => {
  const mockProgress: BatchProgress = {
    task_name: 'メールパース',
    batch_number: 2,
    batch_size: 100,
    total_items: 500,
    processed_count: 200,
    success_count: 195,
    failed_count: 5,
    progress_percent: 40,
    status_message: 'バッチ 2 を処理中...',
    is_complete: false,
  };

  it('renders progress information correctly', () => {
    render(<BatchProgressBar progress={mockProgress} />);

    // 件数表示
    expect(screen.getByText('200 / 500 件')).toBeInTheDocument();
    // パーセント表示
    expect(screen.getByText('40%')).toBeInTheDocument();
    // 成功件数
    expect(screen.getByText('195')).toBeInTheDocument();
    // 失敗件数
    expect(screen.getByText('5')).toBeInTheDocument();
    // ステータスメッセージ
    expect(screen.getByText('バッチ 2 を処理中...')).toBeInTheDocument();
  });

  it('shows completion message when complete', () => {
    const completedProgress: BatchProgress = {
      ...mockProgress,
      is_complete: true,
      status_message: '処理完了',
    };

    render(
      <BatchProgressBar
        progress={completedProgress}
        completeMessage="パースが完了しました"
      />
    );

    expect(screen.getByText('パースが完了しました')).toBeInTheDocument();
    expect(screen.getByRole('status')).toBeInTheDocument();
  });

  it('shows error message when error exists', () => {
    const errorProgress: BatchProgress = {
      ...mockProgress,
      is_complete: true,
      error: 'API接続エラー',
    };

    render(<BatchProgressBar progress={errorProgress} />);

    expect(screen.getByText('API接続エラー')).toBeInTheDocument();
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('hides counts when showCounts is false', () => {
    render(<BatchProgressBar progress={mockProgress} showCounts={false} />);

    // 成功/失敗のラベルが表示されない
    expect(screen.queryByText('成功:')).not.toBeInTheDocument();
    expect(screen.queryByText('失敗:')).not.toBeInTheDocument();
  });

  it('shows batch number when showBatchNumber is true', () => {
    render(
      <BatchProgressBar
        progress={mockProgress}
        showBatchNumber
        showCounts={false}
      />
    );

    // バッチ番号
    expect(screen.getByText('バッチ 2')).toBeInTheDocument();
    // 処理件数
    expect(screen.getByText('200 件処理済み')).toBeInTheDocument();
    // 件数/総数表示は非表示
    expect(screen.queryByText('200 / 500 件')).not.toBeInTheDocument();
    // 成功/失敗のラベルが表示されない
    expect(screen.queryByText('成功:')).not.toBeInTheDocument();
    expect(screen.queryByText('失敗:')).not.toBeInTheDocument();
    // ステータスメッセージ
    expect(screen.getByText('バッチ 2 を処理中...')).toBeInTheDocument();
  });

  it('shows completion message with showBatchNumber', () => {
    const completedProgress: BatchProgress = {
      ...mockProgress,
      is_complete: true,
      status_message: '同期完了',
    };

    render(
      <BatchProgressBar
        progress={completedProgress}
        showBatchNumber
        showCounts={false}
        completeMessage="同期が完了しました"
      />
    );

    expect(screen.getByText('同期が完了しました')).toBeInTheDocument();
    expect(screen.getByRole('status')).toBeInTheDocument();
  });
});
