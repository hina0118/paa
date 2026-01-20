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

interface EmailStats {
  total_emails: number;
  with_body_plain: number;
  with_body_html: number;
  without_body: number;
  avg_plain_length: number;
  avg_html_length: number;
}

// プログレスバーの最大値（文字数）
// テキスト形式: 一般的なメールの平均的な長さを基準に5000文字
// HTML形式: HTMLタグを含むため、テキストの約4倍の20000文字
const PROGRESS_MAX_PLAIN = 5000;
const PROGRESS_MAX_HTML = 20000;

export function Dashboard() {
  const [stats, setStats] = useState<EmailStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadStats = async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<EmailStats>('get_email_stats');
      setStats(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      console.error('Failed to load email stats:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadStats();
  }, []);

  const formatNumber = (num: number) => {
    return new Intl.NumberFormat('ja-JP').format(num);
  };

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 文字';
    return `${formatNumber(Math.round(bytes))} 文字`;
  };

  const calculatePercentage = (part: number, total: number) => {
    if (total === 0) return '0';
    return ((part / total) * 100).toFixed(1);
  };

  return (
    <div className="container mx-auto py-10 space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-3xl font-bold">ダッシュボード</h1>
        <Button onClick={loadStats} disabled={loading}>
          {loading ? '読み込み中...' : '更新'}
        </Button>
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

      {stats && (
        <>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">
                  総メール数
                </CardTitle>
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth="2"
                  className="h-4 w-4 text-muted-foreground"
                >
                  <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
                </svg>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">
                  {formatNumber(stats.total_emails)}
                </div>
                <p className="text-xs text-muted-foreground">
                  取り込み済みメール
                </p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">
                  テキスト本文あり
                </CardTitle>
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth="2"
                  className="h-4 w-4 text-muted-foreground"
                >
                  <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
                  <circle cx="9" cy="7" r="4" />
                  <path d="M22 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75" />
                </svg>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">
                  {formatNumber(stats.with_body_plain)}
                </div>
                <p className="text-xs text-muted-foreground">
                  {calculatePercentage(
                    stats.with_body_plain,
                    stats.total_emails
                  )}
                  % のメール
                </p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">
                  HTML本文あり
                </CardTitle>
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth="2"
                  className="h-4 w-4 text-muted-foreground"
                >
                  <rect width="20" height="14" x="2" y="5" rx="2" />
                  <path d="M2 10h20" />
                </svg>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">
                  {formatNumber(stats.with_body_html)}
                </div>
                <p className="text-xs text-muted-foreground">
                  {calculatePercentage(
                    stats.with_body_html,
                    stats.total_emails
                  )}
                  % のメール
                </p>
              </CardContent>
            </Card>

            <Card className={stats.without_body > 0 ? 'border-amber-500' : ''}>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium">本文なし</CardTitle>
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth="2"
                  className={`h-4 w-4 ${stats.without_body > 0 ? 'text-amber-500' : 'text-muted-foreground'}`}
                >
                  <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z" />
                  <path d="M12 9v4" />
                  <path d="M12 17h.01" />
                </svg>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">
                  {formatNumber(stats.without_body)}
                </div>
                <p className="text-xs text-muted-foreground">
                  {stats.without_body > 0 ? '要確認' : '問題なし'}
                </p>
              </CardContent>
            </Card>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle>平均本文長</CardTitle>
                <CardDescription>メール本文の平均文字数</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium">テキスト形式</span>
                    <span className="text-sm text-muted-foreground">
                      {formatBytes(stats.avg_plain_length)}
                    </span>
                  </div>
                  <div className="mt-2 h-2 w-full bg-secondary rounded-full overflow-hidden">
                    <div
                      className="h-full bg-blue-500 transition-all"
                      style={{
                        width: `${Math.min(100, (stats.avg_plain_length / PROGRESS_MAX_PLAIN) * 100)}%`,
                      }}
                    />
                  </div>
                </div>
                <div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium">HTML形式</span>
                    <span className="text-sm text-muted-foreground">
                      {formatBytes(stats.avg_html_length)}
                    </span>
                  </div>
                  <div className="mt-2 h-2 w-full bg-secondary rounded-full overflow-hidden">
                    <div
                      className="h-full bg-green-500 transition-all"
                      style={{
                        width: `${Math.min(100, (stats.avg_html_length / PROGRESS_MAX_HTML) * 100)}%`,
                      }}
                    />
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>データ品質</CardTitle>
                <CardDescription>
                  取り込まれたメールデータの状態
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <span className="text-sm">本文データ完全性</span>
                    <span className="text-sm font-bold">
                      {calculatePercentage(
                        stats.total_emails - stats.without_body,
                        stats.total_emails
                      )}
                      %
                    </span>
                  </div>
                  <div className="h-2 w-full bg-secondary rounded-full overflow-hidden">
                    <div
                      className={`h-full transition-all ${
                        stats.without_body === 0
                          ? 'bg-green-500'
                          : stats.without_body < stats.total_emails * 0.1
                            ? 'bg-amber-500'
                            : 'bg-red-500'
                      }`}
                      style={{
                        width: `${calculatePercentage(
                          stats.total_emails - stats.without_body,
                          stats.total_emails
                        )}%`,
                      }}
                    />
                  </div>
                  {stats.without_body > 0 && (
                    <p className="text-xs text-amber-600 dark:text-amber-400">
                      {formatNumber(stats.without_body)}{' '}
                      件のメールに本文データがありません。
                      再同期をお勧めします。
                    </p>
                  )}
                  {stats.without_body === 0 && (
                    <p className="text-xs text-green-600 dark:text-green-400">
                      全てのメールに本文データがあります。
                    </p>
                  )}
                </div>
              </CardContent>
            </Card>
          </div>
        </>
      )}

      {!stats && !loading && !error && (
        <Card>
          <CardContent className="flex items-center justify-center py-10">
            <p className="text-muted-foreground">データを読み込んでいます...</p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
