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
- **load** — a bounded in-memory canonical project source loads through
  `RuntimeSession.loadProject({ source })`; diagnostics are summarized.
- **render** — a deterministic fixture frame is uploaded through the real
  `renderer-three` create → `replaceMeshPayload` path (GL-free scene graph).
- **edit and replay** — a proposed command is submitted through
  `submitCommands` (accepted/rejected both observable), the project is closed
  and reloaded from the same canonical source, and replay is checked.

A missing native/WASM/runtime capability is **classified** (see
`SmokeFailureCategory`), never a silent blank success.

## Sample output — PASS (mock facade)

```
asha-smoke: PASS [mock_reference_passed]
command: pnpm --filter @asha/smoke dev:asha-smoke
smokeMode: reference
runtimeMode: mock (nativeAvailable=true)
capabilities: runtimeBridge=mock projectLoad=mock renderer=ok projection=mock
fixture: id=1001 manifestHash=6e52b42cd0fc9373
diagnostics: total=0 fatal=0 blocksLoad=false
render: applied=true sceneNodes=1
stage load: ok — loaded canonical project 1
stage close-reload-replay: ok — closed project 1; reloaded canonical project 1; replay step 1 diverged=false
stage cleanup: ok — destroyed 2 handle(s); leakedHandles=0 outstandingBuffers=0
```

## Sample output — FAIL (load blocked by a fatal diagnostic)

```
asha-smoke: FAIL
capabilities: runtimeBridge=mock projectLoad=unavailable renderer=ok projection=unavailable
stage load: FAIL — canonical project load rejected
failure [load_failure] runtime-session.loadProject: canonical project did not load cleanly → inspect admission diagnostics for the failing artifact
```

## Programmatic use

```ts
import { runSmoke, formatResult } from '@asha/smoke';

const result = await runSmoke();
console.log(formatResult(result));
if (!result.ok) process.exit(1);
```
