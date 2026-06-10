#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT/ts"

echo "==> pnpm install --frozen-lockfile"
pnpm install --frozen-lockfile

echo "==> pnpm -r typecheck"
pnpm -r typecheck

echo "==> pnpm -r test"
pnpm -r test

echo "==> pnpm lint"
pnpm lint

echo "==> policy sandbox negative smoke"
bash "$REPO_ROOT/harness/lint/ts-eslint/policy-sandbox-smoke.sh"
