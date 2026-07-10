# RuntimeSession Facade Status

Status: current public semantic facade. Rust-backed RuntimeSession authority is the product/live path; the explicit reference helper remains a fixture and compatibility surface only.

## Public Import Path

Consumers import semantic RuntimeSession readouts, proposal envelopes, and helper
projections from `@asha/runtime-session`. Product/live bridge-backed facade
construction, launcher types, and transport surfaces stay on the
`@asha/runtime-bridge` package root. Reference/mock helpers live behind the
explicit reference entrypoint and carry a fixture-only backend profile:

```ts
import {
  type RuntimeActionIntentEnvelope,
  type RuntimeSessionFacade,
} from '@asha/runtime-session';
import { createRuntimeSessionFacade } from '@asha/runtime-bridge';
import {
  REFERENCE_RUNTIME_BACKEND_PROFILE,
  createMockRuntimeSession,
} from '@asha/runtime-bridge/reference';
```

No consumer should import package internals, raw native transports, generated file paths, or Rust crate paths.
Demo and Studio live/default flows should not treat `@asha/runtime-bridge/reference` as product authority. Its `REFERENCE_RUNTIME_BACKEND_PROFILE.productAuthority` value is `false`, and reference RuntimeSession identities include `not_product_authority`.
Game-rules reference readouts are compatibility fixtures; product/live authority
is the Rust-backed bridge path through `svc-game-rules` and the modifier rule
substrate.

#5506 completes the package split started by #4547: transport-neutral
`RuntimeSessionFacade` contracts, capability DTOs, semantic readouts, proposal
envelopes, and helper projections are owned by `@asha/runtime-session` and
exported through its package root. `@asha/runtime-bridge` no longer re-exports
that semantic surface. It retains concrete adapter construction, native/reference
transport selection, launchers, render decode, reference helpers, and generated
bridge operation conformance.

The cross-surface consumer proof for #4053 lives in
`ts/packages/smoke/src/public-consumer-compat.test.ts`. It intentionally imports
only package roots and is the current evidence that `asha-demo` can consume the
RuntimeSession readout/HUD surfaces without private ASHA paths.

## Current API

`RuntimeSessionFacade` exposes the same semantic methods in explicit backend
modes. Product/live consumers create the facade from the package root with a
Rust-capable bridge and `mode: 'rust'`; reference fixtures use
`createMockRuntimeSession()` from `@asha/runtime-bridge/reference`.

`RuntimeSessionFacade` exposes:

- `initialize(input)`: validates semantic session/project input, initializes the bridge, and loads a ProjectBundle-shaped request.
- `loadEcrpProject(input)`: validates and loads ProjectBundle-shaped ECRP content (`ProjectBundle`, `EntityDefinition[]`, `SceneDocument` placements, and optional generated `GameRuleModuleManifest[]` declarations). Rust-backed sessions route bootstrap through the bridge authority surface, forward compatible game-rule module manifests to the FPS RuntimeSession load request, and return Rust provenance/read sets; reference sessions keep fixture/project-state compatibility. Malformed declarations and rejected loads mutate nothing.
- `submitCommands(batch)`: submits generated `CommandBatch` values only.
- `tick(input?)`: advances deterministic runtime ticks through the bridge.
- `createCamera(request)`: creates a typed bridge-owned camera.
- `applyFirstPersonCameraInput(envelope)`: applies unconstrained first-person camera motion/look input.
- `applyCollisionConstrainedCameraInput(envelope)`: applies first-person camera motion/look input through the typed collision bridge surface and returns a receipt with collided, blocked axes, world/collision projection hashes, movement hash, and the generated before/attempted/after `CameraCollisionSnapshot`.
- `submitRuntimeActionIntent(envelope)`: accepts a typed `RuntimeActionIntentEnvelope` proposal. Rust-backed sessions route accepted `primary_fire` pressed intents through the Rust bridge authority surface and return combat/fire/health provenance; reference sessions return labelled fixture/reference receipts. Unsupported action intents fail closed with typed receipts.
- `validateGameRuleCatalog(catalog)`: validates generated game-rules catalog DTOs through the bounded bridge surface. Rust-backed sessions call `svc-game-rules`; reference sessions are labelled fixture/reference compatibility. Invalid catalogs return typed diagnostics and trace/evidence, not raw JSON errors.
- `submitGameRuleEffectIntent(catalog, request)`: submits one generated `GameRuleResolutionRequest` against a generated catalog. Accepted receipts carry pending value deltas, applied modifier readouts, trace, evidence refs, and replay hash; rejected requests fail closed with typed diagnostics.
- `readGameRuleRuntimeReadout()`: reads the bounded recent game-rules projection: active modifiers, recent trace entries, recent replay hashes, and latest replay hash. Periodic modifier readouts include source, target, next tick, expiration tick, stack count, and source hash.
- `runAutonomousPolicyTick(input)`: advances a narrow generated-tunnel enemy policy loop, validates typed movement/fire proposals, routes primary fire through runtime action authority, and reports proposal counts, nav/replay hashes, movement/combat summaries, backend provenance, and deterministic tick hash.
- `readLifecycleStatus(request?)`: reads player/enemy lifecycle status, win/loss/in-progress outcome, restart eligibility, fixture reset hash, lifecycle/replay hashes, and terminal death events.
- `requestSessionRestart(intent)`: validates a typed `runtime.restart_session_intent`, rejects stale/non-terminal requests with typed receipts, or resets the session deterministically through the existing restart path.
- `readCombatReadout(request?)`: reads the committed #4040 generated-tunnel combat fixture readouts for compatibility/golden evidence. Runtime action receipts use the loaded RuntimeSession state when a project has been loaded.
- `readGeneratedTunnelReadout(request?)`: reads the #4038 tiny generated tunnel fixture evidence, including seed, config/output/replay hashes, spawn markers, material roles, and render/collision projection hashes.
- `requestGeneratedTunnelOperation(request)`: on Rust-backed sessions, `apply_to_runtime_world` installs the selected `svc-levelgen` tunnel as voxel collision authority and returns its authoritative grid/output/projection hashes. `regenerate` remains fail-closed as an authoring operation; reference sessions claim no runtime apply authority.
- `readNavProjection()`: reads #4041 generated-tunnel nav projection availability/hash evidence.
- `queryNavPath(request?)`: returns reachable or no-path generated-tunnel path readouts.
- `readNavPolicyView()`: returns a read-only/proposal-only policy-facing nav view shape with no mutation/apply authority.
- `readCameraProjection(request)`: reads typed camera projection matrices and projection hash.
- `registerVoxelConversionSource(request)`: registers typed voxel conversion source geometry and material slots through the Rust-owned runtime bridge source registry. Rust-backed sessions delegate to the native/runtime bridge authority surface; reference sessions fail closed with `operation_unimplemented`.
- `registerVoxelConversionMeshAsset(request)`: registers a ProjectBundle/catalog static mesh asset as a Rust authority-visible voxel conversion source. The mesh asset path validates source identity, primitive support, indexed triangle groups, and material slots before the source can be planned; reference sessions fail closed.
- `readVoxelConversionSourceMetadata(request)`: reads Rust-owned metadata for a registered/project mesh conversion source, including source path/hash, material slots, primitive/group counts and bounds, and the latest planned transform when one exists. Missing or stale source metadata returns typed diagnostics instead of requiring Studio to infer from catalog paths.
- `planVoxelConversion(request)`: requests a Rust-owned mesh-to-voxel conversion plan using generated voxel conversion DTOs. Material maps may include authority-visible texture sample assets and per-slot UV sample bindings for the first nearest-texel `palette_index_u16` sampling slice; missing texture snapshots, stale hashes, unsupported policies, and malformed rules return typed conversion diagnostics. Rust-backed sessions route through the native/runtime bridge authority surface; reference sessions fail closed with `operation_unimplemented`.
- `previewVoxelConversion(request)`: requests bounded conversion preview output for a previously planned conversion, guarded by the expected plan hash. Rust-backed sessions return typed diagnostics for stale plan hashes; reference sessions fail closed.
- `applyVoxelConversion(request)`: requests authority application of a validated conversion plan/preview pair. Rust-backed sessions preserve plan/preview hash guards and apply accepted output through the upstream voxel command path; mismatched previews or unsupported authority target grids return classified diagnostics.
- `exportVoxelConversionEvidence(evidence)`: requests export of selected generated voxel conversion evidence refs from the current Rust authority conversion state. Unknown evidence refs fail closed.
- `readVoxelModelInfo(request)`: reads bounded authority-owned model information for an applied voxel conversion target, including identity, bounds, voxel count, optional material counts, source/evidence refs, plan/output hashes, session hash, replay hash, and typed diagnostics for missing or unknown models.
- `readVoxelModelWindow(request)`: reads a quota-guarded voxel-space sample window from an applied authority-owned model. Bounds, material filters, empty-cell inclusion, and max sample counts are validated by Rust; oversized or malformed reads return typed diagnostics instead of dumping full volumes.
- `exportVoxelVolumeAsset(request)`: exports a complete resident converted voxel model as an Asha-native `VoxelVolumeAsset` receipt with Rust-owned sparse runs, named material palette/catalog bindings, provenance refs, canonical JSON, and `svc-voxel-asset` hashes. Missing models, stale session hashes, export limits, duplicate palette bindings, and unrepresentable material refs fail closed through typed voxel-asset diagnostics.
- `saveVoxelVolumeAsset(request)`: validates an explicit runtime-to-stored voxel asset transaction for a ProjectBundle target path, returning a stored-asset diff and canonical payload for host persistence. The operation fails closed for stale runtime hashes, invalid paths/asset ids, representation/hash mismatches, export limits, and missing material refs without silently promoting SessionState into stored content.
- `updateVoxelVolumeAssetPalette(request)`: validates a bounded replacement of one stored `VoxelVolumeAsset.materialPalette` under required canonical/voxel hash guards. Rust rejects malformed or duplicate palette/catalog binding ids, invalid material refs, stale assets, and oversized palettes, then returns the updated stored asset, canonical payload, and ProjectBundle diff without touching runtime SessionState.
- `loadVoxelVolumeAsset(request)`: validates a stored `VoxelVolumeAsset` through `svc-voxel-asset` and explicitly loads it into runtime voxel authority with a receipt/readback. Invalid hashes, schema/media mismatches, material refs, and target grid mismatches fail closed before runtime mutation.
- `validateVoxelAnnotationLayer(request)`: accepts an explicit generated `VoxelAnnotationLayerValidationInput` lifecycle. `draft` carries a hashless `VoxelAnnotationLayerDraft`; Rust validates its target, bounds, sparse membership, quotas, and parent tree and returns a fully normalized `VoxelAnnotationLayer` with authority hashes. `finalized` validates an already stored layer and rejects stale or incorrect hashes. The tagged input prevents mixed draft/finalized payloads.
- `loadVoxelAnnotationLayer(request)`: explicitly loads a validated annotation layer into runtime annotation authority for a loaded voxel-volume asset. Target session hash mismatches, missing target volumes, invalid layer content, and replace conflicts return typed annotation diagnostics without mutating runtime annotation state.
- `readVoxelAnnotationQuery(request)`: reads a loaded runtime annotation layer by cell, bounds, region id, or layer summary with `maxRegions` and optional expected-layer-hash guards. Missing layers, out-of-bounds queries, stale hashes, and quota truncation are represented in generated typed readouts.
- `applyVoxelAnnotationEdit(request)`: applies one typed annotation edit operation to a loaded layer after checking the expected layer hash. Stale hashes, unknown regions, invalid edits, and post-edit validation failures return typed diagnostics; accepted edits return before/after layer hashes.
- `exportVoxelAnnotationLayer(request)`: explicitly exports a loaded runtime annotation layer back to stored DTO/canonical JSON form after an expected-hash check. Runtime-to-stored promotion is receipt-driven and never implicit.
- `readProjection()`: returns a render/projection summary from public render diff contracts.
- `readEcrpRuntimeReadout()`: returns live Entity/CapabilityState/event readouts derived from the selected backend. Rust-backed readouts identify `mode: 'rust'`, `source: 'rust_bridge'`, authority surface, and declared read sets.
- `readTelemetry()`: returns sequence/tick/composition/command/replay/hash summary.
- `restart()`: unloads/reinitializes/reloads the same ProjectBundle input and resets tick/command counters and lifecycle state.

Package-root helpers over the facade expose derived readouts without asking
consumers to maintain parallel demo authority:

- `readRuntimeSessionPlayableLoopState(session, request?)`: reads lifecycle,
  telemetry, and ECRP state through public `RuntimeSessionFacade` methods and
  returns current-epoch HUD counters, health summaries, target identity, command
  gating, and missing-backend diagnostics. Replay history before the most recent
  restart/request-restart record is excluded from `shotsFired`/`actionTick`, so
  reset UI does not accidentally count historical command records.
- `readRuntimeSessionPlayableEncounterTick(session, request)`: derives the
  generated-tunnel enemy actor from ECRP state, applies pause/terminal/missing
  actor gates, advances one autonomous policy tick through RuntimeSession, and
  returns movement/combat/lifecycle summaries. Browser shells still own the
  schedule/timer and pass the current RuntimeSession camera handle/position.

Lifecycle fixture hashes in the current reference slice:

- initial reset hash: `fnv1a64:d0c05bd05488e8a5`
- enemy defeated lifecycle hash: `fnv1a64:5fbf190733451da1`
- player defeated fixture lifecycle hash: `fnv1a64:32322a108d4f2767`

The current reference helper is `createMockRuntimeSession`, a facade over the existing `RuntimeBridge` mock exposed only from `@asha/runtime-bridge/reference`. It is useful for unit tests, compatibility fixtures, and offline smoke baselines. It is not the product/live authority path for demo or Studio, and selected/native backend launchers must fail closed rather than falling back to this helper. For collision-constrained camera input, the reference facade hosts the upstream static-room collision fixture so consumers can prove wall-stop/open-space behavior without importing demo-local physics. For ECRP content, the reference RuntimeSession owns a loaded project-state projection seeded by `loadEcrpProject`; primary-fire receipts, lifecycle updates, entity events, health state, and render visibility apply to the loaded enemy entity in reference mode. It does not claim native runtime attach, product authority, raw state-store access, or renderer ownership.

Evidence lanes:

- `pnpm --filter @asha/runtime-bridge test:evidence:reference` proves the reference RuntimeSession fixture lane remains explicitly non-product authority.
- `pnpm --filter @asha/runtime-bridge test:evidence:rust` proves the public Rust-backed facade reports backend provenance for collision, combat, lifecycle, encounter, and restart.
- `pnpm --filter @asha/smoke test:evidence:reference` and `pnpm --filter @asha/smoke test:evidence:authority` split smoke evidence into reference and authority lanes.
- `pnpm --filter @asha/smoke test:voxel-annotation-proof` proves root-only consumer use of voxel annotation DTOs and RuntimeSession facade verbs when the native Rust bridge is built; without native bridge support it skips with an explicit native-unavailable reason.

## Runtime Vocabulary

The public facade and bridge/native operation names use `RuntimeSession` and `ProjectBundle` vocabulary. The remaining legacy bundle vocabulary is in the protocol crate/wire DTO lane, as documented in `docs/vocabulary-compatibility.md`.

## Non-Claims

The reference RuntimeSession reports explicit non-claims. Current examples include:

- `not_native_runtime`
- `not_raw_state_store`
- `not_arbitrary_json_bridge`
- `not_product_authority`
- `not_renderer`

These non-claims mean the reference facade is still not native runtime attach, raw state-store access, product authority, or renderer ownership. They no longer mean the FPS demo owns local combat/health/target authority; that state now comes through RuntimeSession ECRP/lifecycle/action readouts, with Rust-backed sessions carrying Rust bridge provenance where wired.
