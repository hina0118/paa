import { invoke } from '@tauri-apps/api/core';

/** クリップ済み記事（フロントエンド用正規化型） */
export interface NewsClip {
  id: number;
  title: string;
  url: string;
  sourceName: string;
  publishedAt?: string;
  summary?: string;
  tags: string[];
  clippedAt: string;
}

/** Rust が返す snake_case の生型 */
interface RawNewsClip {
  id: number;
  title: string;
  url: string;
  source_name: string;
  published_at?: string;
  summary?: string;
  tags: string[];
  clipped_at: string;
}

function normalize(raw: RawNewsClip): NewsClip {
  return {
    id: raw.id,
    title: raw.title,
    url: raw.url,
    sourceName: raw.source_name,
    publishedAt: raw.published_at,
    summary: raw.summary,
    tags: raw.tags,
    clippedAt: raw.clipped_at,
  };
}

export interface ClipArticleParams {
  url: string;
  title: string;
  sourceName: string;
  publishedAt?: string;
  description?: string;
}

/** 記事をクリップ保存する（AI要約・タグ生成を含む） */
export async function clipNewsArticle(
  params: ClipArticleParams
): Promise<NewsClip> {
  const raw = await invoke<RawNewsClip>('clip_news_article', {
    url: params.url,
    title: params.title,
    sourceName: params.sourceName,
    publishedAt: params.publishedAt,
    description: params.description,
  });
  return normalize(raw);
}

/** クリップ一覧を取得する */
export async function getNewsClips(): Promise<NewsClip[]> {
  const raw = await invoke<RawNewsClip[]>('get_news_clips');
  return raw.map(normalize);
}

/** クリップを削除する */
export async function deleteNewsClip(id: number): Promise<void> {
  await invoke('delete_news_clip', { id });
}

/** クリップ済み URL 一覧を取得する（ニュース一覧での既クリップ判定用） */
export async function getClippedUrls(): Promise<string[]> {
  return invoke<string[]>('get_clipped_urls');
}
