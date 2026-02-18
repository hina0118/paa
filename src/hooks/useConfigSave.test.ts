import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useConfigSave } from './useConfigSave';

const { mockToastSuccess, mockToastError } = vi.hoisted(() => ({
  mockToastSuccess: vi.fn(),
  mockToastError: vi.fn(),
}));

vi.mock('@/lib/toast', () => ({
  toastSuccess: (...args: unknown[]) => mockToastSuccess(...args),
  toastError: (...args: unknown[]) => mockToastError(...args),
  formatError: (error: unknown) =>
    error instanceof Error ? error.message : String(error),
}));

describe('useConfigSave', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('初期状態は isSaving が false である', () => {
    const saveFn = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() => useConfigSave(saveFn, 'テスト'));

    expect(result.current.isSaving).toBe(false);
  });

  it('save 呼び出し中は isSaving が true になる', async () => {
    let resolve: (() => void) | null = null;
    const saveFn = vi.fn(
      () =>
        new Promise<void>((r) => {
          resolve = r;
        })
    );

    const { result } = renderHook(() => useConfigSave(saveFn, 'テスト'));

    let savePromise: Promise<void>;
    act(() => {
      savePromise = result.current.save();
    });

    expect(result.current.isSaving).toBe(true);

    await act(async () => {
      resolve?.();
      await savePromise;
    });

    expect(result.current.isSaving).toBe(false);
  });

  it('saveFn が成功したとき toastSuccess が呼ばれる', async () => {
    const saveFn = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() => useConfigSave(saveFn, 'バッチサイズ'));

    await act(async () => {
      await result.current.save();
    });

    expect(mockToastSuccess).toHaveBeenCalledWith('バッチサイズを更新しました');
    expect(mockToastError).not.toHaveBeenCalled();
    expect(result.current.isSaving).toBe(false);
  });

  it('saveFn が false を返したとき toastSuccess は呼ばれない', async () => {
    const saveFn = vi.fn().mockResolvedValue(false);
    const { result } = renderHook(() => useConfigSave(saveFn, 'バッチサイズ'));

    await act(async () => {
      await result.current.save();
    });

    expect(mockToastSuccess).not.toHaveBeenCalled();
    expect(mockToastError).not.toHaveBeenCalled();
    expect(result.current.isSaving).toBe(false);
  });

  it('saveFn が例外をスローしたとき toastError が呼ばれる', async () => {
    const saveFn = vi.fn().mockRejectedValue(new Error('DB接続エラー'));
    const { result } = renderHook(() => useConfigSave(saveFn, 'バッチサイズ'));

    await act(async () => {
      await result.current.save();
    });

    expect(mockToastError).toHaveBeenCalledWith(
      '更新に失敗しました: DB接続エラー'
    );
    expect(mockToastSuccess).not.toHaveBeenCalled();
    expect(result.current.isSaving).toBe(false);
  });

  it('saveFn が例外をスローしても isSaving が false に戻る', async () => {
    const saveFn = vi.fn().mockRejectedValue(new Error('error'));
    const { result } = renderHook(() => useConfigSave(saveFn, 'テスト'));

    await act(async () => {
      await result.current.save();
    });

    expect(result.current.isSaving).toBe(false);
  });

  it('save 関数の参照が安定している（useCallback）', () => {
    const saveFn = vi.fn().mockResolvedValue(undefined);
    const { result, rerender } = renderHook(() =>
      useConfigSave(saveFn, 'テスト')
    );
    const first = result.current.save;
    rerender();
    expect(result.current.save).toBe(first);
  });

  it('save が実行中のときに再度呼ばれても二重実行されない', async () => {
    let resolve: (() => void) | null = null;
    const saveFn = vi.fn(
      () =>
        new Promise<void>((r) => {
          resolve = r;
        })
    );

    const { result } = renderHook(() => useConfigSave(saveFn, 'テスト'));

    let firstSavePromise: Promise<void>;
    let secondSavePromise: Promise<void>;
    act(() => {
      firstSavePromise = result.current.save();
    });

    // 実行中に再度呼ぶ
    act(() => {
      secondSavePromise = result.current.save();
    });

    // saveFn は1回だけ呼ばれる
    expect(saveFn).toHaveBeenCalledTimes(1);

    // 両方のプロミスは同じものである
    expect(firstSavePromise).toBe(secondSavePromise);

    await act(async () => {
      resolve?.();
      await firstSavePromise;
    });

    expect(result.current.isSaving).toBe(false);
  });
});
