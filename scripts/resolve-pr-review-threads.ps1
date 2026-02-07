# PR の未解決レビュースレッドを gh api graphql で解決するスクリプト（汎用版）
#
# 使用方法:
#   # モード1: PR番号のみ指定 → 未解決スレッドを自動取得して解決
#   .\scripts\resolve-pr-review-threads.ps1 -PrNumber 59
#
#   # モード2: リポジトリ＋PR番号（gitリポジトリ外で実行する場合）
#   .\scripts\resolve-pr-review-threads.ps1 -Owner hina0118 -Repo paa -PrNumber 59
#
#   # モード3: スレッドIDを明示指定
#   .\scripts\resolve-pr-review-threads.ps1 -PrNumber 59 -ThreadIds @("PRRT_xxx","PRRT_yyy")
#
#   # モード4: 現在のブランチのPRを解決（gh pr view でPR番号を取得）
#   .\scripts\resolve-pr-review-threads.ps1 -CurrentBranch
#
# 前提: gh がインストール済みで gh auth login 済みであること
# インストール: winget install GitHub.cli

param(
    [string]$Owner = "",
    [string]$Repo = "",
    [int]$PrNumber = 0,
    [switch]$CurrentBranch,
    [string[]]$ThreadIds = @()
)

$ErrorActionPreference = "Stop"

# ヘルプ表示
function Show-Usage {
    Write-Host ""
    Write-Host "PR の未解決レビュースレッドを Resolved にします。" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "使用例:" -ForegroundColor Cyan
    Write-Host '  .\scripts\resolve-pr-review-threads.ps1 -PrNumber 59'
    Write-Host '  .\scripts\resolve-pr-review-threads.ps1 -Owner hina0118 -Repo paa -PrNumber 59'
    Write-Host '  .\scripts\resolve-pr-review-threads.ps1 -CurrentBranch'
    Write-Host '  .\scripts\resolve-pr-review-threads.ps1 -PrNumber 59 -ThreadIds @("PRRT_xxx","PRRT_yyy")'
    Write-Host ""
    Write-Host "パラメータ:" -ForegroundColor Cyan
    Write-Host "  -PrNumber      PR番号（必須、-CurrentBranch 使用時は不要）"
    Write-Host "  -Owner         リポジトリオーナー（省略時は gh repo view から取得）"
    Write-Host "  -Repo          リポジトリ名（省略時は gh repo view から取得）"
    Write-Host "  -CurrentBranch 現在のブランチのPRを対象にする"
    Write-Host "  -ThreadIds     解決するスレッドIDの配列（省略時は未解決スレッドを自動取得）"
    Write-Host ""
}

# gh の存在確認
$ghPath = Get-Command gh -ErrorAction SilentlyContinue
if (-not $ghPath) {
    Write-Host "Error: gh not found. Install: winget install GitHub.cli" -ForegroundColor Red
    exit 1
}

# gh 認証確認
$authStatus = gh auth status 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "Error: gh not authenticated. Run: gh auth login" -ForegroundColor Red
    exit 1
}

# Owner/Repo の取得（未指定時）
if (-not $Owner -or -not $Repo) {
    $nameWithOwner = $null
    try {
        $repoInfo = gh repo view --json nameWithOwner 2>$null | ConvertFrom-Json
        $nameWithOwner = $repoInfo.nameWithOwner
    } catch { }
    if (-not $nameWithOwner) {
        $remoteUrl = git config --get remote.origin.url 2>$null
        if ($remoteUrl -match 'github\.com[:/]([^/]+)/([^/.]+)') {
            $nameWithOwner = "$($Matches[1])/$($Matches[2])"
        }
    }
    if ($nameWithOwner) {
        $parts = $nameWithOwner -split "/"
        if (-not $Owner) { $Owner = $parts[0] }
        if (-not $Repo) { $Repo = $parts[1] }
    }
    if (-not $Owner -or -not $Repo) {
        Write-Host 'Error: Could not get repo. Specify -Owner and -Repo.' -ForegroundColor Red
        exit 1
    }
}

# -CurrentBranch の場合は PR 番号を取得
if ($CurrentBranch) {
    $prJson = gh pr view --json number 2>&1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Error: No PR for current branch" -ForegroundColor Red
        exit 1
    }
    $PrNumber = $prJson.number
}

# PrNumber 必須チェック（ThreadIds のみ指定の場合は PrNumber は表示用）
if ($PrNumber -le 0 -and $ThreadIds.Count -eq 0) {
    Show-Usage
    Write-Host "Error: Specify -PrNumber or -ThreadIds" -ForegroundColor Red
    exit 1
}

# ThreadIds が空の場合、GraphQL で未解決スレッドを取得
if ($ThreadIds.Count -eq 0) {
    Write-Host "PR #$PrNumber ($Owner/$Repo) の未解決スレッドを取得中..." -ForegroundColor Cyan

    $fetchQuery = 'query { repository(owner: "' + $Owner + '", name: "' + $Repo + '") { pullRequest(number: ' + $PrNumber + ') { reviewThreads(first: 100) { nodes { id isResolved } } } } }'

    $fetchResult = gh api graphql -f "query=$fetchQuery" 2>&1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Error: Failed to fetch PR info" -ForegroundColor Red
        exit 1
    }

    $threads = $fetchResult.data.repository.pullRequest.reviewThreads.nodes
    $ThreadIds = @($threads | Where-Object { -not $_.isResolved } | ForEach-Object { $_.id })

    if ($ThreadIds.Count -eq 0) {
        Write-Host "No unresolved threads" -ForegroundColor Green
        exit 0
    }

    Write-Host "Found $($ThreadIds.Count) unresolved thread(s)" -ForegroundColor Cyan
}

Write-Host "`nPR #$PrNumber ($Owner/$Repo) のレビュースレッド $($ThreadIds.Count) 件を解決します..." -ForegroundColor Cyan

$successCount = 0
$failCount = 0

foreach ($threadId in $ThreadIds) {
    $resolveQuery = 'mutation { resolveReviewThread(input: {threadId: "' + $threadId + '"}) { thread { isResolved } } }'

    Write-Host "  Resolving $threadId..." -NoNewline
    try {
        $result = gh api graphql -f "query=$resolveQuery" 2>&1
        if ($LASTEXITCODE -eq 0) {
            $json = $result | ConvertFrom-Json
            if ($json.data.resolveReviewThread.thread.isResolved) {
                Write-Host " OK" -ForegroundColor Green
                $successCount++
            } else {
                Write-Host " (already resolved?)" -ForegroundColor Yellow
                $successCount++
            }
        } else {
            Write-Host " Failed: $result" -ForegroundColor Red
            $failCount++
        }
    } catch {
        Write-Host " Failed: $_" -ForegroundColor Red
        $failCount++
    }
    Start-Sleep -Milliseconds 500  # レート制限対策
}

Write-Host "`nDone: $successCount ok, $failCount failed" -ForegroundColor Cyan
