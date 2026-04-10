import { useState, useEffect, useCallback } from 'react';
import type { NewsItem, NewsSource } from '@/lib/news/types';
import { fetchNewsFromSources } from '@/lib/news/fetcher';
import { allNewsSources } from '@/lib/news/sources';

interface UseNewsResult {
  items: NewsItem[];
  loading: boolean;
  error: string | null;
  lastUpdatedAt: Date | null;
  refresh: () => void;
}

export function useNews(sources: NewsSource[] = allNewsSources): UseNewsResult {
  const [items, setItems] = useState<NewsItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdatedAt, setLastUpdatedAt] = useState<Date | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await fetchNewsFromSources(sources);
      setItems(data);
      setLastUpdatedAt(new Date());
    } catch (e) {
      setError(e instanceof Error ? e.message : 'ニュースの取得に失敗しました');
    } finally {
      setLoading(false);
    }
  }, [sources]);

  useEffect(() => {
    refresh();

    const interval = setInterval(refresh, 10 * 60 * 1000);
    return () => clearInterval(interval);
  }, [refresh]);

  return { items, loading, error, lastUpdatedAt, refresh };
}
