#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "==> Verifying Rust dependency graph"
bash "$REPO_ROOT/harness/depgraph/verify-rust-deps.sh"

echo "==> Rust source shape guard"
node "$REPO_ROOT/harness/depgraph/check-rust-source-shape.mjs" "$REPO_ROOT"

echo "==> Rust source shape fixtures"
node "$REPO_ROOT/harness/depgraph/check-rust-source-shape-fixtures.mjs" "$REPO_ROOT"

echo "==> Verifying TypeScript dependency graph"
bash "$REPO_ROOT/harness/depgraph/verify-ts-deps.sh"

echo "==> Runtime bridge root isolation"
node "$REPO_ROOT/harness/depgraph/check-runtime-bridge-root-isolation.mjs" "$REPO_ROOT"

echo "==> Checking generated TypeScript ESLint boundary config"
python3 "$REPO_ROOT/harness/depgraph/generate-ts-eslint-boundaries.py" --check

echo "==> Checking Agent Code Atlas inventory"
python3 "$REPO_ROOT/harness/code-map/check-agent-code-atlas.py" --check

echo "==> Running depgraph negative fixtures"
bash "$REPO_ROOT/harness/depgraph/check-negative-fixtures.sh"

echo "==> Smoke-testing TypeScript package generator"
bash "$REPO_ROOT/harness/depgraph/check-package-generator.sh"
