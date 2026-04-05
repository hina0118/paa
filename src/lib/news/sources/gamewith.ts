import type { NewsSource } from '../types';

export const gamewithSource: NewsSource = {
  id: 'gamewith',
  name: 'GameWith',
  feedUrl: 'https://gamewith.jp/news',
  siteUrl: 'https://gamewith.jp',
  // GameWith は RSS 非対応のため HTML スクレイピングで取得
  htmlSelectors: {
    item: 'a[href*="/articles/"]',
    thumbnail: 'img',
  },
};
