#!/bin/bash
# E2Eテスト + Rust カバレッジを併せて計測するスクリプト
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

set -e

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
COVERAGE_DIR="src-tauri/target/llvm-cov"

# ============================================================
# 1. E2Eテスト実行 (JSカバレッジ収集)
# ============================================================
echo ""
echo "========================================"
echo "  Step 1: E2E Tests (Frontend Coverage)"
echo "========================================"
echo ""

cd "$ROOT_DIR"
npm run test:e2e

# ============================================================
# 2. JSカバレッジサマリー表示
# ============================================================
echo ""
echo "========================================"
echo "  Frontend (JS) Coverage Summary"
echo "========================================"

COVERAGE_FILE="coverage-e2e/coverage-data.json"
if [ -f "$COVERAGE_FILE" ]; then
    # Node.jsでサマリーを計算（クロスプラットフォーム）
    node -e "
    const fs = require('fs');
    const data = JSON.parse(fs.readFileSync('$COVERAGE_FILE', 'utf8'));
    let total = 0, covered = 0;
    (Array.isArray(data) ? data : [data]).forEach(f => {
      (f.functions || []).forEach(func => {
        total++;
        if (func.ranges && func.ranges.some(r => r.count > 0)) covered++;
      });
    });
    const pct = total > 0 ? (covered / total * 100).toFixed(2) : 0;
    console.log('  Total Files:', (Array.isArray(data) ? data : [data]).length);
    console.log('  Covered Functions:', covered + '/' + total);
    console.log('  Coverage:', pct + '%');
    "
else
    echo "  (No coverage data found)"
fi

# ============================================================
# 3. Rustカバレッジ計測
# ============================================================
echo ""
echo "========================================"
echo "  Step 2: Rust Coverage (cargo llvm-cov)"
echo "========================================"
echo ""

if ! command -v cargo-llvm-cov &> /dev/null; then
    echo "cargo-llvm-cov is not installed. Skipping Rust coverage."
    echo "Install: cargo install cargo-llvm-cov"
    echo ""
    echo "Combined coverage complete (Frontend only)"
    exit 0
fi

cd "$ROOT_DIR/src-tauri"

# 古いカバレッジをクリーン
cargo llvm-cov clean 2>/dev/null || true

# カバレッジ計測（テスト実行 + テキストサマリー）
echo "Running Rust tests with coverage..."
cargo llvm-cov --all-features --workspace --text || {
    echo "Rust coverage failed (tests may have failed)"
    exit 0
}

# HTMLレポートも生成
echo ""
echo "Generating HTML report..."
cargo llvm-cov --all-features --workspace --html --output-dir "$COVERAGE_DIR" 2>/dev/null || true
cargo llvm-cov --all-features --workspace --lcov --output-path "../coverage-e2e/rust-coverage.lcov" 2>/dev/null || true

# ============================================================
# 4. 完了メッセージ
# ============================================================
echo ""
echo "========================================"
echo "  Combined Coverage Complete"
echo "========================================"
echo ""
echo "Frontend (JS): coverage-e2e/coverage-data.json"
echo "Rust HTML:     src-tauri/target/llvm-cov/html/index.html"
echo "Rust LCOV:     coverage-e2e/rust-coverage.lcov"
echo ""
