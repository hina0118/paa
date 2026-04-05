import type { NewsSource } from '../types';

export const hobbyWatchSource: NewsSource = {
  id: 'hobby-watch',
  name: 'Hobby Watch',
  feedUrl: 'https://hobby.watch.impress.co.jp/data/rss/1.0/hbw/feed.rdf',
  siteUrl: 'https://hobby.watch.impress.co.jp',
  // 記事URL例: .../docs/news/2099199.html
  // サムネイル: https://asset.watch.impress.co.jp/img/hbw/list/2099199/list.jpg
  // 記事ID例: 2099197 → 前4桁/後3桁 に分割
  // サムネイル: https://asset.watch.impress.co.jp/img/hbw/docs/2099/197/001.jpg
  thumbnailUrlBuilder: {
    pattern: '/(\\d{4})(\\d{3})\\.html$',
    template: 'https://asset.watch.impress.co.jp/img/hbw/docs/$1/$2/001.jpg',
  },
};
