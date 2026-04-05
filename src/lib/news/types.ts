/** ニュースソースの定義 */
export interface NewsSource {
  id: string;
  name: string;
  /** RSS/Atom フィードの URL */
  feedUrl: string;
  /** サイトのトップページ URL */
  siteUrl: string;
}

/** 各ソースから取得した正規化済みニュース記事 */
export interface NewsItem {
  /** `${sourceId}:${rawId}` 形式のユニーク ID */
  id: string;
  title: string;
  url: string;
  /** HTML が含まれる場合あり。表示時にストリップすること */
  description?: string;
  /** RFC 2822 形式の日時文字列（RSS pubDate） */
  publishedAt?: string;
  thumbnailUrl?: string;
  sourceId: string;
  sourceName: string;
}
