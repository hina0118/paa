import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../ui/card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';

interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
}

type LogLevel = 'INFO' | 'WARN' | 'ERROR' | 'DEBUG' | 'TRACE' | null;

export function Logs() {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filterLevel, setFilterLevel] = useState<LogLevel>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [autoRefresh, setAutoRefresh] = useState(false);

  const loadLogs = async (level?: string) => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<LogEntry[]>('get_logs', {
        levelFilter: level || null,
        limit: 500,
      });
      setLogs(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      console.error('Failed to load logs:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    // 初回またはフィルタ変更時にログを読み込む
    loadLogs(filterLevel || undefined);

    // 自動更新が有効な場合はインターバルを設定
    if (autoRefresh) {
      const interval = setInterval(() => {
        loadLogs(filterLevel || undefined);
      }, 2000); // 2秒ごとに更新

      return () => clearInterval(interval);
    }
  }, [autoRefresh, filterLevel]);

  // ログは降順（最新が上）のため、自動更新時にスクロールしない

  const getLevelColor = (level: string) => {
    switch (level) {
      case 'ERROR':
        return 'text-red-600 dark:text-red-400 bg-red-100 dark:bg-red-900/30';
      case 'WARN':
        return 'text-amber-600 dark:text-amber-400 bg-amber-100 dark:bg-amber-900/30';
      case 'INFO':
        return 'text-blue-600 dark:text-blue-400 bg-blue-100 dark:bg-blue-900/30';
      case 'DEBUG':
        return 'text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-900/30';
      case 'TRACE':
        return 'text-purple-600 dark:text-purple-400 bg-purple-100 dark:bg-purple-900/30';
      default:
        return 'text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-900/30';
    }
  };

  const filteredLogs = logs.filter((log) => {
    if (searchQuery) {
      return log.message.toLowerCase().includes(searchQuery.toLowerCase());
    }
    return true;
  });

  const levelCounts = logs.reduce(
    (acc, log) => {
      acc[log.level] = (acc[log.level] || 0) + 1;
      return acc;
    },
    {} as Record<string, number>
  );

  return (
    <div className="flex flex-col h-screen">
      <div className="container mx-auto py-6 space-y-4 flex-shrink-0">
        <div className="flex justify-between items-center">
          <h1 className="text-3xl font-bold">ログビューアー</h1>
          <div className="flex gap-2">
            <Button
              variant={autoRefresh ? 'default' : 'outline'}
              onClick={() => setAutoRefresh(!autoRefresh)}
              aria-label={autoRefresh ? '自動更新を停止' : '自動更新を開始'}
              aria-pressed={autoRefresh}
            >
              {autoRefresh ? '自動更新中' : '自動更新'}
            </Button>
            <Button
              onClick={() => loadLogs(filterLevel || undefined)}
              disabled={loading}
              aria-label="ログを手動で更新"
            >
              {loading ? '読み込み中...' : '更新'}
            </Button>
          </div>
        </div>

        {error && (
          <Card className="border-red-500">
            <CardHeader>
              <CardTitle className="text-red-500">エラー</CardTitle>
            </CardHeader>
            <CardContent>
              <p>{error}</p>
            </CardContent>
          </Card>
        )}

        <div className="grid gap-4 md:grid-cols-5">
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">総ログ数</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{logs.length}</div>
            </CardContent>
          </Card>

          {(['ERROR', 'WARN', 'INFO', 'DEBUG'] as const).map((level) => (
            <Card
              key={level}
              className={filterLevel === level ? 'border-2 border-primary' : ''}
            >
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium">{level}</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">
                  {levelCounts[level] || 0}
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  className="mt-2 w-full"
                  onClick={() =>
                    setFilterLevel(filterLevel === level ? null : level)
                  }
                  aria-label={`${level}レベルのログ${filterLevel === level ? 'フィルタを解除' : 'でフィルタ'}`}
                  aria-pressed={filterLevel === level}
                >
                  {filterLevel === level ? 'フィルタ解除' : 'フィルタ'}
                </Button>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>

      <div className="container mx-auto pb-6 flex-1 flex flex-col min-h-0">
        <Card className="flex-1 flex flex-col min-h-0">
          <CardHeader className="flex-shrink-0">
            <CardTitle>ログ一覧</CardTitle>
            <CardDescription>
              {filterLevel
                ? `${filterLevel}レベルのログを表示中`
                : '全てのログを表示中'}
              {searchQuery && ` - "${searchQuery}" で検索中`}
            </CardDescription>
            <div className="flex gap-2 mt-4">
              <Input
                placeholder="ログメッセージを検索..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="flex-1"
              />
              {(filterLevel || searchQuery) && (
                <Button
                  variant="outline"
                  onClick={() => {
                    setFilterLevel(null);
                    setSearchQuery('');
                  }}
                >
                  全てクリア
                </Button>
              )}
            </div>
          </CardHeader>
          <CardContent className="flex-1 overflow-hidden flex flex-col">
            <div
              className="space-y-2 overflow-y-auto flex-1"
              role="log"
              aria-live={autoRefresh ? 'polite' : 'off'}
              aria-atomic="false"
              aria-label="アプリケーションログ一覧"
            >
              {filteredLogs.length === 0 && !loading && (
                <p className="text-center text-muted-foreground py-10">
                  ログがありません
                </p>
              )}

              {filteredLogs.map((log, index) => (
                <div
                  key={`${log.timestamp}-${log.level}-${index}`}
                  className="flex items-start gap-3 p-3 rounded-lg border hover:bg-muted/50 transition-colors"
                >
                  <span className="text-xs text-muted-foreground font-mono whitespace-nowrap">
                    {log.timestamp}
                  </span>
                  <span
                    className={`text-xs font-bold px-2 py-1 rounded ${getLevelColor(
                      log.level
                    )}`}
                  >
                    {log.level}
                  </span>
                  <span className="text-sm flex-1 font-mono break-all">
                    {log.message}
                  </span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
