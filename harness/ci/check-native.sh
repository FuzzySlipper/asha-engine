#!/usr/bin/env bash
# Build the napi-rs native addon and verify it round-trips from TS (ADR 0006, #2250).
#
# OPT-IN: not part of check-all.sh — it needs the native toolchain + (first run)
# network to fetch napi crates. Run it where those are available.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CRATE_DIR="$REPO_ROOT/engine-rs/crates/bridge/native-bridge"
DEST="$REPO_ROOT/ts/packages/native-bridge/dist/native-bridge.node"

echo "==> Building native-bridge cdylib (release)"
( cd "$CRATE_DIR" && cargo build --release )

echo "==> Installing addon -> $DEST"
mkdir -p "$(dirname "$DEST")"
# cdylib is libnative_bridge.so on Linux / .dylib on macOS / native_bridge.dll on Windows.
ARTIFACT="$(find "$CRATE_DIR/target/release" -maxdepth 1 \
  \( -name 'libnative_bridge.so' -o -name 'libnative_bridge.dylib' -o -name 'native_bridge.dll' \) \
  | head -1)"
if [ -z "$ARTIFACT" ]; then
  echo "FAIL: no cdylib artifact found in $CRATE_DIR/target/release" >&2
  exit 1
fi
cp "$ARTIFACT" "$DEST"

echo "==> Native addon smoke (every export, parity with ReferenceBridge)"
node --input-type=module -e "
import { strict as assert } from 'node:assert';
import { createRequire } from 'node:module';
const require = createRequire('file://$DEST');
const a = require('$DEST');
assert.deepEqual(Object.keys(a).sort(), [
  'getCompositionStatus',
  'initializeEngine',
  'loadWorldBundle',
  'readRenderDiffs',
  'saveCurrentWorld',
  'stepSimulation',
  'submitCommands',
]);
const h = a.initializeEngine(7);
assert.equal(typeof h, 'number');
assert.deepEqual(a.loadWorldBundle(h, 1, 1, 1001), { loadedWorld: 1001, fatalCount: 0, totalCount: 0, blocksLoad: false });
assert.deepEqual(a.submitCommands(h, JSON.stringify([{ op: 'setVoxel', grid: 1, coord: { x: 0, y: 0, z: 0 }, value: { kind: 'solid', material: 1 } }])), { accepted: 1, rejected: 0, rejections: [] });
assert.equal(a.stepSimulation(h, 6), 2);    // tick 6 % 4 == 2, matches ReferenceBridge
assert.deepEqual(a.readRenderDiffs(h, 0), { ops: [] });
assert.deepEqual(a.saveCurrentWorld(h), { artifactsWritten: 3, compactedEdits: 0, retainedEdits: 0 });
assert.deepEqual(a.getCompositionStatus(h), { loadedWorld: 1001, fatalCount: 0, totalCount: 0, blocksLoad: false });
console.log('Native addon smoke: OK');
"

echo "==> runtime-bridge facade tests (native parity test now runs, not skipped)"
( cd "$REPO_ROOT/ts" && pnpm --filter @asha/runtime-bridge test )
