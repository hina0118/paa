import { RefreshCw, Newspaper, Bookmark } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { PageHeader } from '@/components/ui/page-header';
import { Skeleton } from '@/components/ui/skeleton';
import { NewsItemCard } from '@/components/news/news-item-card';
import { NewsClipCard } from '@/components/news/news-clip-card';
import { useNews } from '@/hooks/useNews';
import { useNewsClips } from '@/hooks/useNewsClips';
import { allNewsSources } from '@/lib/news/sources';
import { cn } from '@/lib/utils';

type Tab = 'feed' | 'clips';

export function News() {
  const [activeTab, setActiveTab] = useState<Tab>('feed');

  const {
    items,
    loading: feedLoading,
    error: feedError,
    refresh: refreshFeed,
  } = useNews(allNewsSources);
  const {
    clips,
    clippedUrls,
    clippingUrl,
    clip,
    unclip,
    refresh: refreshClips,
  } = useNewsClips();

  const handleRefresh = () => {
    if (activeTab === 'feed') refreshFeed();
    else refreshClips();
  };

  const loading = activeTab === 'feed' ? feedLoading : false;
  const description =
    activeTab === 'feed'
      ? feedLoading
        ? '読み込み中...'
        : `${items.length}件の記事`
      : `${clips.length}件のクリップ`;

  return (
    <div className="container mx-auto h-full flex flex-col px-6">
      <PageHeader
        title="ニュース"
        description={loading ? '読み込み中...' : description}
        icon={Newspaper}
      >
        <Button
          variant="outline"
          size="sm"
          onClick={handleRefresh}
          disabled={loading}
        >
          <RefreshCw
            className={cn('h-4 w-4 mr-2', loading && 'animate-spin')}
          />
          更新
        </Button>
      </PageHeader>

      {/* タブ */}
      <div className="flex gap-1 border-b mt-2 mb-0 shrink-0">
        <TabButton
          active={activeTab === 'feed'}
          onClick={() => setActiveTab('feed')}
          icon={<Newspaper className="h-3.5 w-3.5" />}
          label="ニュース一覧"
        />
        <TabButton
          active={activeTab === 'clips'}
          onClick={() => setActiveTab('clips')}
          icon={<Bookmark className="h-3.5 w-3.5" />}
          label={`クリップ${clips.length > 0 ? ` (${clips.length})` : ''}`}
        />
      </div>

      <div className="flex-1 overflow-auto py-2">
        {activeTab === 'feed' && (
          <FeedTab
            items={items}
            loading={feedLoading}
            error={feedError}
            clippedUrls={clippedUrls}
            clippingUrl={clippingUrl}
            onClip={clip}
          />
        )}
        {activeTab === 'clips' && <ClipsTab clips={clips} onUnclip={unclip} />}
      </div>
    </div>
  );
}

// -------------------------------------------------------
// サブコンポーネント
// -------------------------------------------------------

function TabButton({
  active,
  onClick,
  icon,
  label,
}: {
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  label: string;
}) {
  return (
    <button
      className={cn(
        'flex items-center gap-1.5 px-4 py-2 text-sm font-medium border-b-2 transition-colors',
        active
          ? 'border-primary text-primary'
          : 'border-transparent text-muted-foreground hover:text-foreground hover:border-border'
      )}
      onClick={onClick}
    >
      {icon}
      {label}
    </button>
  );
}

function FeedTab({
  items,
  loading,
  error,
  clippedUrls,
  clippingUrl,
  onClip,
}: {
  items: ReturnType<typeof useNews>['items'];
  loading: boolean;
  error: string | null;
  clippedUrls: Set<string>;
  clippingUrl: string | null;
  onClip: (item: ReturnType<typeof useNews>['items'][number]) => void;
}) {
  if (error) {
    return (
      <div className="rounded-lg border border-destructive/50 bg-destructive/10 px-4 py-3 text-sm text-destructive mt-2">
        {error}
      </div>
    );
  }

  if (loading && items.length === 0) {
    return (
      <div className="space-y-1">
        {Array.from({ length: 10 }).map((_, i) => (
          <div key={i} className="flex items-start gap-3 px-4 py-3">
            <Skeleton className="w-24 h-16 rounded-md shrink-0" />
            <div className="flex-1 space-y-2">
              <Skeleton className="h-4 w-full" />
              <Skeleton className="h-3 w-3/4" />
              <Skeleton className="h-3 w-24" />
            </div>
          </div>
        ))}
      </div>
    );
  }

  if (items.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-48 text-muted-foreground">
        <Newspaper className="h-10 w-10 mb-3 opacity-30" />
        <p className="text-sm">記事が見つかりませんでした</p>
      </div>
    );
  }

  return (
    <div className="divide-y divide-border/50">
      {items.map((item) => (
        <NewsItemCard
          key={item.id}
          item={item}
          isClipped={clippedUrls.has(item.url)}
          isClipping={clippingUrl === item.url}
          onClip={onClip}
        />
      ))}
    </div>
  );
}

function ClipsTab({
  clips,
  onUnclip,
}: {
  clips: ReturnType<typeof useNewsClips>['clips'];
  onUnclip: (id: number, url: string) => void;
}) {
  if (clips.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-48 text-muted-foreground">
        <Bookmark className="h-10 w-10 mb-3 opacity-30" />
        <p className="text-sm">クリップした記事がありません</p>
        <p className="text-xs mt-1 opacity-70">
          ニュース一覧のブックマークアイコンからクリップできます
        </p>
      </div>
    );
  }

  return (
    <div className="divide-y divide-border/50">
      {clips.map((clip) => (
        <NewsClipCard key={clip.id} clip={clip} onUnclip={onUnclip} />
      ))}
    </div>
  );
}
