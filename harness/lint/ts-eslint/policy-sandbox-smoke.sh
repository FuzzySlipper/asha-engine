#!/usr/bin/env bash
# Negative smoke for the Phase 3 policy sandbox.
#
# Proves the guards actually bite: it drops a deliberately-illegal file into a
# policy package (forbidden shell import + Node built-in + wall-clock global),
# asserts that eslint AND the dependency-graph check both reject it, removes the
# file, and confirms the package is clean again. The temp file is always cleaned
# up, even on failure.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SMOKE_FILE="$REPO_ROOT/ts/packages/policy-core/src/__sandbox_smoke__.ts"

cleanup() { rm -f "$SMOKE_FILE"; }
trap cleanup EXIT

cat > "$SMOKE_FILE" <<'TS'
// Intentionally illegal policy source — used only by the sandbox smoke.
import { readFileSync } from 'node:fs';
import '@asha/renderer-babylon';

export const broken = (): number => {
  readFileSync('/etc/hostname');
  return new Date().getTime() + Math.random();
};
TS

fail() { echo "SANDBOX SMOKE FAILED: $1" >&2; exit 1; }

echo "==> eslint must reject the illegal policy file"
if (cd "$REPO_ROOT/ts" && pnpm exec eslint packages/policy-core/src/__sandbox_smoke__.ts) >/dev/null 2>&1; then
    fail "eslint accepted a file with forbidden import/global"
fi
echo "    eslint rejected it (as required)"

echo "==> dependency-graph check must reject the forbidden import"
if bash "$REPO_ROOT/harness/depgraph/verify-ts-deps.sh" >/dev/null 2>&1; then
    fail "depgraph accepted a forbidden @asha/renderer-babylon import"
fi
echo "    depgraph rejected it (as required)"

cleanup

echo "==> with the file removed, both checks must pass again"
(cd "$REPO_ROOT/ts" && pnpm exec eslint packages/policy-core packages/policy-examples) \
    || fail "policy packages do not lint clean after removing the smoke file"
bash "$REPO_ROOT/harness/depgraph/verify-ts-deps.sh" >/dev/null \
    || fail "depgraph is not clean after removing the smoke file"

echo "Policy sandbox smoke passed: forbidden import and globals are enforced."
