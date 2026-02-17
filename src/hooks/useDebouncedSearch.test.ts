import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useDebouncedSearch } from './useDebouncedSearch';

describe('useDebouncedSearch', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('initializes with empty strings', () => {
    const { result } = renderHook(() => useDebouncedSearch());

    expect(result.current.searchInput).toBe('');
    expect(result.current.searchDebounced).toBe('');
  });

  it('updates searchInput immediately on setSearchInput', () => {
    const { result } = renderHook(() => useDebouncedSearch());

    act(() => {
      result.current.setSearchInput('hello');
    });

    expect(result.current.searchInput).toBe('hello');
    expect(result.current.searchDebounced).toBe('');
  });

  it('updates searchDebounced after delay', () => {
    const { result } = renderHook(() => useDebouncedSearch(300));

    act(() => {
      result.current.setSearchInput('hello');
    });

    // Before delay
    expect(result.current.searchDebounced).toBe('');

    // After delay
    act(() => {
      vi.advanceTimersByTime(300);
    });

    expect(result.current.searchDebounced).toBe('hello');
  });

  it('resets debounce timer on rapid input', () => {
    const { result } = renderHook(() => useDebouncedSearch(300));

    act(() => {
      result.current.setSearchInput('h');
    });

    act(() => {
      vi.advanceTimersByTime(200);
    });

    act(() => {
      result.current.setSearchInput('he');
    });

    act(() => {
      vi.advanceTimersByTime(200);
    });

    // 400ms total but only 200ms since last input
    expect(result.current.searchDebounced).toBe('');

    act(() => {
      vi.advanceTimersByTime(100);
    });

    expect(result.current.searchDebounced).toBe('he');
  });

  it('clearSearch resets both values synchronously', () => {
    const { result } = renderHook(() => useDebouncedSearch(300));

    act(() => {
      result.current.setSearchInput('hello');
    });

    act(() => {
      vi.advanceTimersByTime(300);
    });

    expect(result.current.searchInput).toBe('hello');
    expect(result.current.searchDebounced).toBe('hello');

    act(() => {
      result.current.clearSearch();
    });

    expect(result.current.searchInput).toBe('');
    expect(result.current.searchDebounced).toBe('');
  });

  it('uses custom delay', () => {
    const { result } = renderHook(() => useDebouncedSearch(500));

    act(() => {
      result.current.setSearchInput('test');
    });

    act(() => {
      vi.advanceTimersByTime(300);
    });

    expect(result.current.searchDebounced).toBe('');

    act(() => {
      vi.advanceTimersByTime(200);
    });

    expect(result.current.searchDebounced).toBe('test');
  });

  it('uses default delay of 300ms when not specified', () => {
    const { result } = renderHook(() => useDebouncedSearch());

    act(() => {
      result.current.setSearchInput('test');
    });

    act(() => {
      vi.advanceTimersByTime(299);
    });

    expect(result.current.searchDebounced).toBe('');

    act(() => {
      vi.advanceTimersByTime(1);
    });

    expect(result.current.searchDebounced).toBe('test');
  });

  it('clearSearch prevents pending debounce from firing', () => {
    const { result } = renderHook(() => useDebouncedSearch(300));

    act(() => {
      result.current.setSearchInput('hello');
    });

    // Clear before debounce fires
    act(() => {
      result.current.clearSearch();
    });

    act(() => {
      vi.advanceTimersByTime(300);
    });

    expect(result.current.searchInput).toBe('');
    expect(result.current.searchDebounced).toBe('');
  });
});
