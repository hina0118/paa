import { invoke } from '@tauri-apps/api/core';
import type { NewsItem, NewsSource } from './types';

/** Rust コマンドが返す生データ（snake_case） */
interface RawFeedItem {
  id: string;
  title: string;
  url: string;
  description?: string;
  published_at?: string;
  thumbnail_url?: string;
}

async function fetchFromSource(source: NewsSource): Promise<NewsItem[]> {
  const raw = await invoke<RawFeedItem[]>('fetch_news_feed', {
    url: source.feedUrl,
  });
  return raw.map((item) => ({
    id: `${source.id}:${item.id}`,
    title: item.title,
    url: item.url,
    description: item.description,
    publishedAt: item.published_at,
    thumbnailUrl: item.thumbnail_url,
    sourceId: source.id,
    sourceName: source.name,
  }));
}

/**
 * 複数ソースからニュースを並列取得し、日時の降順で返す。
 * 一部ソースが失敗しても他ソースの結果は返す。
 */
export async function fetchNewsFromSources(
  sources: NewsSource[]
): Promise<NewsItem[]> {
  const results = await Promise.allSettled(sources.map(fetchFromSource));

  const items: NewsItem[] = [];
  for (const result of results) {
    if (result.status === 'fulfilled') {
      items.push(...result.value);
    }
  }

  items.sort((a, b) => {
    if (!a.publishedAt) return 1;
    if (!b.publishedAt) return -1;
    return (
      new Date(b.publishedAt).getTime() - new Date(a.publishedAt).getTime()
    );
  });

  return items;
}
