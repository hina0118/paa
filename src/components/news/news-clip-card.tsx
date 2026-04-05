import { ExternalLink, Trash2, Tag } from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import type { NewsClip } from '@/lib/news/clips';

interface NewsClipCardProps {
  clip: NewsClip;
  onUnclip: (id: number, url: string) => void;
}

/** RFC 2822 または ISO 8601 の日付文字列を "YYYY/MM/DD" に変換 */
function formatDate(dateStr?: string): string {
  if (!dateStr) return '';
  const date = new Date(dateStr);
  if (isNaN(date.getTime())) return '';
  return `${date.getFullYear()}/${String(date.getMonth() + 1).padStart(2, '0')}/${String(date.getDate()).padStart(2, '0')}`;
}

export function NewsClipCard({ clip, onUnclip }: NewsClipCardProps) {
  const handleOpen = () => {
    if (clip.url) {
      openUrl(clip.url).catch(console.error);
    }
  };

  return (
    <div className="px-4 py-4 group">
      {/* ヘッダー行: タイトル + 操作 */}
      <div className="flex items-start gap-2">
        <button
          className="flex-1 min-w-0 text-left"
          onClick={handleOpen}
          title={clip.title}
        >
          <p className="text-sm font-medium leading-snug line-clamp-2 group-hover:text-primary transition-colors flex items-center gap-1.5">
            {clip.title}
            <ExternalLink className="h-3 w-3 shrink-0 inline text-muted-foreground/40" />
          </p>
        </button>

        <button
          className="shrink-0 p-1.5 rounded-md text-muted-foreground/40 hover:text-destructive hover:bg-destructive/10 transition-colors"
          onClick={() => onUnclip(clip.id, clip.url)}
          title="クリップを削除"
          aria-label="クリップを削除"
        >
          <Trash2 className="h-3.5 w-3.5" />
        </button>
      </div>

      {/* メタ情報 */}
      <div className="mt-1 flex items-center gap-2">
        <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-primary/10 text-primary">
          {clip.sourceName}
        </span>
        {clip.publishedAt && (
          <span className="text-[11px] text-muted-foreground">
            {formatDate(clip.publishedAt)}
          </span>
        )}
        <span className="text-[11px] text-muted-foreground/60">
          クリップ: {formatDate(clip.clippedAt)}
        </span>
      </div>

      {/* AI 要約 */}
      {clip.summary && (
        <p className="mt-2 text-xs text-muted-foreground leading-relaxed">
          {clip.summary}
        </p>
      )}

      {/* タグ */}
      {clip.tags.length > 0 && (
        <div className="mt-2 flex items-center gap-1 flex-wrap">
          <Tag className="h-3 w-3 text-muted-foreground/50 shrink-0" />
          {clip.tags.map((tag) => (
            <span
              key={tag}
              className="inline-flex items-center px-1.5 py-0.5 rounded-full text-[10px] bg-muted text-muted-foreground"
            >
              {tag}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
