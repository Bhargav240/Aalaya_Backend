#!/usr/bin/env bash
# Full quality gate: format, lints, tests. Exits non-zero on any failure.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "== cargo fmt --check =="
cargo fmt --all -- --check

echo "== cargo clippy -D warnings =="
cargo clippy --workspace --all-targets -- -D warnings

echo "== cargo test =="
cargo test --workspace

echo "OK"
