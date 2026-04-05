import { useState, useEffect, useCallback } from 'react';
import { toast } from 'sonner';
import type { NewsItem } from '@/lib/news/types';
import {
  type NewsClip,
  clipNewsArticle,
  getNewsClips,
  deleteNewsClip,
  getClippedUrls,
} from '@/lib/news/clips';

interface UseNewsClipsResult {
  clips: NewsClip[];
  /** クリップ済み URL の Set（ニュース一覧でのバッジ表示用） */
  clippedUrls: Set<string>;
  loading: boolean;
  /** クリップ処理中の URL（ボタンのローディング表示用） */
  clippingUrl: string | null;
  clip: (item: NewsItem) => Promise<void>;
  unclip: (id: number, url: string) => Promise<void>;
  refresh: () => Promise<void>;
}

export function useNewsClips(): UseNewsClipsResult {
  const [clips, setClips] = useState<NewsClip[]>([]);
  const [clippedUrls, setClippedUrls] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);
  const [clippingUrl, setClippingUrl] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [clipsData, urls] = await Promise.all([
        getNewsClips(),
        getClippedUrls(),
      ]);
      setClips(clipsData);
      setClippedUrls(new Set(urls));
    } catch (e) {
      console.error('クリップの取得に失敗:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const clip = useCallback(
    async (item: NewsItem) => {
      if (clippingUrl) return;
      setClippingUrl(item.url);
      try {
        const newClip = await clipNewsArticle({
          url: item.url,
          title: item.title,
          sourceName: item.sourceName,
          publishedAt: item.publishedAt,
          description: item.description,
        });
        setClips((prev) => [newClip, ...prev]);
        setClippedUrls((prev) => new Set([...prev, item.url]));
        toast.success('クリップしました');
      } catch (e) {
        // Tauri invoke は string で reject するため typeof string も考慮
        const msg =
          e instanceof Error
            ? e.message
            : typeof e === 'string'
              ? e
              : 'クリップに失敗しました';
        toast.error(msg);
      } finally {
        setClippingUrl(null);
      }
    },
    [clippingUrl]
  );

  const unclip = useCallback(async (id: number, url: string) => {
    try {
      await deleteNewsClip(id);
      setClips((prev) => prev.filter((c) => c.id !== id));
      setClippedUrls((prev) => {
        const next = new Set(prev);
        next.delete(url);
        return next;
      });
      toast.success('クリップを削除しました');
    } catch {
      toast.error('削除に失敗しました');
    }
  }, []);

  return { clips, clippedUrls, loading, clippingUrl, clip, unclip, refresh };
}
