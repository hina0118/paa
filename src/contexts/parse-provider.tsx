import { useState, useEffect, useCallback, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { type ParseMetadata, ParseContext } from './parse-context-value';
import { type BatchProgress, TASK_NAMES } from './batch-progress-types';
import { useBatchNotification } from '@/hooks/useBatchNotification';
import { useBatchProgressEvent } from '@/hooks/useBatchProgressEvent';

const buildCountMessage = (data: BatchProgress) =>
  `成功: ${data.success_count}件、失敗: ${data.failed_count}件`;

export function ParseProvider({ children }: { children: ReactNode }) {
  const [isParsing, setIsParsing] = useState(false);
  const [metadata, setMetadata] = useState<ParseMetadata | null>(null);
  // 商品名パース (Gemini API)
  const [isProductNameParsing, setIsProductNameParsing] = useState(false);
  const [geminiApiKeyStatus, setGeminiApiKeyStatus] = useState<
    'checking' | 'available' | 'unavailable' | 'error'
  >('checking');

  const refreshGeminiApiKeyStatus = useCallback(async () => {
    setGeminiApiKeyStatus('checking');
    try {
      const has = await invoke<boolean>('has_gemini_api_key');
      setGeminiApiKeyStatus(has ? 'available' : 'unavailable');
    } catch (error) {
      console.error('Failed to fetch Gemini API key status:', error);
      setGeminiApiKeyStatus('error');
    }
  }, []);

  const refreshStatus = useCallback(async () => {
    try {
      const status = await invoke<ParseMetadata>('get_parse_status');
      setMetadata(status);
      setIsParsing(status.parse_status === 'running');
    } catch (error) {
      console.error('Failed to fetch parse status:', error);
    }
  }, []);

  const notifyEmailParse = useBatchNotification(
    'メールパース',
    buildCountMessage,
    'email parse'
  );

  const notifyProductNameParse = useBatchNotification(
    '商品名解析',
    buildCountMessage,
    'product name parse'
  );

  const handleEmailParseComplete = useCallback(
    async (data: BatchProgress) => {
      setIsParsing(false);
      refreshStatus();
      await notifyEmailParse(data);
    },
    [refreshStatus, notifyEmailParse]
  );

  const handleProductNameParseComplete = useCallback(
    async (data: BatchProgress) => {
      setIsProductNameParsing(false);
      await notifyProductNameParse(data);
    },
    [notifyProductNameParse]
  );

  const { progress, setProgress } = useBatchProgressEvent(
    TASK_NAMES.EMAIL_PARSE,
    handleEmailParseComplete
  );

  const { progress: productNameProgress, setProgress: setProductNameProgress } =
    useBatchProgressEvent(
      TASK_NAMES.PRODUCT_NAME_PARSE,
      handleProductNameParseComplete
    );

  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

  useEffect(() => {
    refreshGeminiApiKeyStatus();
  }, [refreshGeminiApiKeyStatus]);

  const startParse = async (batchSize?: number) => {
    try {
      setIsParsing(true);
      setProgress(null);
      await invoke('start_batch_parse', { batchSize });
    } catch (error) {
      setIsParsing(false);
      throw error;
    }
  };

  const cancelParse = async () => {
    try {
      await invoke('cancel_parse');
    } catch (error) {
      console.error('Failed to cancel parse:', error);
      throw error;
    }
  };

  const updateBatchSize = async (size: number) => {
    try {
      await invoke('update_parse_batch_size', { batchSize: size });
      await refreshStatus();
    } catch (error) {
      console.error('Failed to update batch size:', error);
      throw error;
    }
  };

  const startProductNameParse = async () => {
    try {
      setIsProductNameParsing(true);
      setProductNameProgress(null);
      await invoke('start_product_name_parse');
    } catch (error) {
      setIsProductNameParsing(false);
      throw error;
    }
  };

  return (
    <ParseContext.Provider
      value={{
        isParsing,
        progress,
        metadata,
        startParse,
        cancelParse,
        refreshStatus,
        updateBatchSize,
        isProductNameParsing,
        productNameProgress,
        startProductNameParse,
        geminiApiKeyStatus,
        hasGeminiApiKey: geminiApiKeyStatus === 'available',
        refreshGeminiApiKeyStatus,
      }}
    >
      {children}
    </ParseContext.Provider>
  );
}
