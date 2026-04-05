import type { NewsSource } from '../types';

export const fourGamerSource: NewsSource = {
  id: '4gamer',
  name: '4Gamer.net',
  feedUrl: 'https://www.4gamer.net/rss/index.xml',
  siteUrl: 'https://www.4gamer.net',
  // RSSにサムネイルなし。記事URLに "TN/top.jpg" を付加して取得
  thumbnailSuffix: 'TN/001.jpg',
};
