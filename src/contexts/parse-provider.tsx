import { useState, useEffect, useCallback, type ReactNode } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import {
  type ParseProgress,
  type ParseMetadata,
  type ProductNameParseProgress,
  ParseContext,
} from './parse-context-value';

export function ParseProvider({ children }: { children: ReactNode }) {
  const [isParsing, setIsParsing] = useState(false);
  const [progress, setProgress] = useState<ParseProgress | null>(null);
  const [metadata, setMetadata] = useState<ParseMetadata | null>(null);
  // 商品名パース (Gemini API)
  const [isProductNameParsing, setIsProductNameParsing] = useState(false);
  const [productNameProgress, setProductNameProgress] =
    useState<ProductNameParseProgress | null>(null);

  const refreshStatus = useCallback(async () => {
    try {
      const status = await invoke<ParseMetadata>('get_parse_status');
      setMetadata(status);
      setIsParsing(status.parse_status === 'running');
    } catch (error) {
      console.error('Failed to fetch parse status:', error);
    }
  }, []);

  useEffect(() => {
    const unlisten = listen<ParseProgress>('parse-progress', (event) => {
      const data = event.payload;
      setProgress(data);

      if (data.is_complete) {
        setIsParsing(false);
        refreshStatus();
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [refreshStatus]);

  // 商品名パース進捗イベントをリッスン
  useEffect(() => {
    const unlisten = listen<ProductNameParseProgress>(
      'product-name-parse-progress',
      (event) => {
        const data = event.payload;
        setProductNameProgress(data);

        if (data.is_complete) {
          setIsProductNameParsing(false);
        }
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

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
      }}
    >
      {children}
    </ParseContext.Provider>
  );
}
