# PR #55 の未解決レビュースレッドを gh api graphql で解決するスクリプト
# 使用方法: .\scripts\resolve-pr-review-threads.ps1
# 前提: gh がインストール済みで gh auth login 済みであること

$ErrorActionPreference = "Stop"
$owner = "hina0118"
$repo = "paa"
$prNumber = 55

# 未解決のスレッドID（レビュー指摘対応分）
$unresolvedThreadIds = @(
    "PRRT_kwDOQ3N1zs5sOopz",   # success_count の正確な集計（ParseBatchResult で対応済み）
    "PRRT_kwDOQ3N1zs5sO4Iy",   # 多重実行ガード（ProductNameParseState で対応済み）
    "PRRT_kwDOQ3N1zs5sO4I-",   # product_name NULL 時の raw_name フォールバック
    "PRRT_kwDOQ3N1zs5sO4JY",   # APIエラー時のレスポンスボディ全文ログ削除
    "PRRT_kwDOQ3N1zs5sO4J1",   # 返却件数不一致時のチャンク全体失敗扱い
    "PRRT_kwDOQ3N1zs5sO4KD"    # success_count の正確な集計（ParseBatchResult で対応済み）
)

Write-Host "gh コマンドの確認..."
$ghPath = Get-Command gh -ErrorAction SilentlyContinue
if (-not $ghPath) {
    Write-Host "エラー: gh コマンドが見つかりません。GitHub CLI をインストールし、ターミナルを再起動してください。" -ForegroundColor Red
    Write-Host "  winget install GitHub.cli" -ForegroundColor Yellow
    exit 1
}

Write-Host "gh 認証の確認..."
$authStatus = gh auth status 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "エラー: gh が認証されていません。gh auth login を実行してください。" -ForegroundColor Red
    exit 1
}

Write-Host "`n未解決のレビュースレッド $($unresolvedThreadIds.Count) 件を解決します..." -ForegroundColor Cyan

$successCount = 0
$failCount = 0

foreach ($threadId in $unresolvedThreadIds) {
    $query = "mutation { resolveReviewThread(input: {threadId: \`"$threadId\`"}) { thread { isResolved } } }"

    Write-Host "  Resolving $threadId..." -NoNewline
    try {
        $result = gh api graphql -f "query=$query" 2>&1
        if ($LASTEXITCODE -eq 0) {
            $json = $result | ConvertFrom-Json
            if ($json.data.resolveReviewThread.thread.isResolved) {
                Write-Host " OK" -ForegroundColor Green
                $successCount++
            } else {
                Write-Host " (既に解決済みの可能性)" -ForegroundColor Yellow
                $successCount++
            }
        } else {
            Write-Host " 失敗: $result" -ForegroundColor Red
            $failCount++
        }
    } catch {
        Write-Host " 失敗: $_" -ForegroundColor Red
        $failCount++
    }
    Start-Sleep -Milliseconds 500  # レート制限対策
}

Write-Host "`n完了: 成功 $successCount 件, 失敗 $failCount 件" -ForegroundColor Cyan
