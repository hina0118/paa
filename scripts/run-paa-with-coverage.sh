#!/bin/bash
# tauri-driver が環境変数を渡さないため、LLVM_PROFILE_FILE を設定して paa を起動するラッパー
# CI (Linux) の coverage-e2e-tauri ジョブで使用
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET_DIR="$ROOT_DIR/src-tauri/target"
BINARY="$TARGET_DIR/debug/paa"

export LLVM_PROFILE_FILE="$TARGET_DIR/src-tauri-%p-%m.profraw"
exec "$BINARY" "$@"
