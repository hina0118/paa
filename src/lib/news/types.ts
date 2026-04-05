/** HTML スクレイピング用セレクタ設定 */
export interface HtmlSelectors {
  /** 記事アイテム要素の CSS セレクタ */
  item: string;
  /** タイトル要素（省略時はアイテムのテキスト内容を使用） */
  title?: string;
  /** サムネイル img 要素 */
  thumbnail?: string;
  /** 日付要素 */
  date?: string;
}

/** ニュースソースの定義 */
export interface NewsSource {
  id: string;
  name: string;
  /** RSS/Atom フィード URL、または HTML スクレイピング対象 URL */
  feedUrl: string;
  /** サイトのトップページ URL */
  siteUrl: string;
  /**
   * 指定した場合は RSS ではなく HTML をスクレイピングして記事を取得する。
   * RSS を持たないサイトへの対応に使用。
   */
  htmlSelectors?: HtmlSelectors;
  /**
   * RSS にサムネイルが含まれない場合に記事 URL へ付加してサムネイル URL を生成する。
   * 例: "TN/001.jpg" → `${articleUrl}TN/001.jpg`
   */
  thumbnailSuffix?: string;
  /**
   * 記事 URL から正規表現でキャプチャしてサムネイル URL を生成する。
   * template 内の "$1" が最初のキャプチャグループに置換される。
   * 例: { pattern: '/(\\d+)\\.html$', template: 'https://example.com/img/$1/list.jpg' }
   */
  thumbnailUrlBuilder?: { pattern: string; template: string };
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
