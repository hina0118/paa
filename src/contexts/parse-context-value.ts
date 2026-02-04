import { createContext } from 'react';

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

export interface ProductNameParseProgress {
  total_items: number;
  parsed_count: number;
  success_count: number;
  failed_count: number;
  status_message: string;
  is_complete: boolean;
  error?: string;
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
  progress: ParseProgress | null;
  metadata: ParseMetadata | null;
  startParse: (batchSize?: number) => Promise<void>;
  cancelParse: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  updateBatchSize: (size: number) => Promise<void>;
  // 商品名パース (Gemini API)
  isProductNameParsing: boolean;
  productNameProgress: ProductNameParseProgress | null;
  startProductNameParse: () => Promise<void>;
  geminiApiKeyStatus: 'checking' | 'available' | 'unavailable' | 'error';
  hasGeminiApiKey: boolean; // geminiApiKeyStatus === 'available' のエイリアス
  refreshGeminiApiKeyStatus: () => Promise<void>;
}

export const ParseContext = createContext<ParseContextType | undefined>(
  undefined
);
