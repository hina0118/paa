import { RefreshCw, Newspaper, Bookmark } from 'lucide-react';
import { useState, useMemo, useRef } from 'react';
import {
  NewsClipCalendar,
  getClipDateKey,
} from '@/components/news/news-clip-calendar';
import { useVirtualizer } from '@tanstack/react-virtual';
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
  const [selectedSources, setSelectedSources] = useState<Set<string>>(
    new Set()
  );

  const {
    items,
    loading: feedLoading,
    error: feedError,
    lastUpdatedAt,
    refresh: refreshFeed,
  } = useNews(allNewsSources);
  const {
    clips,
    clippedUrls,
    clippingUrl,
    clip,
    unclip,
    refresh: refreshClips,
    refreshEvents,
  } = useNewsClips();

  const handleRefresh = () => {
    if (activeTab === 'feed') refreshFeed();
    else refreshClips();
  };

  const toggleSource = (sourceId: string) => {
    setSelectedSources((prev) => {
      const next = new Set(prev);
      if (next.has(sourceId)) next.delete(sourceId);
      else next.add(sourceId);
      return next;
    });
  };

  // フィード表示に使うソース一覧（記事が存在するもののみ）
  const feedSources = useMemo(() => {
    const ids = new Set(items.map((i) => i.sourceId));
    return allNewsSources.filter((s) => ids.has(s.id));
  }, [items]);

  // クリップ表示に使うソース一覧
  const clipSources = useMemo(() => {
    const names = new Set(clips.map((c) => c.sourceName));
    return [...names];
  }, [clips]);

  const filteredItems = useMemo(
    () =>
      selectedSources.size === 0
        ? items
        : items.filter((i) => selectedSources.has(i.sourceId)),
    [items, selectedSources]
  );

  const filteredClips = useMemo(
    () =>
      selectedSources.size === 0
        ? clips
        : clips.filter((c) => selectedSources.has(c.sourceName)),
    [clips, selectedSources]
  );

  const loading = activeTab === 'feed' ? feedLoading : false;
  const visibleCount =
    activeTab === 'feed' ? filteredItems.length : filteredClips.length;
  const totalCount = activeTab === 'feed' ? items.length : clips.length;
  const description = loading
    ? '読み込み中...'
    : selectedSources.size > 0
      ? `${visibleCount} / ${totalCount}件`
      : `${totalCount}件${activeTab === 'clips' ? 'のクリップ' : ''}`;

  // タブ切り替え時にフィルターをリセット
  const handleTabChange = (tab: Tab) => {
    setActiveTab(tab);
    setSelectedSources(new Set());
  };

  return (
    <div className="container mx-auto h-full flex flex-col px-6">
      <PageHeader title="ニュース" description={description} icon={Newspaper}>
        {lastUpdatedAt && (
          <span className="text-xs text-muted-foreground">
            最終更新:{' '}
            {lastUpdatedAt.toLocaleTimeString('ja-JP', {
              hour: '2-digit',
              minute: '2-digit',
            })}
          </span>
        )}
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
      <div className="flex gap-1 border-b mt-2 shrink-0">
        <TabButton
          active={activeTab === 'feed'}
          onClick={() => handleTabChange('feed')}
          icon={<Newspaper className="h-3.5 w-3.5" />}
          label="ニュース一覧"
        />
        <TabButton
          active={activeTab === 'clips'}
          onClick={() => handleTabChange('clips')}
          icon={<Bookmark className="h-3.5 w-3.5" />}
          label={`クリップ${clips.length > 0 ? ` (${clips.length})` : ''}`}
        />
      </div>

      {/* ソースフィルター */}
      {activeTab === 'feed' && feedSources.length > 1 && (
        <SourceFilter
          sources={feedSources.map((s) => ({ id: s.id, name: s.name }))}
          selected={selectedSources}
          onToggle={toggleSource}
        />
      )}
      {activeTab === 'clips' && clipSources.length > 1 && (
        <SourceFilter
          sources={clipSources.map((name) => ({ id: name, name }))}
          selected={selectedSources}
          onToggle={toggleSource}
        />
      )}

      <div className="flex-1 min-h-0">
        {activeTab === 'feed' && (
          <FeedTab
            items={filteredItems}
            loading={feedLoading}
            error={feedError}
            clippedUrls={clippedUrls}
            clippingUrl={clippingUrl}
            onClip={clip}
          />
        )}
        {activeTab === 'clips' && (
          <ClipsTab
            clips={filteredClips}
            onUnclip={unclip}
            onRefreshEvents={refreshEvents}
          />
        )}
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

function SourceFilter({
  sources,
  selected,
  onToggle,
}: {
  sources: { id: string; name: string }[];
  selected: Set<string>;
  onToggle: (id: string) => void;
}) {
  return (
    <div className="flex items-center gap-1.5 py-2 flex-wrap shrink-0">
      {sources.map((source) => {
        const active = selected.has(source.id);
        return (
          <button
            key={source.id}
            onClick={() => onToggle(source.id)}
            className={cn(
              'px-2.5 py-1 rounded-full text-xs font-medium transition-colors border',
              active
                ? 'bg-primary text-primary-foreground border-primary'
                : 'bg-transparent text-muted-foreground border-border hover:border-primary/50 hover:text-foreground'
            )}
          >
            {source.name}
          </button>
        );
      })}
    </div>
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
  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 88,
    overscan: 8,
  });

  if (error) {
    return (
      <div className="rounded-lg border border-destructive/50 bg-destructive/10 px-4 py-3 text-sm text-destructive mt-2">
        {error}
      </div>
    );
  }

  if (loading && items.length === 0) {
    return (
      <div className="space-y-1 py-2">
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
    <div ref={parentRef} className="h-full overflow-auto">
      <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
        {virtualizer.getVirtualItems().map((virtualItem) => {
          const item = items[virtualItem.index];
          return (
            <div
              key={virtualItem.key}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                transform: `translateY(${virtualItem.start}px)`,
              }}
            >
              <div
                className={
                  virtualItem.index > 0 ? 'border-t border-border/50' : ''
                }
              >
                <NewsItemCard
                  item={item}
                  isClipped={clippedUrls.has(item.url)}
                  isClipping={clippingUrl === item.url}
                  onClip={onClip}
                />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function ClipsTab({
  clips,
  onUnclip,
  onRefreshEvents,
}: {
  clips: ReturnType<typeof useNewsClips>['clips'];
  onUnclip: (id: number, url: string) => void;
  onRefreshEvents: (clipId: number) => Promise<void>;
}) {
  const [selectedDate, setSelectedDate] = useState<string | null>(null);

  const visibleClips = useMemo(
    () =>
      selectedDate === null
        ? clips
        : clips.filter((c) => getClipDateKey(c) === selectedDate),
    [clips, selectedDate]
  );

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
    <div className="flex h-full gap-0">
      {/* 左: クリップ一覧 (2/3) */}
      <div className="flex-[2] min-w-0 h-full overflow-hidden">
        <ClipsList clips={visibleClips} onUnclip={onUnclip} />
      </div>

      {/* 右: 月カレンダー (1/3) */}
      <div className="flex-[1] min-w-0 h-full border-l border-border/50 px-2 py-1 overflow-y-auto">
        <NewsClipCalendar
          clips={clips}
          selectedDate={selectedDate}
          onSelectDate={setSelectedDate}
          onRefreshEvents={onRefreshEvents}
        />
      </div>
    </div>
  );
}

function ClipsList({
  clips,
  onUnclip,
}: {
  clips: ReturnType<typeof useNewsClips>['clips'];
  onUnclip: (id: number, url: string) => void;
}) {
  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: clips.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 130,
    overscan: 5,
  });

  if (clips.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-48 text-muted-foreground">
        <Bookmark className="h-8 w-8 mb-2 opacity-30" />
        <p className="text-sm">この日の記事はありません</p>
      </div>
    );
  }

  return (
    <div ref={parentRef} className="h-full overflow-auto">
      <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
        {virtualizer.getVirtualItems().map((virtualItem) => {
          const clip = clips[virtualItem.index];
          return (
            <div
              key={virtualItem.key}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                transform: `translateY(${virtualItem.start}px)`,
              }}
            >
              <div
                className={
                  virtualItem.index > 0 ? 'border-t border-border/50' : ''
                }
              >
                <NewsClipCard clip={clip} onUnclip={onUnclip} />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
