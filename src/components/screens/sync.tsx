import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";

interface FetchResult {
  fetched_count: number;
  saved_count: number;
  skipped_count: number;
}

export function Sync() {
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<FetchResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleFetchEmails = async () => {
    setLoading(true);
    setError(null);
    setResult(null);

    try {
      // Gmailからメッセージを取得してバックエンドでDBに保存
      const fetchResult = await invoke<FetchResult>("fetch_gmail_emails");
      setResult(fetchResult);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="container mx-auto py-10">
      <h1 className="text-3xl font-bold mb-6">Gmail同期</h1>

      <div className="space-y-4">
        <div>
          <Button onClick={handleFetchEmails} disabled={loading}>
            {loading ? "取得中..." : "Gmailメールを取得"}
          </Button>
        </div>

        {result && (
          <div className="p-4 border rounded-lg bg-green-50">
            <h2 className="font-semibold mb-2">取得完了</h2>
            <ul className="space-y-1 text-sm">
              <li>取得件数: {result.fetched_count}件</li>
              <li>保存件数: {result.saved_count}件</li>
              <li>スキップ件数: {result.skipped_count}件</li>
            </ul>
          </div>
        )}

        {error && (
          <div className="p-4 border rounded-lg bg-red-50 text-red-800">
            <h2 className="font-semibold mb-2">エラー</h2>
            <p className="text-sm">{error}</p>
          </div>
        )}

        <div className="p-4 border rounded-lg bg-blue-50">
          <h2 className="font-semibold mb-2">初回セットアップ</h2>
          <p className="text-sm mb-2">
            Gmail APIを使用するには、事前にGoogle Cloud Consoleでの設定が必要です。
          </p>
          <p className="text-sm mb-2">
            詳細は README.md の「Gmail API セットアップ」セクションを参照してください。
          </p>
          <div className="mt-3 p-3 bg-yellow-50 border border-yellow-200 rounded">
            <p className="text-xs font-semibold text-yellow-800 mb-1">
              初回認証について
            </p>
            <p className="text-xs text-yellow-700">
              初回実行時は、ブラウザで認証画面が自動的に開きます。
              もし開かない場合は、コンソール（開発者ツール）に表示されるURLをコピーして、
              手動でブラウザで開いてください。
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
