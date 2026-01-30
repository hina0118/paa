import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useImageUrl } from './useImageUrl';
import * as tauriCore from '@tauri-apps/api/core';
import * as tauriPath from '@tauri-apps/api/path';

vi.mock('@tauri-apps/api/core');
vi.mock('@tauri-apps/api/path');

describe('useImageUrl', () => {
  beforeEach(() => {
    vi.mocked(tauriCore.isTauri).mockReturnValue(false);
    vi.mocked(tauriCore.convertFileSrc).mockImplementation(
      (path: string) => `asset://${path}`
    );
    vi.mocked(tauriPath.appDataDir).mockResolvedValue('/app/data');
    vi.mocked(tauriPath.join).mockImplementation((a: string, b: string) =>
      Promise.resolve(`${a}/${b}`)
    );
  });

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

  it('returns URL when in Tauri and fileName is valid', async () => {
    vi.mocked(tauriCore.isTauri).mockReturnValue(true);
    vi.mocked(tauriPath.join).mockResolvedValue('C:\\app\\data\\images');

    const { result } = renderHook(() => useImageUrl());
    await act(async () => {
      await new Promise((r) => setTimeout(r, 10));
    });

    expect(result.current('image.jpg')).toBe(
      'asset://C:\\app\\data\\images\\image.jpg'
    );
    expect(tauriCore.convertFileSrc).toHaveBeenCalledWith(
      'C:\\app\\data\\images\\image.jpg'
    );
  });

  it('returns null for path traversal attempts', async () => {
    vi.mocked(tauriCore.isTauri).mockReturnValue(true);
    vi.mocked(tauriPath.join).mockResolvedValue('/app/data/images');

    const { result } = renderHook(() => useImageUrl());
    await act(async () => {
      await new Promise((r) => setTimeout(r, 10));
    });

    expect(result.current('../evil')).toBeNull();
    expect(result.current('path/to/file')).toBeNull();
    expect(result.current('..\\windows')).toBeNull();
  });

  it('returns null when convertFileSrc throws', async () => {
    vi.mocked(tauriCore.isTauri).mockReturnValue(true);
    vi.mocked(tauriPath.join).mockResolvedValue('/app/data/images');
    vi.mocked(tauriCore.convertFileSrc).mockImplementation(() => {
      throw new Error('convert failed');
    });

    const { result } = renderHook(() => useImageUrl());
    await act(async () => {
      await new Promise((r) => setTimeout(r, 10));
    });

    expect(result.current('image.jpg')).toBeNull();
  });

  it('returns null when getImagesBasePath rejects', async () => {
    vi.resetModules();
    vi.mocked(tauriCore.isTauri).mockReturnValue(true);
    vi.mocked(tauriPath.appDataDir).mockRejectedValue(new Error('path error'));

    const { useImageUrl: useImageUrlFresh } = await import('./useImageUrl');
    const { result } = renderHook(() => useImageUrlFresh());
    await act(async () => {
      await new Promise((r) => setTimeout(r, 10));
    });

    expect(result.current('image.jpg')).toBeNull();
  });
});
