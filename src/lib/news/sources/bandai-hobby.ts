import type { NewsSource } from '../types';

export const bandaiHobbySource: NewsSource = {
  id: 'bandai-hobby',
  name: 'バンダイ ホビーサイト',
  feedUrl: 'https://bandai-hobby.net/news/',
  siteUrl: 'https://bandai-hobby.net',
  htmlSelectors: {
    item: 'ul.p-newslist__lists li a',
    title: '.p-newslist__tit',
    thumbnail: 'img',
    date: '.p-newslist__date',
  },
};
