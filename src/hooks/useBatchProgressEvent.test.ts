import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useBatchProgressEvent } from './useBatchProgressEvent';
import { mockListen } from '@/test/setup';
import {
  BATCH_PROGRESS_EVENT,
  TASK_NAMES,
  type BatchProgress,
} from '@/contexts/batch-progress-types';

function makeBatchProgress(
  overrides: Partial<BatchProgress> = {}
): BatchProgress {
  return {
    task_name: TASK_NAMES.GMAIL_SYNC,
    batch_number: 1,
    batch_size: 50,
    total_items: 100,
    processed_count: 50,
    success_count: 45,
    failed_count: 5,
    progress_percent: 50,
    status_message: 'In progress',
    is_complete: false,
    ...overrides,
  };
}

describe('useBatchProgressEvent', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockListen.mockResolvedValue(() => {});
  });

  const setupListener = () => {
    let progressCallback:
      | ((e: { payload: BatchProgress }) => Promise<void>)
      | null = null;

    mockListen.mockImplementation((event: string, cb: (e: unknown) => void) => {
      if (event === BATCH_PROGRESS_EVENT) {
        progressCallback = cb as (e: {
          payload: BatchProgress;
        }) => Promise<void>;
      }
      return Promise.resolve(() => {});
    });

    return {
      getProgressCallback: () => progressCallback,
    };
  };

  it('ignores events with non-matching task_name', async () => {
    const { getProgressCallback } = setupListener();
    const onComplete = vi.fn().mockResolvedValue(undefined);

    const { result } = renderHook(() =>
      useBatchProgressEvent(TASK_NAMES.GMAIL_SYNC, onComplete)
    );

    await waitFor(() => expect(getProgressCallback()).not.toBeNull());

    await act(async () => {
      await getProgressCallback()?.({
        payload: makeBatchProgress({
          task_name: TASK_NAMES.EMAIL_PARSE,
          is_complete: true,
        }),
      });
    });

    expect(result.current.progress).toBeNull();
    expect(onComplete).not.toHaveBeenCalled();
  });

  it('updates progress state on matching event', async () => {
    const { getProgressCallback } = setupListener();
    const onComplete = vi.fn().mockResolvedValue(undefined);

    const { result } = renderHook(() =>
      useBatchProgressEvent(TASK_NAMES.GMAIL_SYNC, onComplete)
    );

    await waitFor(() => expect(getProgressCallback()).not.toBeNull());

    const progressData = makeBatchProgress({
      processed_count: 75,
      progress_percent: 75,
    });
    await act(async () => {
      await getProgressCallback()?.({ payload: progressData });
    });

    expect(result.current.progress).toEqual(progressData);
  });

  it('calls onComplete when is_complete is true', async () => {
    const { getProgressCallback } = setupListener();
    const onComplete = vi.fn().mockResolvedValue(undefined);

    renderHook(() => useBatchProgressEvent(TASK_NAMES.GMAIL_SYNC, onComplete));

    await waitFor(() => expect(getProgressCallback()).not.toBeNull());

    const completeData = makeBatchProgress({
      is_complete: true,
      processed_count: 100,
      progress_percent: 100,
    });
    await act(async () => {
      await getProgressCallback()?.({ payload: completeData });
    });

    expect(onComplete).toHaveBeenCalledWith(completeData);
  });

  it('does not call onComplete when is_complete is false', async () => {
    const { getProgressCallback } = setupListener();
    const onComplete = vi.fn().mockResolvedValue(undefined);

    renderHook(() => useBatchProgressEvent(TASK_NAMES.GMAIL_SYNC, onComplete));

    await waitFor(() => expect(getProgressCallback()).not.toBeNull());

    await act(async () => {
      await getProgressCallback()?.({
        payload: makeBatchProgress({ is_complete: false }),
      });
    });

    expect(onComplete).not.toHaveBeenCalled();
  });

  it('unlistens on unmount', async () => {
    const unlistenFn = vi.fn();
    mockListen.mockResolvedValue(unlistenFn);
    const onComplete = vi.fn().mockResolvedValue(undefined);

    const { unmount } = renderHook(() =>
      useBatchProgressEvent(TASK_NAMES.GMAIL_SYNC, onComplete)
    );

    unmount();

    await waitFor(() => expect(unlistenFn).toHaveBeenCalled());
  });

  it('allows caller to reset progress via setProgress', async () => {
    const { getProgressCallback } = setupListener();
    const onComplete = vi.fn().mockResolvedValue(undefined);

    const { result } = renderHook(() =>
      useBatchProgressEvent(TASK_NAMES.GMAIL_SYNC, onComplete)
    );

    await waitFor(() => expect(getProgressCallback()).not.toBeNull());

    // Set progress via event
    await act(async () => {
      await getProgressCallback()?.({
        payload: makeBatchProgress({ processed_count: 50 }),
      });
    });
    expect(result.current.progress).not.toBeNull();

    // Reset via setProgress
    act(() => {
      result.current.setProgress(null);
    });
    expect(result.current.progress).toBeNull();
  });
});
