import { createContext } from 'react';
import type { BatchProgress } from './batch-progress-types';

/**
 * パース進捗（後方互換性のため残す）
 * @deprecated 新しいコードでは BatchProgress を使用してください
 */
export interface ParseProgress {
  batch_number: number;
  total_emails: number;
  parsed_count: number;
  success_count: number;
  failed_count: number;
  status_message: string;
  is_complete: boolean;
  error?: string;
}

/**
 * 商品名パース進捗（後方互換性のため残す）
 * @deprecated 新しいコードでは BatchProgress を使用してください
 */
export interface ProductNameParseProgress {
  total_items: number;
  parsed_count: number;
  success_count: number;
  failed_count: number;
  status_message: string;
  is_complete: boolean;
  error?: string;
}

/**
 * BatchProgress から ParseProgress への変換ヘルパー（後方互換性用）
 */
export function batchProgressToParseProgress(bp: BatchProgress): ParseProgress {
  return {
    batch_number: bp.batch_number,
    total_emails: bp.total_items,
    parsed_count: bp.processed_count,
    success_count: bp.success_count,
    failed_count: bp.failed_count,
    status_message: bp.status_message,
    is_complete: bp.is_complete,
    error: bp.error,
  };
}

/**
 * BatchProgress から ProductNameParseProgress への変換ヘルパー（後方互換性用）
 */
export function batchProgressToProductNameParseProgress(
  bp: BatchProgress
): ProductNameParseProgress {
  return {
    total_items: bp.total_items,
    parsed_count: bp.processed_count,
    success_count: bp.success_count,
    failed_count: bp.failed_count,
    status_message: bp.status_message,
    is_complete: bp.is_complete,
    error: bp.error,
  };
}

export interface ParseMetadata {
  parse_status: 'idle' | 'running' | 'completed' | 'error';
  last_parse_started_at?: string;
  last_parse_completed_at?: string;
  total_parsed_count: number;
  last_error_message?: string;
  batch_size: number;
}

export interface ParseContextType {
  isParsing: boolean;
  /** @deprecated 新しいコードでは batchProgress を使用してください */
  progress: ParseProgress | null;
  /** 共通の進捗型 */
  batchProgress: BatchProgress | null;
  metadata: ParseMetadata | null;
  startParse: (batchSize?: number) => Promise<void>;
  cancelParse: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  updateBatchSize: (size: number) => Promise<void>;
  // 商品名パース (Gemini API)
  isProductNameParsing: boolean;
  /** @deprecated 新しいコードでは productNameBatchProgress を使用してください */
  productNameProgress: ProductNameParseProgress | null;
  /** 共通の進捗型（商品名パース用） */
  productNameBatchProgress: BatchProgress | null;
  startProductNameParse: () => Promise<void>;
  geminiApiKeyStatus: 'checking' | 'available' | 'unavailable' | 'error';
  hasGeminiApiKey: boolean; // geminiApiKeyStatus === 'available' のエイリアス
  refreshGeminiApiKeyStatus: () => Promise<void>;
}

export const ParseContext = createContext<ParseContextType | undefined>(
  undefined
);
