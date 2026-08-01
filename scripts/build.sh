#!/usr/bin/env bash
# =====================================================================
# ScreenTune 本地开发质量门禁（macOS / Linux）
#
# 用法：
#   scripts/build.sh          # 完整检查（fmt + clippy + test + check）
#   scripts/build.sh --fast   # 仅 cargo check
# =====================================================================
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ "${1:-}" == "--fast" ]]; then
  echo "==> cargo check --workspace"
  cargo check --workspace
  exit 0
fi

echo "==> 1/4 cargo fmt --check"
cargo fmt --all -- --check

echo "==> 2/4 cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> 3/4 cargo test"
cargo test --workspace

echo "==> 4/4 cargo check（release）"
cargo check --workspace --release

echo ""
echo "✅ 全部质量门禁通过"
