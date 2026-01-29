# E2Eテスト + Rust カバレッジを併せて計測するスクリプト (PowerShell)
#
# 実行内容:
#   1. E2Eテスト (Playwright) - フロントエンドJSカバレッジを収集
#   2. Rust ユニット/統合テスト (cargo llvm-cov) - Rustカバレッジを収集
#   3. 両方のサマリーを表示
#
# 使用方法:
#   npm run test:e2e:rust-coverage
#
# 前提条件:
#   - cargo-llvm-cov がインストールされていること (Rustカバレッジ用)
#     cargo install cargo-llvm-cov

$ErrorActionPreference = "Stop"

$RootDir = $PSScriptRoot + "\.."
$CoverageDir = "src-tauri\target\llvm-cov"

# ============================================================
# 1. E2Eテスト実行 (JSカバレッジ収集)
# ============================================================
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Step 1: E2E Tests (Frontend Coverage)" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Set-Location $RootDir
npm run test:e2e

if ($LASTEXITCODE -ne 0) {
    Write-Host "E2E tests failed" -ForegroundColor Red
    exit $LASTEXITCODE
}

# ============================================================
# 2. JSカバレッジサマリー表示
# ============================================================
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Frontend (JS) Coverage Summary" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

$CoverageFile = "coverage-e2e\coverage-data.json"
if (Test-Path $CoverageFile) {
    try {
        $coverageData = Get-Content $CoverageFile -Encoding UTF8 | ConvertFrom-Json
        $totalFiles = $coverageData.Count
        $totalFunctions = 0
        $coveredFunctions = 0

        foreach ($file in $coverageData) {
            if ($file.functions -and $file.functions.Count -gt 0) {
                foreach ($func in $file.functions) {
                    $totalFunctions++
                    $hasCoverage = $false
                    if ($func.ranges -and $func.ranges.Count -gt 0) {
                        foreach ($range in $func.ranges) {
                            if ($range.count -gt 0) {
                                $hasCoverage = $true
                                break
                            }
                        }
                    }
                    if ($hasCoverage) { $coveredFunctions++ }
                }
            }
        }

        $jsCoverage = if ($totalFunctions -gt 0) {
            [math]::Round(($coveredFunctions / $totalFunctions) * 100, 2)
        } else { 0 }

        Write-Host "  Total Files: $totalFiles" -ForegroundColor Gray
        Write-Host "  Covered Functions: $coveredFunctions / $totalFunctions" -ForegroundColor Gray
        Write-Host "  Coverage: $jsCoverage%" -ForegroundColor $(if ($jsCoverage -ge 50) { "Green" } else { "Yellow" })
    } catch {
        Write-Host "  (Could not parse coverage data)" -ForegroundColor Yellow
    }
} else {
    Write-Host "  (No coverage data found)" -ForegroundColor Yellow
}

# ============================================================
# 3. Rustカバレッジ計測
# ============================================================
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Step 2: Rust Coverage (cargo llvm-cov)" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$cargoLlvmCov = Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue
if (-not $cargoLlvmCov) {
    Write-Host "cargo-llvm-cov is not installed. Skipping Rust coverage." -ForegroundColor Yellow
    Write-Host "Install: cargo install cargo-llvm-cov" -ForegroundColor Gray
    Write-Host ""
    Write-Host "Combined coverage complete (Frontend only)" -ForegroundColor Green
    exit 0
}

Set-Location $RootDir\src-tauri

# 古いカバレッジをクリーン
$ErrorActionPreference = "Continue"
cargo llvm-cov clean 2>$null | Out-Null

# カバレッジ計測（テスト実行 + レポート生成）
# 注意: cargo-llvm-cov は info を stderr に出力するため、cmd 経由で実行
Write-Host "Running Rust tests with coverage..." -ForegroundColor Gray
$rustOutput = cmd /c "cargo llvm-cov --all-features --workspace --text 2>&1"
$rustExitCode = $LASTEXITCODE
$ErrorActionPreference = "Stop"

if ($rustExitCode -ne 0) {
    Write-Host $rustOutput
    Write-Host "Rust coverage failed (tests may have failed)" -ForegroundColor Yellow
    exit 0
}

# Rustサマリーを表示
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Rust Coverage Summary" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host $rustOutput

# HTMLレポートも生成
Write-Host ""
Write-Host "Generating HTML report..." -ForegroundColor Gray
$ErrorActionPreference = "Continue"
cargo llvm-cov --all-features --workspace --html --output-dir $CoverageDir 2>$null | Out-Null
cargo llvm-cov --all-features --workspace --lcov --output-path "..\coverage-e2e\rust-coverage.lcov" 2>$null | Out-Null
$ErrorActionPreference = "Stop"

# ============================================================
# 4. 完了メッセージ
# ============================================================
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "  Combined Coverage Complete" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Frontend (JS): coverage-e2e\coverage-data.json" -ForegroundColor Cyan
Write-Host "Rust HTML:     src-tauri\target\llvm-cov\html\index.html" -ForegroundColor Cyan
Write-Host "Rust LCOV:     coverage-e2e\rust-coverage.lcov" -ForegroundColor Cyan
Write-Host ""
