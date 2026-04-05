import { ExternalLink, Newspaper, Bookmark, Loader2 } from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { useState } from 'react';
import { cn } from '@/lib/utils';
import type { NewsItem } from '@/lib/news/types';

interface NewsItemCardProps {
  item: NewsItem;
  isClipped?: boolean;
  isClipping?: boolean;
  onClip?: (item: NewsItem) => void;
}

/** HTML タグと主要な HTML エンティティを除去してプレーンテキストに変換 */
function stripHtml(html: string): string {
  return html
    .replace(/<[^>]+>/g, '')
    .replace(/&nbsp;/g, ' ')
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#?\w+;/g, '')
    .trim();
}

/** RFC 2822 日付文字列を "YYYY/MM/DD" 形式に変換 */
function formatDate(dateStr?: string): string {
  if (!dateStr) return '';
  const date = new Date(dateStr);
  if (isNaN(date.getTime())) return '';
  return `${date.getFullYear()}/${String(date.getMonth() + 1).padStart(2, '0')}/${String(date.getDate()).padStart(2, '0')}`;
}

export function NewsItemCard({
  item,
  isClipped = false,
  isClipping = false,
  onClip,
}: NewsItemCardProps) {
  const handleClick = () => {
    if (item.url) {
      openUrl(item.url).catch(console.error);
    }
  };

  const handleClip = (e: React.MouseEvent) => {
    e.stopPropagation();
    onClip?.(item);
  };

  const [imgError, setImgError] = useState(false);

  const description = item.description
    ? stripHtml(item.description).slice(0, 120)
    : '';

  return (
    <div className="flex items-start gap-3 px-4 py-3 group">
      {/* サムネイル */}
      <button
        className="shrink-0 w-24 h-16 rounded-md overflow-hidden bg-muted flex items-center justify-center hover:opacity-90 transition-opacity"
        onClick={handleClick}
        tabIndex={-1}
        aria-label={`${item.title}を開く`}
      >
        {item.thumbnailUrl && !imgError ? (
          <img
            src={item.thumbnailUrl}
            alt=""
            className="w-full h-full object-cover"
            onError={() => setImgError(true)}
          />
        ) : (
          <Newspaper className="h-6 w-6 text-muted-foreground/50" />
        )}
      </button>

      {/* コンテンツ */}
      <button
        className="flex-1 min-w-0 text-left"
        onClick={handleClick}
        title={item.title}
      >
        <div className="flex items-start justify-between gap-2">
          <p className="text-sm font-medium leading-snug line-clamp-2 group-hover:text-primary transition-colors">
            {item.title}
          </p>
          <ExternalLink className="h-3.5 w-3.5 shrink-0 mt-0.5 text-muted-foreground/60 group-hover:text-primary/70 transition-colors" />
        </div>
        {description && (
          <p className="mt-1 text-xs text-muted-foreground line-clamp-2">
            {description}
          </p>
        )}
        <div className="mt-1.5 flex items-center gap-2">
          <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-primary/10 text-primary">
            {item.sourceName}
          </span>
          {item.publishedAt && (
            <span className="text-[11px] text-muted-foreground">
              {formatDate(item.publishedAt)}
            </span>
          )}
        </div>
      </button>

      {/* クリップボタン */}
      {onClip && (
        <button
          className={cn(
            'shrink-0 p-1.5 rounded-md transition-colors',
            isClipped
              ? 'text-primary'
              : 'text-muted-foreground/70 hover:text-primary hover:bg-muted/60'
          )}
          onClick={handleClip}
          disabled={isClipping || isClipped}
          title={isClipped ? 'クリップ済み' : 'クリップする'}
          aria-label={isClipped ? 'クリップ済み' : 'クリップする'}
        >
          {isClipping ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Bookmark
              className="h-4 w-4"
              fill={isClipped ? 'currentColor' : 'none'}
            />
          )}
        </button>
      )}
    </div>
  );
}
