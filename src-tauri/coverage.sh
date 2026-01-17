#!/bin/bash
# カバレッジ計測スクリプト (Bash) - cargo-llvm-cov使用

set -e

echo "🧪 cargo-llvm-cov でカバレッジを計測します..."
echo ""

# 古いカバレッジデータをクリーンアップ
echo "古いカバレッジデータをクリーンアップ中..."
cargo llvm-cov clean

# テスト実行とカバレッジ計測
echo ""
echo "テストを実行してカバレッジを計測中..."
cargo llvm-cov --all-features --workspace --html

echo ""
echo "✅ カバレッジレポートが生成されました！"
echo ""
echo "📊 HTMLレポート: target/llvm-cov/html/index.html"
echo ""
echo "レポートを開くには:"
echo "  open target/llvm-cov/html/index.html  # macOS"
echo "  xdg-open target/llvm-cov/html/index.html  # Linux"
echo "  start target/llvm-cov/html/index.html  # Windows (Git Bash)"
echo ""
echo "他のフォーマットで出力する場合:"
echo "  cargo llvm-cov --lcov --output-path coverage.lcov  # LCOV形式"
echo "  cargo llvm-cov --json --output-path coverage.json  # JSON形式"
echo "  cargo llvm-cov --text                               # テキスト形式（コンソール出力）"
