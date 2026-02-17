import { useState, useEffect, useCallback } from 'react';

const DEFAULT_DELAY_MS = 300;

/**
 * 検索入力のデバウンスを管理する汎用フック
 *
 * - `searchInput` はユーザーの入力に即座に追従する（UI表示用）
 * - `searchDebounced` は入力停止後 `delayMs` ミリ秒で更新される（クエリ用）
 * - `clearSearch()` は両方を同期的にリセットする
 *
 * @param delayMs - デバウンス遅延（ミリ秒）。デフォルト 300ms
 */
export function useDebouncedSearch(delayMs: number = DEFAULT_DELAY_MS) {
  const [searchInput, setSearchInput] = useState('');
  const [searchDebounced, setSearchDebounced] = useState('');

  useEffect(() => {
    const timer = setTimeout(() => {
      setSearchDebounced(searchInput);
    }, delayMs);
    return () => clearTimeout(timer);
  }, [searchInput, delayMs]);

  const clearSearch = useCallback(() => {
    setSearchInput('');
    setSearchDebounced('');
  }, []);

  return {
    searchInput,
    searchDebounced,
    setSearchInput,
    clearSearch,
  };
}
