import { formatDateTime } from '@/lib/utils';
import {
  formatNumber,
  formatBytes,
  calculatePercentage,
} from '@/lib/formatters';
import type { EmailStats } from '@/hooks/useDashboardStats';
import type { ParseMetadata } from '@/contexts/parse-context-value';
import type { SyncMetadata } from '@/contexts/sync-context-value';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../ui/card';

// プログレスバーの最大値（文字数）
// テキスト形式: 一般的なメールの平均的な長さを基準に5000文字
// HTML形式: HTMLタグを含むため、テキストの約4倍の20000文字
const PROGRESS_MAX_PLAIN = 5000;
const PROGRESS_MAX_HTML = 20000;

type Props = {
  emailStats: EmailStats;
  syncMetadata: SyncMetadata | null;
  parseMetadata: ParseMetadata | null;
};

export function EmailStatsSection({
  emailStats,
  syncMetadata,
  parseMetadata,
}: Props) {
  return (
    <>
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">総メール数</CardTitle>
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
              {formatNumber(emailStats.total_emails)}
            </div>
            <p className="text-xs text-muted-foreground">取り込み済みメール</p>
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
              {formatNumber(emailStats.with_body_plain)}
            </div>
            <p className="text-xs text-muted-foreground">
              {calculatePercentage(
                emailStats.with_body_plain,
                emailStats.total_emails
              )}
              % のメール
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">HTML本文あり</CardTitle>
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
              {formatNumber(emailStats.with_body_html)}
            </div>
            <p className="text-xs text-muted-foreground">
              {calculatePercentage(
                emailStats.with_body_html,
                emailStats.total_emails
              )}
              % のメール
            </p>
          </CardContent>
        </Card>

        <Card className={emailStats.without_body > 0 ? 'border-amber-500' : ''}>
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
              className={`h-4 w-4 ${emailStats.without_body > 0 ? 'text-amber-500' : 'text-muted-foreground'}`}
            >
              <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z" />
              <path d="M12 9v4" />
              <path d="M12 17h.01" />
            </svg>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {formatNumber(emailStats.without_body)}
            </div>
            <p className="text-xs text-muted-foreground">
              {emailStats.without_body > 0 ? '要確認' : '問題なし'}
            </p>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        <Card>
          <CardHeader>
            <CardTitle>同期状況</CardTitle>
            <CardDescription>Gmail からのメール取得状態</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div>
                <div className="flex items-center justify-between">
                  <span className="text-sm">ステータス</span>
                  <span
                    className={`px-2 py-1 rounded text-xs font-semibold ${
                      syncMetadata?.sync_status === 'syncing'
                        ? 'bg-blue-100 text-blue-800'
                        : syncMetadata?.sync_status === 'idle'
                          ? 'bg-green-100 text-green-800'
                          : syncMetadata?.sync_status === 'paused'
                            ? 'bg-yellow-100 text-yellow-800'
                            : syncMetadata?.sync_status === 'error'
                              ? 'bg-red-100 text-red-800'
                              : 'bg-gray-100 text-gray-800'
                    }`}
                  >
                    {syncMetadata?.sync_status === 'syncing'
                      ? '同期中'
                      : syncMetadata?.sync_status === 'idle'
                        ? '待機中'
                        : syncMetadata?.sync_status === 'paused'
                          ? '一時停止'
                          : syncMetadata?.sync_status === 'error'
                            ? 'エラー'
                            : '不明'}
                  </span>
                </div>
              </div>
              <div>
                <div className="flex items-center justify-between">
                  <span className="text-sm">総取得件数</span>
                  <span className="text-lg font-bold">
                    {formatNumber(syncMetadata?.total_synced_count ?? 0)}
                  </span>
                </div>
              </div>
              {syncMetadata?.last_sync_completed_at && (
                <p className="text-xs text-muted-foreground">
                  最終同期:{' '}
                  {formatDateTime(syncMetadata.last_sync_completed_at)}
                </p>
              )}
              {syncMetadata?.last_error_message && (
                <p className="text-xs text-red-600 dark:text-red-400">
                  エラー: {syncMetadata.last_error_message}
                </p>
              )}
            </div>
          </CardContent>
        </Card>

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
                  {formatBytes(emailStats.avg_plain_length)}
                </span>
              </div>
              <div className="mt-2 h-2 w-full bg-secondary rounded-full overflow-hidden">
                <div
                  className="h-full bg-blue-500 transition-all"
                  style={{
                    width: `${Math.min(100, (emailStats.avg_plain_length / PROGRESS_MAX_PLAIN) * 100)}%`,
                  }}
                />
              </div>
            </div>
            <div>
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">HTML形式</span>
                <span className="text-sm text-muted-foreground">
                  {formatBytes(emailStats.avg_html_length)}
                </span>
              </div>
              <div className="mt-2 h-2 w-full bg-secondary rounded-full overflow-hidden">
                <div
                  className="h-full bg-green-500 transition-all"
                  style={{
                    width: `${Math.min(100, (emailStats.avg_html_length / PROGRESS_MAX_HTML) * 100)}%`,
                  }}
                />
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>データ品質</CardTitle>
            <CardDescription>取り込まれたメールデータの状態</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-sm">本文データ完全性</span>
                <span className="text-sm font-bold">
                  {calculatePercentage(
                    emailStats.total_emails - emailStats.without_body,
                    emailStats.total_emails
                  )}
                  %
                </span>
              </div>
              <div className="h-2 w-full bg-secondary rounded-full overflow-hidden">
                <div
                  className={`h-full transition-all ${
                    emailStats.without_body === 0
                      ? 'bg-green-500'
                      : emailStats.without_body < emailStats.total_emails * 0.1
                        ? 'bg-amber-500'
                        : 'bg-red-500'
                  }`}
                  style={{
                    width: `${calculatePercentage(
                      emailStats.total_emails - emailStats.without_body,
                      emailStats.total_emails
                    )}%`,
                  }}
                />
              </div>
              {emailStats.without_body > 0 && (
                <p className="text-xs text-amber-600 dark:text-amber-400">
                  {formatNumber(emailStats.without_body)}{' '}
                  件のメールに本文データがありません。 再同期をお勧めします。
                </p>
              )}
              {emailStats.without_body === 0 && (
                <p className="text-xs text-green-600 dark:text-green-400">
                  全てのメールに本文データがあります。
                </p>
              )}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>パース状況</CardTitle>
            <CardDescription>メールからの注文情報抽出状態</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div>
                <div className="flex items-center justify-between">
                  <span className="text-sm">ステータス</span>
                  <span
                    className={`px-2 py-1 rounded text-xs font-semibold ${
                      parseMetadata?.parse_status === 'running'
                        ? 'bg-blue-100 text-blue-800'
                        : parseMetadata?.parse_status === 'completed'
                          ? 'bg-green-100 text-green-800'
                          : parseMetadata?.parse_status === 'error'
                            ? 'bg-red-100 text-red-800'
                            : 'bg-gray-100 text-gray-800'
                    }`}
                  >
                    {parseMetadata?.parse_status === 'running'
                      ? 'パース中'
                      : parseMetadata?.parse_status === 'completed'
                        ? '完了'
                        : parseMetadata?.parse_status === 'error'
                          ? 'エラー'
                          : '待機中'}
                  </span>
                </div>
              </div>
              <div>
                <div className="flex items-center justify-between">
                  <span className="text-sm">総パース件数</span>
                  <span className="text-lg font-bold">
                    {formatNumber(parseMetadata?.total_parsed_count || 0)}
                  </span>
                </div>
              </div>
              {parseMetadata?.last_parse_completed_at && (
                <p className="text-xs text-muted-foreground">
                  最終完了:{' '}
                  {formatDateTime(parseMetadata.last_parse_completed_at)}
                </p>
              )}
              {parseMetadata?.last_error_message && (
                <p className="text-xs text-red-600 dark:text-red-400">
                  エラー: {parseMetadata.last_error_message}
                </p>
              )}
            </div>
          </CardContent>
        </Card>
      </div>
    </>
  );
}
