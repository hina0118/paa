import { useState, useCallback, useRef } from 'react';
import { toastSuccess, toastError, formatError } from '@/lib/toast';

export interface UseConfigSaveResult {
  isSaving: boolean;
  save: () => Promise<void>;
}

/**
 * 設定値の保存処理を共通化するカスタムフック
 *
 * ## 責務
 * - 保存中フラグ (`isSaving`) の管理
 * - 保存成功時のトースト表示
 * - 保存失敗時のエラートースト表示
 *
 * @param saveFn - バリデーションと実際の保存処理を行う非同期関数。
 *                 バリデーション失敗時は `false` を返すこと。
 *                 保存成功時は `true` (または `undefined`) を返すこと。
 * @param label  - 成功トースト表示に使うラベル (例: 'バッチサイズ')
 */
export function useConfigSave(
  saveFn: () => Promise<boolean | void>,
  label: string
): UseConfigSaveResult {
  const [isSaving, setIsSaving] = useState(false);
  const inFlightPromiseRef = useRef<Promise<void> | null>(null);

  const save = useCallback(() => {
    if (inFlightPromiseRef.current) {
      return inFlightPromiseRef.current;
    }

    const saveOperation = (async () => {
      setIsSaving(true);
      try {
        const result = await saveFn();
        if (result !== false) {
          toastSuccess(`${label}を更新しました`);
        }
      } catch (error) {
        toastError(`更新に失敗しました: ${formatError(error)}`);
      } finally {
        setIsSaving(false);
        inFlightPromiseRef.current = null;
      }
    })();

    inFlightPromiseRef.current = saveOperation;
    return saveOperation;
  }, [saveFn, label]);

  return { isSaving, save };
}
