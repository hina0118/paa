import { useState, useCallback, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { DeliveryCheckContext } from './delivery-check-context-value';
import { type BatchProgress, TASK_NAMES } from './batch-progress-types';
import { useBatchNotification } from '@/hooks/useBatchNotification';
import { useBatchProgressEvent } from '@/hooks/useBatchProgressEvent';

const buildResultMessage = (data: BatchProgress) =>
  `成功: ${data.success_count}件、失敗: ${data.failed_count}件`;

export function DeliveryCheckProvider({ children }: { children: ReactNode }) {
  const [isChecking, setIsChecking] = useState(false);

  const notifyComplete = useBatchNotification(
    '配送状況確認',
    buildResultMessage,
    'delivery check'
  );

  const handleComplete = useCallback(
    async (data: BatchProgress) => {
      setIsChecking(false);
      await notifyComplete(data);
    },
    [notifyComplete]
  );

  const { progress, setProgress } = useBatchProgressEvent(
    TASK_NAMES.DELIVERY_CHECK,
    handleComplete
  );

  const startDeliveryCheck = async () => {
    setIsChecking(true);
    setProgress(null);
    try {
      await invoke('start_delivery_check');
    } catch (error) {
      setIsChecking(false);
      throw error;
    }
  };

  const cancelDeliveryCheck = async () => {
    try {
      await invoke('cancel_delivery_check');
    } catch (error) {
      console.error('Failed to cancel delivery check:', error);
      throw error;
    }
  };

  return (
    <DeliveryCheckContext.Provider
      value={{ isChecking, progress, startDeliveryCheck, cancelDeliveryCheck }}
    >
      {children}
    </DeliveryCheckContext.Provider>
  );
}
