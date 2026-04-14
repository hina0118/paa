import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Ban, Trash2 } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { PageHeader } from '@/components/ui/page-header';
import { toastSuccess, toastError, formatError } from '@/lib/toast';

interface ExclusionPattern {
  id: number;
  shop_domain: string | null;
  keyword: string;
  match_type: 'contains' | 'starts_with' | 'exact';
  note: string | null;
  created_at: string;
}

const MATCH_TYPE_LABELS: Record<string, string> = {
  contains: '部分一致',
  starts_with: '前方一致',
  exact: '完全一致',
};

export function ExclusionPatterns() {
  const [patterns, setPatterns] = useState<ExclusionPattern[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  const loadPatterns = async () => {
    try {
      const result = await invoke<ExclusionPattern[]>(
        'list_exclusion_patterns'
      );
      setPatterns(result);
    } catch (e) {
      toastError(`除外パターンの取得に失敗しました: ${formatError(e)}`);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadPatterns();
  }, []);

  const handleDelete = async (id: number) => {
    try {
      await invoke('delete_exclusion_pattern', { id });
      toastSuccess('除外キーワードを削除しました');
      setPatterns((prev) => prev.filter((p) => p.id !== id));
    } catch (e) {
      toastError(`削除に失敗しました: ${formatError(e)}`);
    }
  };

  return (
    <div className="flex flex-col h-full">
      <PageHeader icon={Ban} title="除外キーワード" />
      <div className="flex-1 overflow-auto p-4">
        <Card>
          <CardHeader>
            <CardTitle className="text-base">登録済みキーワード</CardTitle>
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <p className="text-sm text-muted-foreground">読み込み中...</p>
            ) : patterns.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                除外キーワードが登録されていません
              </p>
            ) : (
              <div className="divide-y">
                {patterns.map((p) => (
                  <div
                    key={p.id}
                    className="flex items-center justify-between py-2 gap-2"
                  >
                    <div className="flex items-center gap-2 flex-wrap min-w-0">
                      <span className="font-medium text-sm">{p.keyword}</span>
                      <span className="text-xs px-1.5 py-0.5 rounded bg-muted text-muted-foreground">
                        {MATCH_TYPE_LABELS[p.match_type] ?? p.match_type}
                      </span>
                      {p.shop_domain ? (
                        <span className="text-xs px-1.5 py-0.5 rounded bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300">
                          {p.shop_domain}
                        </span>
                      ) : (
                        <span className="text-xs text-muted-foreground">
                          全ショップ
                        </span>
                      )}
                      {p.note && (
                        <span className="text-xs text-muted-foreground truncate">
                          {p.note}
                        </span>
                      )}
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="shrink-0 text-destructive hover:text-destructive"
                      onClick={() => handleDelete(p.id)}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
