import { useState, useEffect, useCallback, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { toastSuccess, toastError } from '@/lib/toast';
import { notify, isAppWindowVisible } from '@/lib/utils';
import {
  FullParsePipelineContext,
  PIPELINE_STEP_LABELS,
  type PipelineStep,
} from './full-parse-pipeline-context-value';

export function FullParsePipelineProvider({
  children,
}: {
  children: ReactNode;
}) {
  const [isRunning, setIsRunning] = useState(false);
  const [currentStep, setCurrentStep] = useState<PipelineStep | null>(null);

  const handleComplete = useCallback(async () => {
    setIsRunning(false);
    setCurrentStep(null);
    const visible = await isAppWindowVisible();
    if (visible) {
      toastSuccess('一括パースが完了しました');
    } else {
      try {
        await notify('一括パース完了', '全ステップが完了しました');
      } catch (error) {
        console.error(
          'Failed to send full parse pipeline notification:',
          error
        );
      }
    }
  }, []);

  useEffect(() => {
    let unlistenStep: (() => void) | null = null;
    let unlistenComplete: (() => void) | null = null;
    let isActive = true;

    listen<PipelineStep>('full-parse:step_started', (event) => {
      if (!isActive) return;
      setIsRunning(true);
      setCurrentStep(event.payload);
    })
      .then((fn) => {
        if (!isActive) fn();
        else unlistenStep = fn;
      })
      .catch((e) => {
        console.error('Failed to set up full-parse:step_started listener:', e);
      });

    listen<void>('full-parse:complete', () => {
      if (!isActive) return;
      handleComplete();
    })
      .then((fn) => {
        if (!isActive) fn();
        else unlistenComplete = fn;
      })
      .catch((e) => {
        console.error('Failed to set up full-parse:complete listener:', e);
      });

    return () => {
      isActive = false;
      unlistenStep?.();
      unlistenComplete?.();
    };
  }, [handleComplete]);

  const startPipeline = async () => {
    try {
      setIsRunning(true);
      setCurrentStep(null);
      await invoke('start_full_parse_pipeline');
    } catch (error) {
      setIsRunning(false);
      setCurrentStep(null);
      const message = error instanceof Error ? error.message : String(error);
      toastError('一括パースの開始に失敗しました', message);
      throw error;
    }
  };

  return (
    <FullParsePipelineContext.Provider
      value={{ isRunning, currentStep, startPipeline }}
    >
      {children}
    </FullParsePipelineContext.Provider>
  );
}

export { PIPELINE_STEP_LABELS };
