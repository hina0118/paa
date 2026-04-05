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

function toNewsItem(source: NewsSource, item: RawFeedItem): NewsItem {
  const thumbnailUrl =
    item.thumbnail_url ??
    (source.thumbnailSuffix && item.url
      ? item.url.replace(/\/?$/, '/') + source.thumbnailSuffix
      : undefined) ??
    (source.thumbnailUrlBuilder && item.url
      ? (() => {
          const m = item.url.match(
            new RegExp(source.thumbnailUrlBuilder!.pattern)
          );
          return m
            ? source
                .thumbnailUrlBuilder!.template.replace('$1', m[1] ?? '')
                .replace('$2', m[2] ?? '')
            : undefined;
        })()
      : undefined);

  return {
    id: `${source.id}:${item.id}`,
    title: item.title,
    url: item.url,
    description: item.description,
    publishedAt: item.published_at,
    thumbnailUrl,
    sourceId: source.id,
    sourceName: source.name,
  };
}

async function fetchFromSource(source: NewsSource): Promise<NewsItem[]> {
  if (source.htmlSelectors) {
    // HTML スクレイピングで取得
    const raw = await invoke<RawFeedItem[]>('fetch_news_html', {
      url: source.feedUrl,
      selectors: {
        item: source.htmlSelectors.item,
        title: source.htmlSelectors.title,
        thumbnail: source.htmlSelectors.thumbnail,
        date: source.htmlSelectors.date,
      },
    });
    return raw.map((item) => toNewsItem(source, item));
  }

  // RSS/Atom フィードで取得
  const raw = await invoke<RawFeedItem[]>('fetch_news_feed', {
    url: source.feedUrl,
  });
  return raw.map((item) => toNewsItem(source, item));
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
