import { toast as sonnerToast } from 'sonner';

/**
 * エラーをユーザー向けメッセージ文字列に変換する
 */
export function formatError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * 成功トーストを表示する
 */
export function toastSuccess(message: string, description?: string): void {
  sonnerToast.success(message, description ? { description } : undefined);
}

/**
 * エラートーストを表示する
 */
export function toastError(message: string, description?: string): void {
  sonnerToast.error(message, description ? { description } : undefined);
}

/**
 * 警告トーストを表示する
 */
export function toastWarning(message: string, description?: string): void {
  sonnerToast.warning(message, description ? { description } : undefined);
}

/**
 * 情報トーストを表示する
 */
export function toastInfo(message: string, description?: string): void {
  sonnerToast.info(message, description ? { description } : undefined);
}
