# @asha/smoke — developer smoke harness

One canonical command boots the ASHA runtime facade against an abstract fixture
world, probes capability status, drives the real **load → projection → render**
path, proposes a validated edit through the authority command path, and emits a
single structured `SmokeResult` (never ambiguous console noise).

## Run it

```bash
# from the repo's ts/ directory
pnpm --filter @asha/smoke dev:asha-smoke
# or the workspace alias
pnpm dev:asha-smoke
```

Perf lanes are separate from the smoke command:

```bash
# same-host logged baseline (structural invariants gate, timings trend only)
pnpm --filter @asha/smoke dev:asha-perf

# optional discrete-GPU/WebGL context lane; skips clearly unless explicitly enabled
pnpm --filter @asha/smoke dev:asha-gpu-perf
```

Exit code is `0` on PASS, `1` on FAIL. Structured artifacts are written to
`harness/smoke-out/asha-smoke.{txt,json}` (gitignored) for closeout evidence; Den
or any external tool can link to them.

## What it proves

- **boot** — the runtime facade comes up; native availability is probed and
  reported. The canonical smoke runs on the fully-wired **mock** facade for
  determinism (the native addon is a partial prototype today); native mode is
  opt-in by injecting a `bootBridge`.
- **load** — an abstract fixture ProjectBundle (`sceneId 1001`) loads through
  the real `loadProjectBundle` facade verb; diagnostics are summarized.
- **render** — a deterministic fixture frame is uploaded through the real
  `renderer-three` create → `replaceMeshPayload` path (GL-free scene graph).
- **edit-save** — a proposed command is submitted through `submitCommands`
  (accepted/rejected both observable) and the ProjectBundle is saved via
  `saveProjectBundle`.

A missing native/WASM/runtime capability is **classified** (see
`SmokeFailureCategory`), never a silent blank success.

## Sample output — PASS (mock facade)

```
asha-smoke: PASS
command: pnpm --filter @asha/smoke dev:asha-smoke
runtimeMode: mock (nativeAvailable=true)
capabilities: runtimeBridge=mock projectBundleLoad=mock renderer=ok projection=mock
fixture: id=1001 projectBundleHash=f4a19eb318f7749d
diagnostics: total=0 fatal=0 blocksLoad=false
render: applied=true sceneNodes=1
stage boot: ok — runtime facade up in mock mode (nativeAvailable=true)
stage load: ok — loaded ProjectBundle 1001
stage render: ok — applied fixture frame; scene nodes=1
stage edit-save: ok — proposed 1 command → accepted=1 rejected=0; rejected-path visible=true; saved artifacts=3
```

## Sample output — FAIL (load blocked by a fatal diagnostic)

```
asha-smoke: FAIL
capabilities: runtimeBridge=mock projectBundleLoad=unavailable renderer=ok projection=unavailable
stage load: FAIL — load did not settle (loadedProjectBundle=null, blocksLoad=true)
failure [load_failure] runtime-bridge.loadProjectBundle: ProjectBundle 1001 did not load cleanly → inspect composition diagnostics for the failing artifact
```

## Programmatic use

```ts
import { runSmoke, formatResult } from '@asha/smoke';

const result = runSmoke();
console.log(formatResult(result));
if (!result.ok) process.exit(1);
```
