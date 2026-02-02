# PR の未解決レビュースレッドを gh api graphql で解決するスクリプト
# 使用方法:
#   .\scripts\resolve-pr-review-threads.ps1 -Owner hina0118 -Repo paa -PrNumber 55 -ThreadIds @("PRRT_xxx","PRRT_yyy")
#   .\scripts\resolve-pr-review-threads.ps1  # デフォルト値で PR #55 の既知スレッドを解決
# 前提: gh がインストール済みで gh auth login 済みであること

param(
    [string]$Owner = "hina0118",
    [string]$Repo = "paa",
    [int]$PrNumber = 55,
    [string[]]$ThreadIds = @(
        "PRRT_kwDOQ3N1zs5sPPlb",   # Gemini API保存/削除のテスト追加
        "PRRT_kwDOQ3N1zs5sPPlz",   # 保存ボタン取得をaria-labelで安定化
        "PRRT_kwDOQ3N1zs5sPPmB",   # try_start()後の早期returnでfinish()呼び忘れ
        "PRRT_kwDOQ3N1zs5sPPmT"    # API失敗時のレスポンス本文ログ
    )
)

$ErrorActionPreference = "Stop"

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

Write-Host "`nPR #$PrNumber ($Owner/$Repo) の未解決レビュースレッド $($ThreadIds.Count) 件を解決します..." -ForegroundColor Cyan

$successCount = 0
$failCount = 0

foreach ($threadId in $ThreadIds) {
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
