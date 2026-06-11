#!/usr/bin/env bash
# Mechanical guardrails for the runtime bridge surface (task #2249, ADR 0006).
#
# Rejects opaque escape hatches in STABLE bridge surfaces. test/devtools/replay
# paths are quarantined (matching manifest `surface = "quarantined"` ops).
#
# Run: bash harness/bridge/check-bridge-guardrails.sh
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

fail=0

# Stable Rust bridge surfaces.
RUST_DIRS=(
  "engine-rs/crates/bridge/runtime-bridge-api/src"
  "engine-rs/crates/bridge/native-bridge/src"
)
# Stable TS bridge surfaces.
TS_DIRS=(
  "ts/packages/runtime-bridge/src"
  "ts/packages/native-bridge/src"
)

# Quarantine: skip generated, tests, and devtools files; and skip comment lines
# (the tokens appear in doc comments describing the rules). grep output is
# `path:lineno:content` — drop lines whose content starts with //, ///, //!, *, #.
exclude_quarantine() {
  grep -vE '(/generated/|\.test\.|/__tests__/|/devtools/|/fixtures/)' \
    | grep -vE ':[0-9]+:[[:space:]]*(//|/\*|\*|#)'
}

scan() {
  local label="$1" pattern="$2" dir="$3"
  [ -d "$dir" ] || return 0
  local hits
  hits="$(grep -rnE "$pattern" "$dir" --include='*.rs' --include='*.ts' 2>/dev/null \
            | exclude_quarantine || true)"
  if [ -n "$hits" ]; then
    echo "FAIL ($label) in $dir:"
    echo "$hits" | sed 's/^/    /'
    fail=1
  fi
}

for d in "${RUST_DIRS[@]}"; do
  scan "serde_json::Value" 'serde_json::Value' "$d"
  scan "boxed trait object" 'Box<dyn ' "$d"
  scan "methodName+json dispatch" '\bcall_rust\b|\bdispatch\s*\(\s*method' "$d"
done

for d in "${TS_DIRS[@]}"; do
  scan "TS any" ':\s*any\b|as\s+any\b' "$d"
  scan "TS unknown payload" ':\s*unknown\b' "$d"
  scan "callRust dispatcher" 'callRust\s*\(' "$d"
done

if [ "$fail" -eq 0 ]; then
  echo "Bridge guardrails: OK (no opaque escape hatches in stable surfaces)"
fi
exit "$fail"
