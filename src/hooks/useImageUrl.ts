import { useCallback, useEffect, useState } from 'react';
import { appDataDir, join } from '@tauri-apps/api/path';
import { convertFileSrc, isTauri } from '@tauri-apps/api/core';

let cachedBasePath: string | null = null;
let cachedPromise: Promise<string> | null = null;

function getImagesBasePath(): Promise<string> {
  if (cachedBasePath !== null) {
    return Promise.resolve(cachedBasePath);
  }
  if (!cachedPromise) {
    cachedPromise = (async () => {
      try {
        const appData = await appDataDir();
        const basePath = await join(appData, 'images');
        cachedBasePath = basePath;
        return basePath;
      } finally {
        cachedPromise = null;
      }
    })();
  }
  return cachedPromise;
}

/**
 * Returns a function to convert image file name to displayable URL.
 * Uses convertFileSrc for Tauri asset protocol.
 * Returns null when not in Tauri or when fileName is empty.
 */
export function useImageUrl() {
  const [basePath, setBasePath] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;
    getImagesBasePath()
      .then((path) => {
        if (!cancelled) setBasePath(path);
      })
      .catch(() => {
        if (!cancelled) setBasePath(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const getImageUrl = useCallback(
    (fileName: string | null | undefined): string | null => {
      if (!fileName?.trim() || !basePath) return null;
      // パストラバーサル防止: 区切り文字や '..' を含む場合は拒否
      const sanitized = fileName.trim();
      if (/[/\\]|\.\./.test(sanitized)) return null;
      try {
        const separator = basePath.includes('\\') ? '\\' : '/';
        const fullPath = `${basePath.replace(/[/\\]$/, '')}${separator}${sanitized}`;
        return convertFileSrc(fullPath);
      } catch {
        return null;
      }
    },
    [basePath]
  );

  return getImageUrl;
}
