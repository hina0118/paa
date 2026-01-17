# カバレッジ計測スクリプト (PowerShell) - cargo-llvm-cov使用

Write-Host "🧪 cargo-llvm-cov でカバレッジを計測します..." -ForegroundColor Cyan
Write-Host ""

# 古いカバレッジデータをクリーンアップ
Write-Host "古いカバレッジデータをクリーンアップ中..." -ForegroundColor Yellow
cargo llvm-cov clean

# テスト実行とカバレッジ計測
Write-Host ""
Write-Host "テストを実行してカバレッジを計測中..." -ForegroundColor Cyan
cargo llvm-cov --all-features --workspace --html

if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "❌ テストまたはカバレッジ計測が失敗しました" -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host ""
Write-Host "✅ カバレッジレポートが生成されました！" -ForegroundColor Green
Write-Host ""
Write-Host "📊 HTMLレポート: target\llvm-cov\html\index.html" -ForegroundColor Cyan
Write-Host ""
Write-Host "レポートを開くには:" -ForegroundColor Yellow
Write-Host "  start target\llvm-cov\html\index.html" -ForegroundColor White
Write-Host ""
Write-Host "他のフォーマットで出力する場合:" -ForegroundColor Gray
Write-Host "  cargo llvm-cov --lcov --output-path coverage.lcov  # LCOV形式" -ForegroundColor Gray
Write-Host "  cargo llvm-cov --json --output-path coverage.json  # JSON形式" -ForegroundColor Gray
Write-Host "  cargo llvm-cov --text                               # テキスト形式（コンソール出力）" -ForegroundColor Gray
