import { describe, it, expect, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useImageUrl } from './useImageUrl';

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://${path}`,
  isTauri: () => false,
}));

vi.mock('@tauri-apps/api/path', () => ({
  appDataDir: () => Promise.resolve('/app/data'),
  join: () => Promise.resolve('/app/data/images'),
}));

describe('useImageUrl', () => {
  it('returns a function', () => {
    const { result } = renderHook(() => useImageUrl());
    expect(typeof result.current).toBe('function');
  });

  it('returns null when not in Tauri (basePath stays null)', async () => {
    const { result } = renderHook(() => useImageUrl());
    await act(async () => {
      await new Promise((r) => setTimeout(r, 10));
    });
    expect(result.current('image.jpg')).toBeNull();
    expect(result.current('')).toBeNull();
    expect(result.current(null)).toBeNull();
  });
});
