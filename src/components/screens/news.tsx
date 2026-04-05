import { RefreshCw, Newspaper } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { PageHeader } from '@/components/ui/page-header';
import { Skeleton } from '@/components/ui/skeleton';
import { NewsItemCard } from '@/components/news/news-item-card';
import { useNews } from '@/hooks/useNews';
import { allNewsSources } from '@/lib/news/sources';

export function News() {
  const { items, loading, error, refresh } = useNews(allNewsSources);

  return (
    <div className="container mx-auto h-full flex flex-col px-6">
      <PageHeader
        title="ニュース"
        description={loading ? '読み込み中...' : `${items.length}件の記事`}
        icon={Newspaper}
      >
        <Button
          variant="outline"
          size="sm"
          onClick={refresh}
          disabled={loading}
        >
          <RefreshCw
            className={`h-4 w-4 mr-2 ${loading ? 'animate-spin' : ''}`}
          />
          更新
        </Button>
      </PageHeader>

      <div className="flex-1 overflow-auto py-4">
        {error && (
          <div className="rounded-lg border border-destructive/50 bg-destructive/10 px-4 py-3 text-sm text-destructive mb-4">
            {error}
          </div>
        )}

        {loading && items.length === 0 ? (
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
        ) : items.length === 0 && !error ? (
          <div className="flex flex-col items-center justify-center h-48 text-muted-foreground">
            <Newspaper className="h-10 w-10 mb-3 opacity-30" />
            <p className="text-sm">記事が見つかりませんでした</p>
          </div>
        ) : (
          <div className="divide-y divide-border/50">
            {items.map((item) => (
              <NewsItemCard key={item.id} item={item} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
