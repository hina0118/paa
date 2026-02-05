import { createContext } from 'react';
import type { BatchProgress } from './batch-progress-types';

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
  /** 共通の進捗型 */
  progress: BatchProgress | null;
  metadata: ParseMetadata | null;
  startParse: (batchSize?: number) => Promise<void>;
  cancelParse: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  updateBatchSize: (size: number) => Promise<void>;
  // 商品名パース (Gemini API)
  isProductNameParsing: boolean;
  /** 共通の進捗型（商品名パース用） */
  productNameProgress: BatchProgress | null;
  startProductNameParse: () => Promise<void>;
  geminiApiKeyStatus: 'checking' | 'available' | 'unavailable' | 'error';
  hasGeminiApiKey: boolean; // geminiApiKeyStatus === 'available' のエイリアス
  refreshGeminiApiKeyStatus: () => Promise<void>;
}

export const ParseContext = createContext<ParseContextType | undefined>(
  undefined
);
