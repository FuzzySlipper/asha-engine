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

# Rust often keeps unit tests in a trailing `#[cfg(test)] mod tests` within a
# stable source file. Treat that module like a separate test file without
# excluding earlier production items that may also carry `#[cfg(test)]`.
exclude_inline_test_modules() {
  declare -A test_module_start_by_file=()
  local hit path remainder line test_module_start
  while IFS= read -r hit; do
    [ -n "$hit" ] || continue
    path="${hit%%:*}"
    remainder="${hit#*:}"
    line="${remainder%%:*}"
    if [[ "$path" != *.rs ]]; then
      printf '%s\n' "$hit"
      continue
    fi
    if [[ ! -v "test_module_start_by_file[$path]" ]]; then
      test_module_start_by_file["$path"]="$(awk '
        previous == "#[cfg(test)]" && $0 ~ /^mod[[:space:]]+tests[[:space:]]*\{/ {
          print NR - 1
          exit
        }
        { previous = $0 }
      ' "$path")"
    fi
    test_module_start="${test_module_start_by_file[$path]}"
    if [ -z "$test_module_start" ] || [ "$line" -lt "$test_module_start" ]; then
      printf '%s\n' "$hit"
    fi
  done
}

scan() {
  local label="$1" pattern="$2" dir="$3" extra_exclude="${4:-}"
  [ -d "$dir" ] || return 0
  local hits
  hits="$(grep -rnE "$pattern" "$dir" --include='*.rs' --include='*.ts' 2>/dev/null \
            | exclude_inline_test_modules \
            | exclude_quarantine || true)"
  if [ -n "$extra_exclude" ]; then
    hits="$(printf '%s\n' "$hits" | grep -vE "$extra_exclude" || true)"
  fi
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
  # `unknown` is forbidden as an escape hatch, EXCEPT in render-decode.ts — that is the
  # payload-validation gate where `unknown`→typed `@asha/contracts` values is the
  # sanctioned pattern (a decoder, the opposite of an untyped escape hatch).
  scan "TS unknown payload" ':\s*unknown\b' "$d" 'render-decode\.ts'
  scan "callRust dispatcher" 'callRust\s*\(' "$d"
done

# Capability ports are fixed compile-time subsets of the one public root. A
# port may not inherit the full root or become a second dynamically dispatched
# RuntimeBridge surface.
scan \
  "capability port inherits full RuntimeBridge" \
  'interface Runtime[A-Za-z]+Port[[:space:]]+extends[[:space:]]+RuntimeBridge' \
  "ts/packages/runtime-bridge/src"
scan \
  "dynamic capability port lookup" \
  'runtimeBridgePorts[[:space:]]*\([^)]*,|RuntimeBridgePorts\[[^]]+\]' \
  "ts/packages/runtime-bridge/src"

if [ "$fail" -eq 0 ]; then
  echo "Bridge guardrails: OK (no opaque escape hatches in stable surfaces)"
fi
exit "$fail"
