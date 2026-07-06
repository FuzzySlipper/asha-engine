# Consumer Compatibility Surface

Status: task #2536 compatibility surface for downstream consumers. This is not a public-registry semver promise.

## Purpose

ASHA is still local-path / in-house engine substrate work, but downstream consumers need a durable place to answer:

- which generated contract surface am I using?
- which runtime bridge facade surface am I using?
- where is the changelog/migration note for a breaking border change?
- what should a consumer do when the surface is incompatible?

The answer is split between machine-readable package metadata and this human-readable changelog/process document.

## Machine-readable metadata

The engine-owned public surface manifest is:

```text
harness/public-surface/ts-packages.json
```

Every `ts/packages/*` package is listed there as `public`, `unstable`, or `internal`.
Consumer repos should validate allowlists against that manifest instead of inventing
their own package truth. The manifest records each package's ownership key, intended
consumer role, compatibility marker when one exists, and changelog anchor.
It also records consumer-role import policies, starting with the `asha-demo`
package-root allowlist and private/internal forbidden alternatives.

Tier 1 public packages carry `asha.compatibility` in `package.json` and a package-local
`compatibility.json` file. Some unstable surfaces carry package-local compatibility
metadata while their consumer role is still being ratified.

| Surface | Status | Metadata file | Compatibility version | Role |
|---|---|---|---|---|
| `@asha/contracts` | `public` | `ts/packages/contracts/compatibility.json` | `contracts.v0` | Generated semantic DTO/type border from Rust protocol crates. |
| `@asha/runtime-bridge` | `public` | `ts/packages/runtime-bridge/compatibility.json` | `runtime-bridge.v0` | Transport-neutral runtime facade, manifest-backed operation vocabulary, typed errors. |
| `@asha/catalog-core` | `unstable` | none | none | Typed gameplay preset/catalog validation surface for consumer-owned FPS tuning data; not runtime authority. |
| `@asha/command-registry` | `unstable` | `ts/packages/command-registry/src/manifest.golden.json` | `command-registry.v0` | Studio command/evidence metadata registry. |
| `@asha/devtools` | `unstable` | `ts/packages/devtools/compatibility.json` | `devtools-protocol.v0` | Observational attach/readout protocol for tools and testing harnesses. |
| `@asha/game-workspace` | `unstable` | `ts/packages/game-workspace/compatibility.json` | `game-workspace.v0` | Typed game/workspace manifest validation for consumer repos. |
| `@asha/render-projection` | `unstable` | `ts/packages/render-projection/compatibility.json` | `render-projection.v0` | Renderer-neutral retained render-diff application model. |
| `@asha/renderer-host` | `unstable` | `ts/packages/renderer-host/compatibility.json` | `renderer-host.v0` | Backend-neutral browser render surface host for demos. |
| `@asha/ui-dom` | `unstable` | none | none | Render-agnostic UI projection/control descriptors; not authority. |

Additional unstable package statuses:

- `@asha/catalog-core` is an unstable gameplay preset/catalog validation package. It may expose root-level typed tuning schemas and readouts for consumer-owned data, but it does not execute runtime authority, own generated contracts, or validate commands.
- `@asha/editor-tools` is an unstable Studio/editor helper package. It is editor-local state only, not authority.
- `@asha/renderer-host` is the unstable browser render surface host for human-facing demos. It exposes backend-neutral mount/lifecycle/projection handles and may use `@asha/renderer-three` internally while that remains the selected browser backend.
- `@asha/renderer-three` is an unstable Three.js implementation package for engine smoke/testing and the approved migration window only. It is not the long-term public renderer contract; consumers should prefer `@asha/renderer-host` for browser mounting and `@asha/render-projection` for renderer-neutral retained semantics unless a task explicitly approves the root package binding.
- `@asha/ui-dom` is an unstable render-agnostic UI projection/control descriptor package. It can expose root-level HUD/menu projection helpers, but it does not execute runtime commands or own DOM framework state.

Internal packages, including `@asha/native-bridge`, `@asha/wasm-replay-bridge`, `@asha/app`, `@asha/electron-main`, internal policy packages, `@asha/catalog-examples`, and `@asha/smoke`, are not downstream public surfaces.

The metadata schema is intentionally tiny for now:

- `schemaVersion`: metadata schema version. Current value: `1`.
- `surface`: package/surface name.
- `compatibilityVersion`: opaque ASHA compatibility marker for consumers and conformance artifacts.
- `packageVersion`: current package version; not a registry promise yet.
- `sourceOfTruth`: where agents should make source changes.
- `changelog`: section in this document for surface-specific compatibility entries.
- `migrationNoteTemplate`: section in this document that breaking changes must fill in.
- `failClosedPolicy`: what consumers should do when the version or operation is incompatible.
- `pinningGuidance`: how downstream consumers should record the surface they tested.
- `breakingChangeRequires`: minimum evidence checklist for border-breaking changes.

`harness/public-surface/check-public-boundary.py` validates that the engine manifest covers every TS package, compatibility metadata has real changelog anchors, ownership entries exist, and raw/native transports remain internal.

## Consumer Repo Roles

- `asha-testing` is the synthetic proof/conformance consumer. It owns boundary checks, compatibility evidence, generated proof artifacts, and scripted conformance workflows.
- `asha-demo` is the new human-facing demo/product-content repo. It should start from a product README and consume approved engine public or unstable surfaces through the engine manifest. Proof harnesses can be added later, but should not become the repo identity.
- `asha-studio` is the editor/product tooling repo. It may use Studio-approved unstable packages through its own boundary policy, but those allowlists should validate against the engine manifest.

No consumer should import raw native transports, generated contract internals, ASHA package `src/*` paths, Rust crate paths, or arbitrary runtime JSON tunnels. Missing public API should become an ASHA engine feature request, not a private import.

## asha-demo Initial Import Policy

Status: task #4018 policy gate for the first minimal `/home/dev/asha-demo`
skeleton. This section does not promote new public packages; it records the
current manifest decision in `harness/public-surface/ts-packages.json`.

The first `asha-demo` skeleton may depend on only these ASHA package roots:

| Package | Manifest status | Initial demo use | Rationale |
|---|---|---|---|
| `@asha/contracts` | `public` | Allowed | Generated DTO/type border from Rust protocol crates. Import from the package root only; never from `src/generated/*` or `dist/generated/*`. |
| `@asha/runtime-bridge` | `public` | Allowed, but no native/raw transport bypass | Transport-neutral runtime facade. Current World* method names are compatibility names; demo docs should use RuntimeSession/ProjectBundle vocabulary. |
| `@asha/catalog-core` | `unstable` | Allowed for gameplay preset/catalog validation only | Demo-owned tuning values may live in typed `fps_gameplay_preset.v0` data. Runtime authority, command validation, collision, combat application, policy execution, and procedural generation remain engine-owned. |
| `@asha/game-workspace` | `unstable` | Allowed for manifest/workspace validation | The current typed ASHA Game Project manifest/workspace surface. This is the preferred first skeleton dependency. |
| `@asha/render-projection` | `unstable` | Allowed for renderer-neutral projection state only | Consumers may use retained render-diff projection semantics through the root package. This is not permission to mutate authority or decode arbitrary JSON. |
| `@asha/renderer-host` | `unstable` | Preferred browser renderer mount path | Demo code mounts visible ASHA render surfaces through backend-neutral lifecycle/status handles. Three.js remains an engine-owned backend detail behind this host. |
| `@asha/renderer-three` | `unstable` | Allowed for the static-room renderer path approved in #4029 and the first-person generated-tunnel viewport path approved in #4067 | Consumers may import only from the package root and must treat it as an implementation binding over public render diffs/projection state, not as authority or a stable renderer contract. |
| `@asha/command-registry` | `unstable` | Optional, only for declared command/readout metadata | Useful for Studio-compatible typed command/evidence metadata. The skeleton should not require it unless it has a concrete manifest/readout need. |
| `@asha/ui-dom` | `unstable` | Optional, only for typed HUD/menu projection/control descriptors approved in #4043 | Useful for render-agnostic health/status/menu readouts and typed UI intents. It must not execute runtime authority commands. |

The first skeleton must not import these ASHA surfaces directly:

| Forbidden surface | Decision |
|---|---|
| `@asha/devtools` | Remains Studio/testing-only. Studio owns live/runtime readouts; `asha-demo` should not make devtools a direct product dependency. |
| `@asha/script-sdk`, `@asha/script-host`, `@asha/policy-core`, `@asha/policy-examples` | Remain internal. Demo-owned policy packs are deferred until ASHA main exposes a public policy-authoring/packaging surface. `@asha/game-workspace` already classifies policy source authoring as reserved/deferred. |
| `@asha/native-bridge`, `@asha/wasm-replay-bridge` | Remain internal. Runtime access goes through `@asha/runtime-bridge`; replay/WASM proof paths stay engine/testing-owned. |
| ASHA package `src/*` or `dist/generated/*` paths | Forbidden. Consumers use package roots only. |
| Rust crate paths or generated contract hand edits | Forbidden. Protocol changes go through Rust protocol source plus `protocol-codegen`. |

Renderer decision for this gate: task #4385 adds `@asha/renderer-host` as the
preferred browser render surface path. Demo code should mount browser render
surfaces through the host and feed it public render frames / `@asha/render-projection`
semantics. Task #4029 and #4067 previously approved a narrow package-root
`@asha/renderer-three` binding for static-room and generated-tunnel projection
work; that remains a migration allowance only and does not promote
`@asha/renderer-three` to the long-term renderer contract.

Policy decision for this gate: no demo-owned TypeScript policy package is
allowed yet. Catalog or policy directories may exist as documented placeholders
only if they do not import internal ASHA policy packages and do not claim runtime
policy execution.

No manifest change was made for #4018 because the current engine manifest already
encodes the intended roles: `asha-demo` may use the allowed package roots above,
while renderer-three, devtools, raw transports, replay bridge, and policy authoring
packages remain outside the demo boundary.

Task #4053 adds an engine-owned consumer compatibility proof in
`@asha/smoke` (`public-consumer-compat.test.ts`). The proof imports only
`@asha/runtime-bridge` and `@asha/ui-dom` package roots, exercises the approved
RuntimeSession motion/collision, generated tunnel, combat/health, nav/path,
policy-view, and HUD/menu projections, and verifies fail-closed typed receipts
instead of arbitrary JSON payloads. It also imports `@asha/runtime-bridge` through
the package `browser` condition and fails if native-only symbols leak into the
browser entry. This is the explicit public-surface safety gate for resuming
#4037, #4044, #4045, and #4046 as long as those tasks stay on approved package
roots and do not introduce private ASHA paths, raw transports, Rust crate imports,
or JSON command tunnels.

## Generated contract compatibility log

### `contracts.v0` — initial local-path boundary

Status: current initial compatibility marker for the committed generated TypeScript contracts in `@asha/contracts`.

Source of truth:

- Rust protocol crates under `engine-rs/crates/protocol/*`.
- Generated TypeScript under `ts/packages/contracts/src/generated/*`.
- Generator command: `cargo run -p protocol-codegen`.
- Check command: `bash harness/ci/check-contracts.sh`.

Consumer behavior:

- Consumers import only from `@asha/contracts` root export.
- Consumers do **not** import generated file paths directly.
- Consumers do **not** copy generated DTOs into their own repo as a forked truth source.
- `asha-testing` records the ASHA git commit plus `contracts.v0` in conformance artifacts until #2536-style metadata is copied into downstream artifacts. The new human-facing `asha-demo` should record the same metadata only for demo/product evidence it actually owns.

Breaking generated-contract changes require a migration note using the template below.

Additive notes under `contracts.v0`:

- #2563 adds a public `view` generated module for deterministic camera/view evidence: `CameraHandle`, camera pose/basis/projection/viewport DTOs, first-person camera input envelopes, and projection snapshots with column-major matrices. The compatibility marker remains `contracts.v0` because the change is additive and consumers that do not import the new types are unaffected.
- #2895 promotes existing generated public asset/render DTOs (`CatalogEntry`, `MaterialProjection`, `StaticMeshAsset`, and `RenderFrameDiff`) into first-class Studio command/runtime surfaces for model/material preview. No generated files changed in this task; the note records that these existing DTOs are now an intended public consumer shape for `@asha/command-registry` and `@asha/runtime-bridge` model/material preview evidence.

## Runtime bridge compatibility log

### `runtime-bridge.v0` — initial local-path facade boundary

Status: current initial compatibility marker for the committed `@asha/runtime-bridge` facade and runtime bridge manifest family.

Source of truth:

- Bridge manifest: `engine-rs/crates/bridge/runtime-bridge-api/bridge-manifest.toml`.
- Generated conformance descriptor: `ts/packages/runtime-bridge/src/generated/conformance.json`.
- Runtime facade root export: `ts/packages/runtime-bridge/src/index.ts`.
- Check command: `bash harness/ci/check-bridge.sh`.

Consumer behavior:

- Consumers import only from `@asha/runtime-bridge` root export.
- Consumers never import `@asha/native-bridge` or `@asha/wasm-replay-bridge` as a runtime transport.
- Required operations must either be present on the facade or fail closed with classified `RuntimeBridgeError` kinds.
- `native_unavailable` and `operation_unimplemented` are acceptable diagnostics for missing native implementation during prototype/conformance work only when the task records an explicit gap; consumers must not silently fall back to raw transports.

Breaking facade/operation changes require a migration note using the template below.

Additive notes under `runtime-bridge.v0`:

- #2564 adds three stable camera/view operations to the manifest-backed facade: `create_camera` / `createCamera`, `apply_first_person_camera_input` / `applyFirstPersonCameraInput`, and `read_camera_projection` / `readCameraProjection`. Native remains fail-closed with `operation_unimplemented` until a real native implementation lands; the mock/reference paths provide deterministic boundary evidence only. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #2895 adds one stable model/material preview/readback operation to the manifest-backed facade: `read_model_material_preview` / `readModelMaterialPreview`. The mock/reference facade derives a typed `RenderFrameDiff` from public `CatalogEntry` / `MaterialProjection` / `StaticMeshAsset` inputs. Native intentionally fail-closes with `operation_unimplemented` until a real native implementation is wired; consumers must not bypass this through renderer internals or raw transports. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4028 adds a semantic `RuntimeSession` facade exported from `@asha/runtime-bridge`: `RuntimeSessionFacade` types for initialize/load, typed command submission, deterministic tick, projection readout, telemetry/replay/hash summary, and restart. The reference helper `createMockRuntimeSession` is now explicitly imported from `@asha/runtime-bridge/reference` so production consumers do not pick up the mock backend through the root. It wraps the existing public bridge without adding raw transports or arbitrary JSON calls. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4030 adds browser FPS input collection and RuntimeSession camera input methods at the package root. `BrowserFpsInputCollector` maps structural keyboard/mouse/pointer inputs to a typed `runtime.apply_first_person_camera_input` command carrying `FirstPersonCameraInputEnvelope`, plus typed pointer-lock shell intents. `RuntimeSessionFacade` now exposes `createCamera`, `applyFirstPersonCameraInput`, and `readCameraProjection` wrappers over the existing public camera bridge operations. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4036 adds the first public typed runtime action/fire intent protocol at the `@asha/runtime-bridge` package root. Browser FPS primary-button press/release now emits `runtime.propose_runtime_action_intent` commands carrying `RuntimeActionIntentEnvelope` values (`primary_fire`, `pressed`/`released`, camera, tick, source, pressed state). `RuntimeSessionFacade.submitRuntimeActionIntent` accepts this typed proposal and returns a fail-closed `unsupported` receipt with `combat_runtime_not_wired` until #4051 wires combat/fire authority. Consumers must not replace this with raw JSON action tunnels. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4047 adds `RuntimeSessionFacade.applyCollisionConstrainedCameraInput`, a package-root wrapper around the generated `CollisionConstrainedCameraInputEnvelope` / `CameraCollisionSnapshot` bridge surface. The receipt exposes before/attempted/after motion evidence through the snapshot plus collided, blocked axes, world hash, collision projection hash, movement hash, and a replay record kind. The reference mock hosts the upstream static-room collision fixture so forward movement into the wall blocks while lateral movement in open space succeeds; consumers must still use this facade instead of demo-local physics, generated internals, native transports, or Rust crates. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4050 adds the public generated tunnel readout facade: `RuntimeSessionFacade.readGeneratedTunnelReadout` plus `TINY_GENERATED_TUNNEL_READOUT` and generated tunnel readout types from the `@asha/runtime-bridge` root export. The readout exposes #4038 `tiny-enclosed` fixture evidence: seed `17`, config hash `e1d156c6b55137a7`, output hash `a9b504096397f5b4`, replay hash `fnv1a64:0821a0c2aea17dff`, render projection hash `fnv1a64:21eb8696f6f3b5c4`, collision projection hash `fnv1a64:78b242163cf67524`, spawn markers, material roles, and volume/corridor summaries. `RuntimeSessionFacade.requestGeneratedTunnelOperation` provides typed fail-closed receipts for `regenerate` and `apply_to_runtime_world`; consumers must not replace these with local generation or JSON action tunnels. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4051 wires the public fire/combat/health reference readout: `RuntimeSessionFacade.submitRuntimeActionIntent` accepts `primary_fire` pressed intents and returns a `CombatRuntimeReadout` for the #4040 generated-tunnel hit/death fixture; `RuntimeSessionFacade.readCombatReadout` also exposes the geometry-blocked miss readout. Public root exports include `GENERATED_TUNNEL_FIRE_HIT_READOUT`, `GENERATED_TUNNEL_FIRE_MISS_READOUT`, and combat readout types. Hit/death evidence uses health hash `3c89045230f2d9d9` and replay hash `6b133026c511b0f5`; miss evidence uses health hash `56b1331c0f202ff1` and replay hash `3b1e1a9897571bc4`. HUD/menu rendering remains #4043, and consumers must not introduce local combat authority or JSON action tunnels. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4052 adds public nav/pathfinding readouts: `RuntimeSessionFacade.readNavProjection`, `queryNavPath`, and `readNavPolicyView`, plus nav readout constants/types from the `@asha/runtime-bridge` root export. The #4041 generated-tunnel projection exposes walkable cells `66` and projection hash `d1f6ac3e051d6b6e`; the reachable path readout has visited `21`, path length `9`, and path hash `e8e1ea7a09811ced`; the no-path readout is typed as `blocked` with empty path hash `a8c7f832281a39c5`. `readNavPolicyView` is explicitly read-only/proposal-only and exposes no mutate/apply-path method. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4042 adds the first public constrained enemy policy fixture through the `@asha/runtime-bridge` root export: `createGeneratedTunnelEnemyPolicyFixture`, `createEnemyPolicyView`, `proposeEnemyPolicyFrame`, and `validateEnemyPolicySource`. The fixture consumes the read-only/proposal-only nav policy view, proposes a typed movement intent toward the generated tunnel target, and emits a `RuntimeActionIntentEnvelope` with source `enemy_policy` for primary fire; `RuntimeSessionFacade.submitRuntimeActionIntent` remains the authority path for fire/combat validation. The source validator rejects policy text that references clock, ambient randomness, network, DOM, filesystem, process, dynamic-code, or dynamic-import capabilities. Movement remains proposal-only in this slice until a runtime movement authority surface lands; consumers must not substitute demo-local state mutation, private policy packages, or JSON command tunnels. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4065 adds `RuntimeSessionFacade.runAutonomousPolicyTick`, a narrow autonomous enemy-policy loop readout for the generated tunnel fixture. Each tick advances the reference session, builds the read-only nav/policy view, validates typed policy proposals, rejects forbidden policy source capabilities, routes primary-fire proposals through `submitRuntimeActionIntent`, and reports proposal counts, movement/combat summaries, nav path hash, replay record hashes, and a deterministic tick hash. Movement proposals remain `unsupported` with `movement_authority_not_wired`; this is not a generic event bus, behavior tree, demo-local authority, or JSON command tunnel. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4066 adds public lifecycle/restart readouts on `RuntimeSessionFacade`: `readLifecycleStatus` and `requestSessionRestart`. The lifecycle readout reports player/enemy health/dead state, win/loss/in-progress outcome, restart eligibility, terminal lifecycle events, reset hash `fnv1a64:d0c05bd05488e8a5`, enemy-defeated lifecycle hash `fnv1a64:5fbf190733451da1`, and player-defeated fixture hash `fnv1a64:32322a108d4f2767`. `requestSessionRestart` validates typed `runtime.restart_session_intent` proposals from HUD/programmatic sources, rejects stale session hashes or non-terminal-gated requests with typed receipts, and resets the reference session deterministically through the RuntimeSession restart path. This does not add save/load persistence, UI authority, demo-local lifecycle mutation, or arbitrary JSON commands. The compatibility marker remains `runtime-bridge.v0` because the change is additive.

## Command registry compatibility log

### `command-registry.v0` — unstable Studio command metadata

Status: unstable root-barrel package surface for Studio command/evidence metadata.

Source of truth:

- Root export: `ts/packages/command-registry/src/index.ts`.
- Registry implementation and golden: `ts/packages/command-registry/src/manifest.ts` and `src/manifest.golden.json`.

Consumer behavior:

- Consumers import only from `@asha/command-registry` root export.
- Consumers do not treat command-registry examples as authority, runtime, or renderer truth.
- The registry describes typed command metadata and evidence posture; execution and authority validation stay in the runtime/Rust surfaces.

## Devtools protocol compatibility log

### `devtools-protocol.v0` — unstable attach/readout protocol

Status: unstable observational protocol for Studio and synthetic testing consumers.

Consumer behavior:

- Consumers import only from `@asha/devtools` root export.
- Devtools is observational: it formats projected diagnostics, attach protocol state, and readouts; it does not mutate authority.
- Consumers must fail closed on unsupported protocol versions or missing evidence instead of replacing the typed protocol with generic JSON method tunnels.

## Game workspace compatibility log

### `game-workspace.v0` — unstable consumer workspace manifest

Status: unstable typed manifest/workspace validation package for consumer repos.

Consumer behavior:

- Consumers import only from `@asha/game-workspace` root export.
- `asha-testing` uses it for synthetic conformance/proof workflows.
- The new `asha-demo` may use it for human-facing project workspace setup, but should keep product identity separate from proof harness machinery.
- Manifest validation rejects private transport hints, ASHA internals, generated paths, and unsupported backend/profile claims.

## Catalog Core unstable status

`@asha/catalog-core` is explicit but unstable. It exposes typed gameplay
preset/catalog validation for consumer-owned tuning data. It does not execute
runtime authority, own generated contracts, mutate ProjectBundle state, run
policy, apply combat damage, resolve collision, or generate worlds.

Additive notes under this unstable status:

- #4101 adds `fps_gameplay_preset.v0` and
  `fps_gameplay_preset_catalog.v0` for the default generated-tunnel FPS loop.
  Game repos may own `displayName`, player controller tuning, weapon/fire
  tuning, enemy behavior references, encounter references, and generator preset
  references through the typed catalog surface. Engine-owned concerns remain
  schema validation, runtime authority, collision resolution, combat damage
  application, policy execution, and procedural generation. The default preset
  fixture is
  `harness/fixtures/gameplay-presets/generated-tunnel-default-fps.snapshot.txt`;
  its preset hash is `fnv1a64:c5a07d62670d6616`, tuning hash is
  `fnv1a64:a9d279e7f8749a0f`, reference hash is
  `fnv1a64:16fe3b71072981e3`, and catalog hash is
  `fnv1a64:51431466746a3fc4`. Demo constants should migrate by replacing local
  movement/look/fire/enemy/encounter/generator constants with reads from
  `readDefaultFpsGameplayPreset()` or `readFpsGameplayPresetCatalog()` while
  continuing to submit runtime commands through `@asha/runtime-bridge`. The
  preset and catalog readouts now include `authorityBoundary`, which explicitly
  labels `@asha/catalog-core` validation as DTO shape / consumer tuning range
  validation only. Runtime acceptance remains owned by Rust RuntimeSession
  authority surfaces such as `loadEcrpProject`, collision input, primary-fire
  action intents, policy ticks, encounter transitions, and restart.

## Render projection compatibility log

### `render-projection.v0` — unstable renderer-neutral retained projection

Status: unstable root-barrel package surface for renderer-neutral render-diff application.

Consumer behavior:

- Consumers import only from `@asha/render-projection` root export.
- Consumers feed it decoded `RenderFrameDiff` / `RenderDiff` values from public contracts or runtime facade helpers; it does not decode arbitrary JSON or call raw transports.
- Consumers bind returned neutral application instructions or retained snapshots into their renderer of choice. Three.js is one possible binding, not the public ASHA contract.
- The projection fails closed on duplicate/stale handles, unsupported diff operations, malformed mesh payloads, and in-use static mesh redefinitions.

## Renderer host compatibility log

### `renderer-host.v0` - unstable backend-neutral browser render host

Status: unstable root-barrel package surface for browser demo renderer mounting.

Consumer behavior:

- Consumers import only from `@asha/renderer-host` root export.
- Consumers mount a browser surface with `mountAshaRendererSurface(canvas, options)` and receive backend-neutral lifecycle, pointer-lock, movement-status, projection-snapshot, and interaction handles.
- Consumers feed the host public `RenderFrameDiff` values or helper-built frames. They do not import `@asha/renderer-three`, `three`, `THREE`, `WebGLRenderer`, or `ThreeRenderer`.
- Backend identity is diagnostic metadata only. The current implementation uses the engine-owned Three.js backend internally, but downstream call sites should not change if ASHA later swaps to Babylon.js or a native Rust renderer host.
- The host does not own gameplay, collision, combat, runtime authority, or command validation. Runtime intents still go through `@asha/runtime-bridge` and Rust authority surfaces.

## Renderer Three unstable status

`@asha/renderer-three` is explicit but unstable. It is an engine-owned Three.js implementation package for smoke/testing and should not be treated as the cross-repo renderer contract. Studio and demos should prefer `@asha/render-projection` for renderer-neutral ASHA semantics and keep Three.js code as a local binding.

Additive notes under this unstable status:

- #4029 widens the engine manifest so `asha-demo` may import the package root for the static-room render path only. The public helper `createStaticRoomRenderFrame` emits a synthetic `RenderFrameDiff`; `renderProjectedFrame` applies that frame through `@asha/render-projection` and the retained `ThreeRenderer`. Evidence lives in `harness/fixtures/render-diffs/static-room.json` and `harness/goldens/render-diffs/static-room.snapshot`. This is structural render evidence only: no gameplay loop, runtime attachment, authority mutation, collision simulation, or browser screenshot is claimed.
- #4067 adds the first-person generated-tunnel viewport adapter at the `@asha/renderer-three` package root: `createGeneratedTunnelViewportFrame`, `renderFirstPersonTunnelViewport`, and `summarizeFirstPersonTunnelViewport`. The adapter consumes `GeneratedTunnelReadout` plus `CameraProjectionSnapshot`, creates a deterministic tunnel shell/spawn-marker `RenderFrameDiff`, applies it through `RenderProjection` and `ThreeRenderer`, and reports `first_person_tunnel_viewport.v0` summary evidence. Current fixture hashes are viewport frame `fnv1a64:db081afd570c2f30` and structural snapshot `fnv1a64:35ad3bca1a9f1667`; generated tunnel projection hashes remain render `fnv1a64:21eb8696f6f3b5c4` and collision `fnv1a64:78b242163cf67524`. This is still projection-only: no runtime authority, collision authority, local generation, animation system, or pixel golden is claimed.

## Editor Tools unstable status

`@asha/editor-tools` is explicit but unstable. It holds editor-local state helpers and previews only; it does not validate or mutate authority. Studio may consume it through root exports while the engine manifest records it as an unstable editor/tooling surface.

## UI DOM unstable status

`@asha/ui-dom` is explicit but unstable. It holds render-agnostic UI projection/control descriptors for engine-owned UI surfaces; it does not own authority, runtime command execution, native transport, policy behavior, or a DOM framework requirement.

Additive notes under this unstable status:

- #4043 adds `buildHudProjection` and `hudControlToIntent` for typed HUD/menu projection. The projection exposes health, status, non-claim text, and resume/restart/options/exit controls as pure data. `hudControlToIntent` emits typed proposals such as `runtime.restart_session_intent`; runtime/session code must still validate and execute restart behavior. No arbitrary JSON payloads or UI authority are introduced.

## Consumer pinning guidance

Until ASHA has registry/package publication, downstream consumers pin by local path plus ASHA git commit:

```json
{
  "dependencies": {
    "@asha/contracts": "file:../asha/ts/packages/contracts",
    "@asha/runtime-bridge": "file:../asha/ts/packages/runtime-bridge"
  }
}
```

Conformance artifacts should record:

- ASHA git commit or source path being tested;
- `@asha/contracts` compatibility version from `compatibility.json`;
- `@asha/runtime-bridge` compatibility version from `compatibility.json`;
- any unstable package compatibility markers listed in `harness/public-surface/ts-packages.json`;
- any explicit compatibility gaps or migration tasks.

If a consumer sees an unknown `compatibilityVersion`, missing metadata file, missing required operation, or breaking-change log entry without a migration note, it should fail closed and file an ASHA engine compatibility/request task instead of papering over the gap.

## Migration note template

Use this template for every breaking generated-contract or runtime-bridge facade change.

```markdown
### <compatibility-version> — <short title>

Surface: `@asha/contracts` or `@asha/runtime-bridge`
Change type: breaking | additive | deprecation | fail-closed behavior change
Introduced by: <task id / commit / PR>
Source files: <Rust protocol crate, bridge manifest, facade file, generator>

What changed:
- ...

Why this is engine-level:
- ...

Downstream impact:
- `asha-testing`: ...
- `asha-demo`: ...
- future consumers: ...

Required migration:
1. ...
2. ...

Required evidence:
- generated contract sync / bridge check command:
- fixture/golden update:
- downstream typecheck/conformance command:

Fail-closed expectation:
- what consumers should reject rather than silently adapting:
```

## Generated-file hand-edit policy

Generated contract files are committed for worker convenience but never hand-edited.

Current generated files include `ts/packages/contracts/src/generated/*.ts` and carry a generated header. The source of truth is Rust protocol code plus `protocol-codegen`.

If a generated contract output is wrong:

1. change the Rust protocol source or generator;
2. run `cargo run -p protocol-codegen`;
3. commit source and generated output together;
4. update this compatibility document and migration notes if the consumer-visible border changed;
5. run `bash harness/ci/check-contracts.sh`.

Manual edits to generated output are drift, not a shortcut.
