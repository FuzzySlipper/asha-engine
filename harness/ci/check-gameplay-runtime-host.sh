#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACT_DIR="${ASHA_GAMEPLAY_RUNTIME_HOST_ARTIFACT_DIR:-$ROOT/harness/smoke-out/gameplay-runtime-host}"
EVIDENCE="$ARTIFACT_DIR/integration-evidence.jsonl"
FAILURE_LOG="$ARTIFACT_DIR/failure.log"

mkdir -p "$ARTIFACT_DIR"
: > "$EVIDENCE"
rm -f "$FAILURE_LOG"

run_with_evidence() {
  local label="$1"
  shift
  local output
  output="$(mktemp)"
  set +e
  "$@" > >(tee "$output") 2>&1
  local status=$?
  set -e
  sed -n 's/^ASHA_GAMEPLAY_RUNTIME_HOST_EVIDENCE=//p' "$output" >> "$EVIDENCE"
  if [[ $status -ne 0 ]]; then
    {
      echo "gate=$label"
      echo "exitStatus=$status"
      tail -n 200 "$output"
    } > "$FAILURE_LOG"
    echo "Gameplay runtime host gate failed; bounded evidence: $EVIDENCE" >&2
    echo "Bounded failure tail: $FAILURE_LOG" >&2
    rm -f "$output"
    return "$status"
  fi
  rm -f "$output"
}

echo "==> Running direct gameplay RuntimeSession host integration suite"
run_with_evidence host \
  cargo test --locked --offline --manifest-path "$ROOT/engine-rs/Cargo.toml" \
    -p gameplay-runtime-host -- --nocapture

jq -s -e '
  length >= 4 and all(
    .schemaVersion == 1 and
    (.session != null) and
    (.waveOrAction | type == "string") and
    (.registryDigest | type == "string") and
    (.runtimeHostHash | type == "string") and
    (.evidenceHashes | type == "array" and length <= 8)
  )
' "$EVIDENCE" >/dev/null

echo "Gameplay runtime host integration passed."
echo "Evidence: $EVIDENCE"
