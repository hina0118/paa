/**
 * バッチ処理進捗の共通型定義
 *
 * バックエンドの BatchProgressEvent に対応するフロントエンド型
 * すべてのバッチ処理（メール同期、メールパース、商品名パース）で使用されます
 */

/**
 * バッチ処理の進捗情報
 */
export interface BatchProgress {
  /** タスク名（"メール同期", "メールパース", "商品名パース" など） */
  task_name: string;
  /** 現在のバッチ番号（1から開始） */
  batch_number: number;
  /** このバッチで処理した件数 */
  batch_size: number;
  /** 全体の処理対象件数 */
  total_items: number;
  /** これまでに処理した件数 */
  processed_count: number;
  /** 成功件数 */
  success_count: number;
  /** 失敗件数 */
  failed_count: number;
  /** 進捗率（0.0 ~ 100.0） */
  progress_percent: number;
  /** 状態メッセージ */
  status_message: string;
  /** 処理完了フラグ */
  is_complete: boolean;
  /** エラーメッセージ（エラー時のみ） */
  error?: string;
}

/**
 * タスク名の定数
 */
export const TASK_NAMES = {
  GMAIL_SYNC: 'メール同期',
  EMAIL_PARSE: 'メールパース',
  PRODUCT_NAME_PARSE: '商品名パース',
} as const;

/**
 * タスク名の型
 */
export type TaskName = (typeof TASK_NAMES)[keyof typeof TASK_NAMES];

/**
 * 特定のタスクの進捗かどうかを判定するヘルパー関数
 */
export function isTaskProgress(
  progress: BatchProgress,
  taskName: TaskName
): boolean {
  return progress.task_name === taskName;
}

/**
 * 進捗イベント名（すべてのバッチ処理で共通）
 */
export const BATCH_PROGRESS_EVENT = 'batch-progress';
