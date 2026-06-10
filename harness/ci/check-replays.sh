#!/usr/bin/env bash
# Checks every committed golden replay under harness/goldens/replays by playing
# it back through replay-tool against the current authority logic. Fails
# (non-zero) on the first golden that no longer reproduces; replay-tool prints
# the routed divergence report naming the replay and the diverging step.
#
# (Replay *format* fixtures with synthetic hashes live under harness/fixtures/
# and are exercised by sim-replay's unit tests, not here.)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GOLDEN_DIR="$REPO_ROOT/harness/goldens/replays"

echo "==> Building replay-tool"
cargo build --quiet --manifest-path "$REPO_ROOT/engine-rs/Cargo.toml" -p replay-tool
BIN="$REPO_ROOT/engine-rs/target/debug/replay-tool"

shopt -s nullglob
goldens=("$GOLDEN_DIR"/*.replay)
if [ ${#goldens[@]} -eq 0 ]; then
    echo "No golden replays found in $GOLDEN_DIR" >&2
    exit 1
fi

echo "==> Checking ${#goldens[@]} golden replay(s)"
for golden in "${goldens[@]}"; do
    "$BIN" check "$golden"
done

echo "All golden replays reproduced under current authority logic."
