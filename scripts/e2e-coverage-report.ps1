# E2Eテストのカバレッジレポートを生成するスクリプト

$ErrorActionPreference = "Stop"

Write-Host "📊 E2Eテストのカバレッジレポートを生成中..." -ForegroundColor Cyan

# E2Eテストを実行（カバレッジ収集付き）
Write-Host "🧪 E2Eテストを実行中..." -ForegroundColor Cyan
cmd /c npm run test:e2e

if ($LASTEXITCODE -ne 0) {
    Write-Host "⚠️  テストが失敗しましたが、カバレッジデータは収集されている可能性があります。" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "📊 カバレッジレポート:" -ForegroundColor Cyan
Write-Host "   注意: Playwrightの組み込みカバレッジはJS/CSSカバレッジのみです。" -ForegroundColor Gray
Write-Host "   より詳細なカバレッジ（行・分岐・関数）が必要な場合は、" -ForegroundColor Gray
Write-Host "   vite-plugin-istanbul などの追加設定が必要です。" -ForegroundColor Gray
