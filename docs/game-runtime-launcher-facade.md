# Game runtime launcher facade

This note records the V1 public launcher shape for ASHA game workflows. It is the
implementation reference for tasks #3652 through #3659 and the follow-on demo/runtime
proof work in #3650.

## Decision

Game consumers launch a runtime through a canonical public facade exported from
`@asha/runtime-bridge`. The facade owns runtime construction, launch sequencing,
command receipts, replay/evidence metadata, and non-claim classification. Proof
consumers such as `asha-testing`, and product/demo consumers such as `asha-demo`,
must not instantiate `createMockRuntimeBridge()` directly for a dev runtime and
must not invent a separate JSON command tunnel.

The lower-level `RuntimeBridge` interface remains the bounded transport facade for
engine operations. The game launcher is a higher-level public composition layer around
that interface:

```ts
import {
  createReferenceGameRuntimeLauncher,
  type GameRuntimeLauncher,
  type GameRuntimeLaunchRequest,
} from '@asha/runtime-bridge';
```

The first implementation is a deterministic reference launcher. It may internally use
the existing TypeScript reference/mock bridge while ASHA lacks a live runtime entry, but
it reports `runtimeMode: "reference"` and emits explicit non-claims. `runtimeMode:
"mock"` stays reserved for unit tests and intentionally mocked fixtures, not the
user-facing demo runtime.

## Ownership

`@asha/runtime-bridge` owns:

- `GameRuntimeMode = "reference" | "native" | "degraded"`.
- `GameRuntimeLaunchRequest`: identity, workspace, manifest/runtime-entry metadata,
  fixture/world bundle input, compatibility markers, and optional deterministic clock.
- `GameRuntimeSession`: launched runtime identity, current projection/evidence readback,
  command proposal, replay/evidence export, telemetry pull, render-diff pull, and shutdown.
- `GameRuntimeCommandReceipt`: command sequence id, bounded command batch, accepted/rejected
  status, runtime error classification, authority hash before/after, and replay correlation.
- `GameRuntimeNonClaim`: explicit statements such as `not_native_runtime`,
  `not_hardware_gpu`, `not_performance_evidence`, and `not_publish_artifact`.
- fail-closed diagnostics for missing compatibility, missing world bundle, unsupported
  runtime entry, unimplemented bridge operation, rejected command, stale sequence, and
  stale readback.

`@asha/game-workspace` owns parsing and validating `asha.game.toml`, catalog/workspace
shape, and publish asset manifests. It should translate its manifest model into a
launcher request; the launcher should not become a manifest parser.

`@asha/devtools` owns the attach protocol. Devtools adapters map launcher/session read
models to `DevtoolsRuntimeIdentity`, projections, render diffs, telemetry, replay export,
and evidence export. `@asha/runtime-bridge` should not depend on `@asha/devtools`.

`asha-testing` owns synthetic proof-specific file loading, command-line ergonomics,
and conformance artifacts. It imports public package roots only, creates a reference
launcher, launches a session, and serves devtools data from the session read models.
The new `asha-demo` should reuse the same public launcher contract for human-facing
demo flows without inheriting the proof harness as its repo identity.

## Public API Shape

The exported launcher should be narrow and typed:

```ts
export type GameRuntimeMode = 'reference' | 'native' | 'degraded';

export interface GameRuntimeLaunchRequest {
  gameId: string;
  workspaceId: string;
  runtimeEntry: string;
  compatibility: GameRuntimeCompatibility;
  world: GameRuntimeWorldSource;
  startedAtIso?: string;
}

export interface GameRuntimeLauncher {
  readonly mode: GameRuntimeMode;
  launch(request: GameRuntimeLaunchRequest): Promise<GameRuntimeSession>;
}

export interface GameRuntimeSession {
  readonly identity: GameRuntimeIdentity;
  pullProjection(): Promise<GameRuntimeProjection>;
  pullRenderDiff(cursor?: number): Promise<GameRuntimeRenderDiffSnapshot>;
  pullTelemetry(): Promise<GameRuntimeTelemetrySnapshot>;
  proposeCommands(batch: CommandBatch): Promise<GameRuntimeCommandReceipt>;
  exportReplay(request: GameRuntimeReplayExportRequest): Promise<GameRuntimeReplayExport>;
  exportEvidence(request: GameRuntimeEvidenceExportRequest): Promise<GameRuntimeEvidenceExport>;
  shutdown(): Promise<void>;
}
```

The exact field names can evolve in #3653, but the boundary must stay command-shaped and
read-model-shaped. Avoid `call(methodName, json)`, private mutation callbacks, package
`src/**` imports, raw native bridge imports, or generated-schema path imports.

## Launch Sequence

The reference launcher performs the same sequence every time:

1. Validate required launch request fields and compatibility markers.
2. Create the underlying bounded `RuntimeBridge`.
3. `initializeEngine` with deterministic seed/options.
4. Load the requested world bundle or fixture through a public bridge method.
5. Read composition status and fail closed if the world is not loaded.
6. Pull the initial render/projection readback.
7. Emit a session identity with `runtimeMode: "reference"`, compatibility metadata,
   start time, authority hash, and non-claims.

The session owns command sequencing. Every proposed command batch records before/after
hashes and sequence ids even when rejected. Devtools, replay export, and evidence export
consume that same sequence log; they must not rebuild a parallel command timeline.

## Evidence And Non-Claims

The reference launcher proves the demo can launch, accept bounded public commands, project
state, export replay/evidence envelopes, and preserve compatibility markers. It does not
prove live native execution, hardware rendering, GPU timing, performance, publish validity,
or runtime-bridge readiness for production authority.

Those non-claims should be machine-readable in every dev-smoke/game-workflow artifact that
uses the reference launcher.

## Follow-On Acceptance

#3653 should add exported public types and tests that compile against the package root only.
It should also add boundary coverage in `asha-testing` so direct proof-consumer imports of
`createMockRuntimeBridge`, `MockRuntimeBridge`, `NativeRuntimeBridge`, raw transports, private
source paths, and generic method/json tunnels fail.

#3654 should implement `createReferenceGameRuntimeLauncher()` in `@asha/runtime-bridge` using
existing public bridge operations internally. The implementation must report
`runtimeMode: "reference"` and should include negative tests for missing world, unsupported
runtime entry, and failed command/readback paths.

#3655 through #3659 should then replace the proof consumer's `runtimeMode: "mock"` dev
runtime, wire devtools to the session read models, and add command/replay/non-claim
gates without weakening the existing public-boundary checks.
