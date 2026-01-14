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
      const fetchResult = await invoke<FetchResult>("fetch_gmail_emails");
      setResult(fetchResult);
    } catch (err) {
      setError(err as string);
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
          <p className="text-sm">
            詳細は README.md の「Gmail API セットアップ」セクションを参照してください。
          </p>
        </div>
      </div>
    </div>
  );
}
