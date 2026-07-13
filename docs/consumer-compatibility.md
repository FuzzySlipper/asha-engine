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
harness/public-surface/rust-crates.json
```

Every `ts/packages/*` package is listed there as `public`, `unstable`, or `internal`.
Consumer repos should validate allowlists against that manifest instead of inventing
their own package truth. The manifest records each package's ownership key, intended
consumer role, compatibility marker when one exists, and changelog anchor.
It also records consumer-role import policies, starting with the `asha-demo`
package-root allowlist and private/internal forbidden alternatives.

The Rust manifest records approved public facade crates for downstream compiled
game modules. For `asha-demo`, the current approved Rust dependency is:

```toml
asha-game-rule-extension = { path = "../asha-engine/public-rust/game-rule-extension" }
```

Downstream game repos must depend on that facade path, not on
`../asha-engine/engine-rs/crates/*`. The facade re-exports the public
game-rule extension trait and generated extension DTOs while the implementation
source of truth remains inside the engine workspace.

Role-scoped observed consumption is recorded separately in
`harness/consumer-needs/manifests/` under the schema documented in
`docs/consumer-needs-manifests.md`. Public-surface manifests answer what a role
may import; consumer-needs manifests answer which operations, types, providers,
selectors, fields, quotas, bindings, and proof level a real consumer requires.
`./harness/ci/check-consumer-needs.sh` validates both together without treating
type existence as provider or delivery evidence.

Tier 1 public packages carry `asha.compatibility` in `package.json` and a package-local
`compatibility.json` file. Some unstable surfaces carry package-local compatibility
metadata while their consumer role is still being ratified.

| Surface | Status | Metadata file | Compatibility version | Role |
|---|---|---|---|---|
| `@asha/contracts` | `public` | `ts/packages/contracts/compatibility.json` | `contracts.v0` | Generated semantic DTO/type border from Rust protocol crates. |
| `@asha/runtime-bridge` | `public` | `ts/packages/runtime-bridge/compatibility.json` | `runtime-bridge.v0` | Transport-neutral runtime facade, manifest-backed operation vocabulary, typed errors. |
| `@asha/runtime-session` | `unstable` | `ts/packages/runtime-session/compatibility.json` | `runtime-session.v0` | Transport-neutral RuntimeSession semantic readouts, proposal envelopes, and domain helper projections. |
| `@asha/browser-host` | `unstable` | `ts/packages/browser-host/compatibility.json` | `browser-host.v0` | Browser/dev static UI host that installs the native Rust RuntimeBridge provider before app boot. |
| `@asha/catalog-core` | `unstable` | none | none | Typed gameplay preset/catalog validation surface for consumer-owned FPS tuning data; not runtime authority. |
| `@asha/command-registry` | `unstable` | `ts/packages/command-registry/src/manifest.golden.json` | `command-registry.v0` | Studio command/evidence metadata registry. |
| `@asha/devtools` | `unstable` | `ts/packages/devtools/compatibility.json` | `devtools-protocol.v0` | Observational attach/readout protocol for tools and testing harnesses. |
| `@asha/game-workspace` | `unstable` | `ts/packages/game-workspace/compatibility.json` | `game-workspace.v0` | Typed game/workspace manifest validation for consumer repos. |
| `@asha/render-projection` | `unstable` | `ts/packages/render-projection/compatibility.json` | `render-projection.v0` | Renderer-neutral retained render-diff application model. |
| `@asha/renderer-host` | `unstable` | `ts/packages/renderer-host/compatibility.json` | `renderer-host.v0` | Backend-neutral browser render surface host for demos. |
| `@asha/ui-dom` | `unstable` | none | none | Render-agnostic UI projection/control descriptors; not authority. |

Additional unstable package statuses:

- `@asha/catalog-core` is an unstable gameplay preset/catalog validation package. It may expose root-level typed tuning schemas and readouts for consumer-owned data, but it does not execute runtime authority, own generated contracts, or validate commands.
- `@asha/browser-host` is the unstable host surface for ASHA Game Projects that need human-playable browser/dev runs with native Rust RuntimeBridge authority. It serves a built UI root, installs `globalThis.ashaRuntimeBridge` with provider kind `asha.runtime_bridge.native_rust_provider.v1`, and fails closed for missing/spoofed providers instead of falling back to reference authority.
- `@asha/editor-tools` is an unstable Studio/editor helper package. It is editor-local state only, not authority.
- `@asha/runtime-session` is the unstable transport-neutral RuntimeSession contract package introduced by #4547 and completed as the facade owner by #5506. It owns `RuntimeSessionFacade`, capability contracts, runtime action intents, generated tunnel and combat/nav/encounter readouts, combat feedback projection, enemy policy proposal shapes, and ECRP render target identity. `@asha/runtime-bridge` constructs concrete adapters and owns transport access; it does not re-export the semantic session surface.
- `@asha/renderer-host` is the unstable browser render surface host for human-facing demos. It exposes backend-neutral mount/lifecycle/projection handles and may use `@asha/renderer-three` internally while that remains the selected browser backend.
- `@asha/renderer-three` is an unstable Three.js implementation package for engine smoke/testing only. It is not the long-term public renderer contract; human-facing demos should use `@asha/renderer-host` for browser mounting and `@asha/render-projection` for renderer-neutral retained semantics.
- `@asha/ui-dom` is an unstable render-agnostic UI projection/control descriptor package. It can expose root-level HUD/menu projection helpers, but it does not execute runtime commands or own DOM framework state.
- Browser input ownership lives in `@asha/runtime-bridge` through `BrowserInputHost`, which attaches DOM listeners and submits normalized samples through the public RuntimeSession input surface. FPS and editor code consume named actions through `BrowserFpsResolvedActionConsumer` and `EditorResolvedInputConsumer`; they do not own key codes. Consumers must not replace this with demo-local WASD/mouse-look globals, renderer-three imports, bare Three.js controls, raw runtime transports, or generated internals.
- RuntimeSession ECRP loads may declare generated `GameRuleModuleManifest[]`
  values through `loadEcrpProject(input.gameRuleModules)`. Rust-backed
  sessions validate the declaration shape, forward compatible manifests to the
  FPS RuntimeSession authority load, and fail closed before bridge mutation when
  declarations are malformed. Consumers must not install game-rule modules
  through demo-local registries, private native transports, or raw JSON tunnels.

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

## asha-studio Voxel Conversion Boundary

Status: task #4287 consumer compatibility record for the first mesh-to-voxel conversion lane.

Asha Studio may build voxel conversion UI and workflow affordances from these ASHA package roots:

| Package root | Studio use | Boundary |
|---|---|---|
| `@asha/contracts` | Generated voxel conversion DTOs: plan/preview/apply/model-info/model-window requests and readouts, plans, previews, receipts, diagnostics, and evidence refs. | Import from the package root only; never from generated file paths or copied DTO forks. |
| `@asha/contracts` | Generated game-rules DTOs: effect bundles, modifier definitions, stack/duration/tick policies, diagnostics, resolution receipts, traces, and evidence refs. | Contract/config surface only. Downstream TS may author catalog data and read receipts, but must not resolve, apply, or commit authority effects locally. |
| `@asha/runtime-bridge` | `RuntimeSessionFacade` voxel conversion methods: `planVoxelConversion`, `previewVoxelConversion`, `applyVoxelConversion`, `exportVoxelConversionEvidence`, `readVoxelModelInfo`, and `readVoxelModelWindow`. | RuntimeSession remains the authority route. Missing native/reference support must surface as classified fail-closed errors. |
| `@asha/runtime-bridge` | `RuntimeSessionFacade` game-rules methods: `validateGameRuleCatalog`, `submitGameRuleEffectIntent`, and `readGameRuleRuntimeReadout`. | Consumers submit generated DTOs and display receipts/readouts only. Rust-backed sessions route through `svc-game-rules`; reference mode is labelled fixture compatibility and must not be promoted to product authority. |
| `@asha/command-registry` | Studio command/menu/timeline metadata for `voxel_conversion.plan`, `voxel_conversion.preview`, `voxel_conversion.apply`, and `voxel_conversion.export_evidence`. | Metadata describes commands, contracts, artifacts, retry/idempotency, and UI placement; it does not execute conversion or validate authority. |
| `@asha/render-projection` | Optional renderer-neutral projection/evidence readback for previews once the runtime emits public render frames. | Projection is descriptive. It must not become mesh voxelization authority or a renderer-private data source. |
| `@asha/ui-dom` | Optional render-agnostic panel/control descriptors if Studio chooses to share UI readout vocabulary. | UI descriptors may propose or display; they do not mutate runtime authority. |
| `@asha/devtools` / `@asha/editor-tools` / `@asha/game-workspace` / `@asha/catalog-core` | Existing Studio-approved unstable tooling surfaces when the surrounding workflow needs attach/readout, editor-local state, workspace manifests, or catalog validation. | These packages do not own mesh-to-voxel conversion authority. |

The engine manifest already records the Studio policy in
`harness/public-surface/ts-packages.json`: Studio may consume `@asha/contracts`,
`@asha/runtime-bridge`, `@asha/command-registry`, `@asha/devtools`,
`@asha/editor-tools`, `@asha/game-workspace`, `@asha/render-projection`,
`@asha/catalog-core`, and `@asha/ui-dom` through package roots, plus the explicit
`@asha/runtime-bridge/reference` fixture subpath. Studio must not import
`@asha/native-bridge`, `@asha/renderer-three`, `@asha/wasm-replay-bridge`,
policy/script internals, ASHA package `src/*` paths, ASHA package
`dist/generated/*` paths, Rust crate paths, raw bridge operations, renderer
buffers as authority, or VoxelForge runtime code.

Fail-closed behavior is part of the compatibility contract:

- unavailable native/reference backend support reports `RuntimeBridgeError` with
  `operation_unimplemented` on the runtime facade methods from #4284;
- unsupported source assets, invalid material maps, oversized output, stale
  source hashes, stale authority snapshots, and replay mismatches are typed
  voxel conversion diagnostics, not best-effort partial output;
- material maps may include generated texture sample assets and UV sample
  bindings for the Rust-owned nearest-texel `palette_index_u16` sampling slice;
  missing texture snapshots, texture hash mismatches, missing UV refs,
  unsupported texture formats/policies, and invalid material rules fail closed
  through generated voxel conversion diagnostics;
- Studio should display and preserve those diagnostics/evidence refs rather than
  falling back to local conversion, private generated paths, raw native calls, or
  arbitrary JSON command tunnels.

Predecessor evidence for the conversion lane is engine-owned. The durable
foundation is the ASHA voxel capability series (`asha/voxel-capability-roadmap-index`
and especially `voxel-capability-06-voxel-meshing`,
`voxel-capability-07-mesh-payload-render-protocol`,
`voxel-capability-08-threejs-voxel-renderer-path`,
`voxel-capability-10-picking-selection`, and
`voxel-capability-11-collision-physics`), plus the committed #4282-#4286 task
slices. VoxelForge-derived assets or candidates may be used only as predecessor
evidence after asset/license review; they are not runtime dependencies, source
truth, or a Studio-owned conversion path.

The #4286 consumer proof covers the practical adoption boundary:
`harness/fixtures/voxel-conversion/studio-consumer-proof.json` and
`ts/packages/smoke/src/voxel-conversion-consumer-proof.test.ts` import only
approved public roots, verify command metadata and generated DTO shapes, check
the Rust authority golden
`harness/goldens/voxel-conversion/conversion-summary.golden`, and assert the
RuntimeSession facade fails closed until backend wiring lands.

## asha-demo Initial Import Policy

Status: task #4018 policy gate for the first minimal `/home/dev/asha-demo`
skeleton. This section does not promote new public packages; it records the
current manifest decision in `harness/public-surface/ts-packages.json`.

The first `asha-demo` skeleton may depend on only these ASHA package roots:

| Package | Manifest status | Initial demo use | Rationale |
|---|---|---|---|
| `@asha/contracts` | `public` | Allowed | Generated DTO/type border from Rust protocol crates. Import from the package root only; never from `src/generated/*` or `dist/generated/*`. |
| `@asha/runtime-bridge` | `public` | Allowed, but no native/raw transport bypass | Transport-neutral runtime facade. Current World* method names are compatibility names; demo docs should use RuntimeSession/ProjectBundle vocabulary. |
| `@asha/runtime-bridge` | `public` | Allowed for bounded game-rules RuntimeSession methods | Demo may validate generated catalogs, submit typed effect intents, and read active modifier/trace/replay projections through `RuntimeSessionFacade`; it must not implement local TS rule authority or arbitrary JSON rule dispatch. |
| `@asha/catalog-core` | `unstable` | Allowed for gameplay preset/catalog validation only | Demo-owned tuning values may live in typed `fps_gameplay_preset.v0` data. Runtime authority, command validation, collision, combat application, policy execution, and procedural generation remain engine-owned. |
| `@asha/game-workspace` | `unstable` | Allowed for manifest/workspace validation | The current typed ASHA Game Project manifest/workspace surface. This is the preferred first skeleton dependency. |
| `@asha/render-projection` | `unstable` | Allowed for renderer-neutral projection state only | Consumers may use retained render-diff projection semantics through the root package. This is not permission to mutate authority or decode arbitrary JSON. |
| `@asha/renderer-host` | `unstable` | Preferred browser renderer mount path | Demo code mounts visible ASHA render surfaces through backend-neutral lifecycle/status handles. Three.js remains an engine-owned backend detail behind this host. |
| `@asha/command-registry` | `unstable` | Optional, only for declared command/readout metadata | Useful for Studio-compatible typed command/evidence metadata. The skeleton should not require it unless it has a concrete manifest/readout need. |
| `@asha/ui-dom` | `unstable` | Optional, only for typed HUD/menu projection/control descriptors approved in #4043 | Useful for render-agnostic health/status/menu readouts and typed UI intents. It must not execute runtime authority commands. |

The first skeleton must not import these ASHA surfaces directly:

| Forbidden surface | Decision |
|---|---|
| `@asha/devtools` | Remains Studio/testing-only. Studio owns live/runtime readouts; `asha-demo` should not make devtools a direct product dependency. |
| `@asha/renderer-three` | Backend implementation package. `asha-demo` mounts render surfaces through `@asha/renderer-host`; any Three.js backend wiring stays behind that host. |
| `@asha/script-sdk`, `@asha/script-host`, `@asha/policy-core`, `@asha/policy-examples` | Remain internal. Demo-owned policy packs are deferred until ASHA main exposes a public policy-authoring/packaging surface. `@asha/game-workspace` already classifies policy source authoring as reserved/deferred. |
| `@asha/native-bridge`, `@asha/wasm-replay-bridge` | Remain internal. Runtime access goes through `@asha/runtime-bridge`; replay/WASM proof paths stay engine/testing-owned. |
| ASHA package `src/*` or `dist/generated/*` paths | Forbidden. Consumers use package roots only. |
| Rust crate paths or generated contract hand edits | Forbidden. Protocol changes go through Rust protocol source plus `protocol-codegen`. |

Renderer decision for this gate: task #4385 adds `@asha/renderer-host` as the
preferred browser render surface path. Demo code should mount browser render
surfaces through the host and feed it public render frames / `@asha/render-projection`
semantics. Task #4387 removes the old `asha-demo` renderer-three allowance from
the engine manifest. Static-room and generated-tunnel projection work that needs
the concrete backend now belongs to engine-owned smoke/testing or the
`@asha/renderer-host` implementation path, not demo app code.

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

### `render-material-descriptor.v2` — typed feedback parameters

Surface: `@asha/contracts` render and asset DTOs. Change type: additive wire
format with a required source-level migration for typed object literals.
Introduced by #5602.

- `RenderMaterialDescriptor` now carries `schemaVersion: 2`, `textureTint`,
  `emissionColor`, and `emissionIntensity`.
- `RenderDiff` adds `setMaterialInstanceParameters`, targeting one retained
  static-mesh handle and declared material slot; `parameters: null` resets the
  slot to descriptor defaults.
- Asset `RenderMaterial` adds `textureTint` and `emissionColor`; its existing
  `emissive` field remains the catalog-side intensity for stored-data
  compatibility.
- Runtime decoding accepts unversioned/schema-v1 render descriptors and
  normalizes neutral defaults. Rust catalog decoding accepts stored material
  styles that omit the two new catalog fields. Unsupported descriptor versions
  and invalid values fail closed.
- Typed consumers constructing render/catalog object literals must add the new
  fields and regenerate/rebuild together. Consumers continue feeding public
  `RenderFrameDiff` values to `@asha/render-projection` or
  `@asha/renderer-host`; they do not import renderer internals.

Evidence: `harness/ci/check-contracts.sh`,
`harness/ci/check-render-goldens.sh`, and the generated
`material-feedback.json` / `material-feedback.snapshot` pair. See
`docs/material-feedback.md` for lifecycle and non-claims.

## Rust game-rule extension compatibility log

### `asha-game-rule-extension` — public local-path facade

Status: task #4743 public Rust dependency lane for downstream game-owned rule
modules.

Source of truth:

- Public facade: `public-rust/game-rule-extension`.
- Engine implementation/API source: `engine-rs/crates/rules/game-rule-extension`.
- Generated extension DTO source: `engine-rs/crates/protocol/protocol-game-extension`.
- Metadata: `harness/public-surface/rust-crates.json`.
- Check command: `python3 harness/public-surface/check-public-boundary.py`.

Consumer behavior:

- Downstream game crates depend on `asha-game-rule-extension` through the public
  facade path, for example:

  ```toml
  asha-game-rule-extension = { path = "../asha-engine/public-rust/game-rule-extension" }
  ```

- Consumers import the Rust crate as `asha_game_rule_extension`.
- Consumers may use the successor `GameplayModuleManifest`, namespaced
  `GameplayContractRef`, invocation descriptors, and registry readout contracts
  through the same facade while the legacy hook path migrates. These contracts
  do not expose registry mutation or TypeScript callbacks.
- Existing consumers maintaining the historical compatibility hook use re-exported typed DTOs such as
  `GameRuleModuleManifest`, `GameRuleModuleRef`, `WeaponEffectHookRequest`,
  `GameExtensionProposal`, `GameExtensionHookReceipt`, and replay evidence
  types.
- Consumers do not depend on `engine-rs/crates/*`, vendor generated DTOs,
  hand-edit generated contracts, call RuntimeSession internals, or mutate ASHA
  authority directly.

This facade is a compile-time rule-module API only. RuntimeSession still
validates declared manifests, invokes modules through an approved host path, and
applies accepted effects through Rust authority.

New gameplay authority should not add another hook to this surface. Use
`asha-gameplay-module-sdk` plus `asha-gameplay-runtime-host`, following
`docs/gameplay-fabric-growth-recipes.md`. The compatibility hook remains only
until its existing callers migrate.

## Rust gameplay module SDK compatibility log

- 2026-07-13: Added serde-backed codec/configuration/state helpers and the
  explicit `GameplayDerivedModuleTopology` authoring path. This is additive;
  it derives existing closed manifest/provider/runtime declarations and does
  not add ambient registration or new mutation rights. The runtime host
  continues to re-export `GameplayRuntimeDeclaredReadPlan` at its prior path.
- 2026-07-11: Added generated gameplay-module configuration/binding contracts
  and the deterministic public `GameplayModuleBindingRegistryBuilder`. This is
  additive for module authors; executable behavior remains statically linked and
  Session activation remains Rust-owned.

### `asha-gameplay-module-sdk` — public static gameplay-module facade

Status: task #5634 public Rust authoring and static-composition lane.

- Public facade: `public-rust/gameplay-module-sdk`.
- Engine source: `engine-rs/crates/rules/gameplay-module-sdk`.
- Downstream dependency:

  ```toml
  asha-gameplay-module-sdk = { path = "../asha-engine/public-rust/gameplay-module-sdk" }
  ```

The facade exposes namespaced gameplay contracts, typed declared reads,
configuration/state schema metadata, typed codec and state-adapter edges,
boring event/proposal/local-fact helpers, and static provider composition. It
does not expose raw Session/entity/module stores, mutable runtime registries,
query-provider internals, or authority-owner implementations. Generated
TypeScript remains a configuration/projection contract and cannot execute Rust
authority behavior.

The initial compatibility marker is the crate version plus exact SDK, contract,
source, schema, and artifact hashes recorded in each module manifest. Wave 1 is
static composition only; runtime plugin loading is not part of this surface.

## Rust gameplay runtime host compatibility log

- 2026-07-12: `GameplayRuntimeHost` now owns the replayable gameplay action
  scheduler. `GameplayRuntimeProjectInput` adds the closed scheduler definition;
  public Rust exposes typed schedule/trigger/route commands and bounded
  readout; host snapshots retain the complete scheduler state. The TypeScript
  load/advance/readout contract adds the matching scheduler shapes. The host
  hash now includes current EntityStore/prefab authority and scheduler state,
  so movement outside trigger transitions is still visible authority drift.

- 2026-07-11: Added the public Rust `GameplayRuntimeHost::decide` path and
  `GameplayRuntimeDecisionOwner` port. Decision invocations now receive frozen
  declared reads; host snapshots preserve pending continuation generations and
  a bounded decision receipt ledger. Missing, wrong, replayed, and stale resume
  tokens fail before module invocation. This is additive and remains a
  statically linked Rust authority boundary; no TypeScript callback or dynamic
  owner registry is introduced.

- 2026-07-11: Added `asha-gameplay-runtime-host` as the public static product
  host for generated bindings/triggers, frozen declared reads, engine-owner
  routing, collision-constrained actor movement, reaction frames, and
  save/restore. Added the transport-neutral `GameplayRuntimeHostTransport` and
  five `RuntimeSessionFacade` operations. The reference RuntimeSession fails
  closed; downstream products supply a consumer-owned native provider that
  statically links their real module crates.

### `asha-gameplay-runtime-host` — static downstream RuntimeSession host

Status: tasks #5674 and #5677 public Rust and transport-neutral host lane.

- Public Rust facade: `public-rust/gameplay-runtime-host`.
- Engine source: `engine-rs/crates/rules/gameplay-runtime-host`.
- Browser contract: `@asha/runtime-session` package root.
- Concrete facade construction: `@asha/runtime-bridge` package root.

The compatibility boundary is additive and static. Consumers may use the
generated `GameplayTriggerDefinition`, `GameplayModuleBindingRegistry`, host
readout/frame hashes, validated prefab bootstrap/readouts, and the
`GameplayRuntimeHostTransport` port. They may not
import `engine-rs/crates/*`, install JavaScript authority callbacks, substitute
the reference RuntimeSession, or route arbitrary JSON mutations.

## Rust gameplay module conformance compatibility log

- 2026-07-11: Added `asha-gameplay-module-conformance` as an additive public
  build/dev tool. It consumes the existing SDK and ProjectBundle binding
  contracts; it does not add runtime plugin loading or a new authority path.

### `asha-gameplay-module-conformance` — public downstream proof runner

Status: task #5635 public build, bootstrap, invocation, state, and replay proof.

- Public facade: `public-rust/gameplay-module-conformance`.
- Owning engine authority: `engine-rs/crates/rules/rule-project-bundle` plus the
  existing gameplay fabric/state coordinators.
- Downstream dependency:

  ```toml
  asha-gameplay-module-conformance = { path = "../asha-engine/public-rust/gameplay-module-conformance" }
  ```

This crate is intended as a development or conformance dependency. It accepts a
statically linked `GameplayStaticComposition`, authored ProjectBundle-shaped
binding content, and typed root events. Its report is additive and
schema-versioned. Module code should depend only on
`asha-gameplay-module-sdk`; shipping runtime code does not need the conformance
runner.

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

### `contracts.v0` — explicit collision-camera movement mode

Surface: `@asha/contracts`
Change type: breaking
Introduced by: #5533
Source files: `protocol-view`, `protocol-codegen`

What changed:
- `CollisionConstrainedCameraInputEnvelope` now requires `movementMode: 'grounded' | 'freeFlight'`, and `CameraCollisionEvidence` echoes the selected mode.

Why this is engine-level:
- Rust collision-camera authority owns the locomotion basis and vertical-input policy; consumers must not infer or patch either behavior locally.

Downstream impact:
- Engine fixtures and `asha-demo` must select `grounded` for ordinary FPS movement. Tools that intentionally navigate vertically must select `freeFlight`.

Required migration:
1. Add `movementMode: 'grounded'` to FPS collision envelopes.
2. Use `freeFlight` only for intentional pitch-aware or vertical locomotion; grounded envelopes with nonzero `moveUp` fail closed.

Required evidence:
- `./harness/ci/check-contracts.sh`
- `./harness/ci/check-native.sh`
- downstream package-root typecheck/smoke

Fail-closed expectation:
- Consumers reject missing/unknown movement modes and do not preserve the former implicit behavior with local pose mutation.

## Runtime bridge compatibility log

### `runtime-bridge.v0` — initial local-path facade boundary

Status: current initial compatibility marker for the committed `@asha/runtime-bridge` facade and runtime bridge manifest family.

Source of truth:

- Bridge manifest: `engine-rs/crates/bridge/runtime-bridge-api/bridge-manifest.toml`.
- Generated conformance descriptor: `ts/packages/runtime-bridge/src/generated/conformance.json`.
- Runtime facade root export: `ts/packages/runtime-bridge/src/index.ts`.
- Check command: `bash harness/ci/check-bridge.sh`.

Consumer behavior:

- Consumers import RuntimeSession contracts and semantics from `@asha/runtime-session`, and concrete bridge construction/transport operations from `@asha/runtime-bridge`.
- Consumers never import `@asha/native-bridge` or `@asha/wasm-replay-bridge` as a runtime transport.
- Required operations must either be present on the facade or fail closed with classified `RuntimeBridgeError` kinds.
- `native_unavailable` and `operation_unimplemented` are acceptable diagnostics for missing native implementation during prototype/conformance work only when the task records an explicit gap; consumers must not silently fall back to raw transports.

Breaking facade/operation changes require a migration note using the template below.

Additive notes under `runtime-bridge.v0`:

- #5604 adds generated first-person/orbit/top-down controller state, revision-guarded mode/navigation receipts, and stable `apply_camera_mode_command`, `apply_camera_navigation_input`, and `read_camera_controller_state` operations through native and RuntimeSession. Rust owns accepted pivot, distance/height, angles, terrain clearance, pose, revision, and hashes. `BrowserInputHost` adds normalized wheel delivery plus a consuming `cameraNavigation` context, and `ResolvedCameraNavigationConsumer` sequences it with camera authority so FPS and pivot controllers cannot run together. `@asha/renderer-host` may sample receipt endpoints for disposable linear or smooth-step display, but interpolation never becomes authority or replay truth. The compatibility marker remains `runtime-bridge.v0` because the operations and named actions are additive; native addon and bridge packages must be rebuilt together.
- #5613 adds generated Session time-control contracts plus stable `apply_time_control_command` / `read_time_control_state` bridge operations and matching `RuntimeSessionFacade` methods. Rust owns pause, resume, bounded wall-clock cadence multipliers, and exact paused steps. HUD pause/resume intents and resolved named-input actions map to the same generated commands; headless callers use the same facade. Paused simulation leaves projection/inspection reads live, exact stepping remains paused, and speed never scales deterministic tick delta or replay state. The native `stepSimulation` result now carries its authoritative returned tick as well as diff count so paused transports cannot falsely report advancement. The compatibility marker remains `runtime-bridge.v0` because the public additions are additive; native addon and bridge packages must be rebuilt together.
- #5642 intentionally replaces the five-key `BrowserFpsInputCollector` compatibility surface with `BrowserInputHost`, `BrowserFpsResolvedActionConsumer`, and the stable RuntimeSession input operations. DOM normalization is centralized in the host; catalog validation, context priority/consumption, and resolution are Rust Session authority. Renderer controls now require an initialized public input port, and editor tools consume only resolved `editor.*` actions. This is a breaking v0 cleanup: there is no legacy collector export or fallback.
- #5643 adds `RecordedInputAction`, `InputActionReplayReceipt`, and stable `replay_resolved_input_action` / `replayResolvedInputAction`. Accepted input receipts issue semantic records containing resolved action plus catalog/context/record hashes and no platform control or browser event. Rust validates record integrity, current catalog/context, declaration/binding lineage, phase/value shape, and per-input-Session repeat delivery before returning an action. The default browser catalog resolves `Escape` to `runtime.time.pause`/`runtime.time.resume`; `ResolvedPauseContextConsumer` sequences public context and #5613 time-control commands so menu priority consumes gameplay/camera input while projection and inspection remain live. Gamepad, touch, IME/text composition, accessibility switches, modifiers/chords, and rebinding UI remain explicit non-claims. The compatibility marker remains `runtime-bridge.v0` because the replay surface is additive, while the default Escape action ids intentionally move from menu labels to time-control intent labels.
- #4547 starts the package decomposition campaign by moving transport-neutral RuntimeSession semantic readouts and proposal helper modules into `@asha/runtime-session`, while preserving the existing `@asha/runtime-bridge` root exports as compatibility re-exports. Runtime bridge still owns native transport access, launchers, render decode, generated bridge operations, reference helpers, and bridge-backed RuntimeSession facade adapters during this migration phase. Consumers may begin importing semantic readout/helper types from `@asha/runtime-session`, but existing approved `@asha/runtime-bridge` imports remain valid.
- #5506 completes that migration as an intentional breaking cleanup: `RuntimeSessionFacade`, lifecycle/ECRP/gameplay contracts, shared operation DTOs, and semantic helpers now resolve only from `@asha/runtime-session`. `@asha/runtime-bridge` exports concrete `createRuntimeSessionFacade` construction plus transport/launcher surfaces and removes the compatibility semantic re-export. Engine consumers were updated in place; satellite consumers must make the same package-root import split.
- #2564 adds three stable camera/view operations to the manifest-backed facade: `create_camera` / `createCamera`, `apply_first_person_camera_input` / `applyFirstPersonCameraInput`, and `read_camera_projection` / `readCameraProjection`. Native remains fail-closed with `operation_unimplemented` until a real native implementation lands; the mock/reference paths provide deterministic boundary evidence only. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #2895 adds one stable model/material preview/readback operation to the manifest-backed facade: `read_model_material_preview` / `readModelMaterialPreview`. The mock/reference facade derives a typed `RenderFrameDiff` from public `CatalogEntry` / `MaterialProjection` / `StaticMeshAsset` inputs. Native intentionally fail-closes with `operation_unimplemented` until a real native implementation is wired; consumers must not bypass this through renderer internals or raw transports. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4028 adds a semantic `RuntimeSession` facade exported from `@asha/runtime-bridge`: `RuntimeSessionFacade` types for initialize/load, typed command submission, deterministic tick, projection readout, telemetry/replay/hash summary, and restart. The reference helper `createMockRuntimeSession` is now explicitly imported from `@asha/runtime-bridge/reference` so production consumers do not pick up the mock backend through the root. It wraps the existing public bridge without adding raw transports or arbitrary JSON calls. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4030 adds browser FPS input collection and RuntimeSession camera input methods at the package root. `BrowserFpsInputCollector` maps structural keyboard/mouse/pointer inputs to a typed `runtime.apply_first_person_camera_input` command carrying `FirstPersonCameraInputEnvelope`, plus typed pointer-lock shell intents. `RuntimeSessionFacade` now exposes `createCamera`, `applyFirstPersonCameraInput`, and `readCameraProjection` wrappers over the existing public camera bridge operations. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4404 promotes `BrowserFpsInputCollector` into the shared browser/standalone FPS input ownership lane. It now exposes typed shell state (`active`, `disabled`, `paused`) and `drainInputFrame()` for runtime-neutral movement/look frames that renderer-host can consume without a RuntimeSession camera handle. `drainFrame()` remains the typed RuntimeSession camera/action proposal path. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4036 adds the first public typed runtime action/fire intent protocol at the `@asha/runtime-bridge` package root. Browser FPS primary-button press/release now emits `runtime.propose_runtime_action_intent` commands carrying `RuntimeActionIntentEnvelope` values (`primary_fire`, `pressed`/`released`, camera, tick, source, pressed state). `RuntimeSessionFacade.submitRuntimeActionIntent` accepts this typed proposal and returns a fail-closed `unsupported` receipt with `combat_runtime_not_wired` until #4051 wires combat/fire authority. Consumers must not replace this with raw JSON action tunnels. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4047 adds `RuntimeSessionFacade.applyCollisionConstrainedCameraInput`, a package-root wrapper around the generated `CollisionConstrainedCameraInputEnvelope` / `CameraCollisionSnapshot` bridge surface. The receipt exposes before/attempted/after motion evidence through the snapshot plus collided, blocked axes, world hash, collision projection hash, movement hash, and a replay record kind. The reference mock hosts the upstream static-room collision fixture so forward movement into the wall blocks while lateral movement in open space succeeds; consumers must still use this facade instead of demo-local physics, generated internals, native transports, or Rust crates. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4050 adds the public generated tunnel readout facade: `RuntimeSessionFacade.readGeneratedTunnelReadout` plus `TINY_GENERATED_TUNNEL_READOUT` and generated tunnel readout types from the `@asha/runtime-session` root export. The current `tiny-enclosed` v2 fixture exposes seed `17`, config hash `e1d156c6b55137a7`, output hash `1471496d88d70647`, replay hash `fnv1a64:0821a0c2aea17dff`, render projection hash `fnv1a64:21eb8696f6f3b5c4`, collision projection hash `fnv1a64:627389be013a3154`, spawn markers, material roles, runtime frame, and volume/corridor summaries. `RuntimeSessionFacade.requestGeneratedTunnelOperation` provides typed fail-closed receipts for `regenerate` and `apply_to_runtime_world`; consumers must not replace these with local generation or JSON action tunnels. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #5532 wires `requestGeneratedTunnelOperation({ operation: 'apply_to_runtime_world' })` through the stable `apply_generated_tunnel_to_runtime_world` bridge operation. Rust `svc-levelgen` regenerates the selected `tiny-enclosed` source and atomically installs its voxel world as runtime collision authority after FPS/ECRP load. Rust `svc-collision` owns the stable source/projection identity used by both apply and movement receipts. Explicit shooter/target role pairs remain semantic targeted-fire requests resolved by Rust FPS authority; anonymous ray fire remains voxel-occluded. The applied receipt exposes grid, config/output, collision-source, and runtime collision-projection hashes so consumers do not hardcode grid `1` or invent collision geometry. Collision-constrained movement continuously sweeps each slide axis and rejects per-axis travel above 256 world units without camera mutation, preventing endpoint tunneling while bounding query cost. `regenerate` remains unsupported and reference sessions retain explicit non-authority behavior. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #5533 intentionally breaks the pre-registry collision-camera envelope by requiring generated `movementMode` evidence. `grounded` derives forward/right from yaw only, keeps Y stable, and rejects nonzero vertical input before mutation; `freeFlight` explicitly preserves pitch-aware and vertical motion. The stable bridge operation name and RuntimeSession method are unchanged, and the compatibility marker remains `runtime-bridge.v0` while consumers pin the engine commit during the current v0 phase.
- #5547 completes the generated `VoxelCommand` native JSON border. `setVoxel`, `fillRegion`, and `generateChunk`, including both `empty` and `solid` voxel values, now parse through the bounded Rust bridge parser into the canonical `core-commands::VoxelCommand` union before authority validation. The public `RuntimeSessionFacade.submitCommands` shape is unchanged. An exhaustive public-root native consumer fixture makes generated command/value vocabulary growth fail TypeScript compilation until the native proof is updated. Unknown variants, malformed payloads, and extra fields fail closed. The compatibility marker remains `runtime-bridge.v0` because this fixes native conformance to the existing generated contract.
- #5556 completes the native `CommandResult` response border. `submitCommands` now returns generated tagged `VoxelEditRejection` DTOs instead of Rust debug strings, with exhaustive mappings for `unknownMaterial`, `emptyRegion`, `chunkNotResident`, and `generationDivergence`. The public native RuntimeSession proof asserts exact DTOs for the three rejection paths reachable through command submission, and its exhaustive generated-reason record plus Rust's exhaustive native match make vocabulary growth fail closed at compile time. Existing command resource bounds and the stable facade signature are unchanged.
- #4051 wires the public fire/combat/health reference readout: `RuntimeSessionFacade.submitRuntimeActionIntent` accepts `primary_fire` pressed intents and returns a `CombatRuntimeReadout` for the #4040 generated-tunnel hit/death fixture; `RuntimeSessionFacade.readCombatReadout` also exposes the geometry-blocked miss readout. Public root exports include `GENERATED_TUNNEL_FIRE_HIT_READOUT`, `GENERATED_TUNNEL_FIRE_MISS_READOUT`, and combat readout types. Hit/death evidence uses health hash `3c89045230f2d9d9` and replay hash `6b133026c511b0f5`; miss evidence uses health hash `56b1331c0f202ff1` and replay hash `3b1e1a9897571bc4`. HUD/menu rendering remains #4043, and consumers must not introduce local combat authority or JSON action tunnels. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4052 adds public nav/pathfinding readouts: `RuntimeSessionFacade.readNavProjection`, `queryNavPath`, and `readNavPolicyView`, plus nav readout constants/types from the `@asha/runtime-bridge` root export. The current generated-tunnel projection exposes walkable cells `45` and projection hash `59b4093625b10e49`; the reachable path readout has visited `41`, path length `9`, and path hash `09ed0284f7c175e1`; the no-path readout is typed as `blocked` with empty path hash `a8c7f832281a39c5`. `readNavPolicyView` is explicitly read-only/proposal-only and exposes no mutate/apply-path method. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4042 adds the first public constrained enemy policy fixture through the `@asha/runtime-bridge` root export: `createGeneratedTunnelEnemyPolicyFixture`, `createEnemyPolicyView`, `proposeEnemyPolicyFrame`, and `validateEnemyPolicySource`. The fixture consumes the read-only/proposal-only nav policy view, proposes a typed movement intent toward the generated tunnel target, and emits a `RuntimeActionIntentEnvelope` with source `enemy_policy` for primary fire; `RuntimeSessionFacade.submitRuntimeActionIntent` remains the authority path for fire/combat validation. The source validator rejects policy text that references clock, ambient randomness, network, DOM, filesystem, process, dynamic-code, or dynamic-import capabilities. Movement remains proposal-only in this slice until a runtime movement authority surface lands; consumers must not substitute demo-local state mutation, private policy packages, or JSON command tunnels. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4065 adds `RuntimeSessionFacade.runAutonomousPolicyTick`, a narrow autonomous enemy-policy loop readout for the generated tunnel fixture. Each tick advances the reference session, builds the read-only nav/policy view, validates typed policy proposals, rejects forbidden policy source capabilities, routes primary-fire proposals through `submitRuntimeActionIntent`, and reports proposal counts, movement/combat summaries, nav path hash, replay record hashes, and a deterministic tick hash. Movement proposals remain `unsupported` with `movement_authority_not_wired`; this is not a generic event bus, behavior tree, demo-local authority, or JSON command tunnel. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4066 adds public lifecycle/restart readouts on `RuntimeSessionFacade`: `readLifecycleStatus` and `requestSessionRestart`. The lifecycle readout reports player/enemy health/dead state, win/loss/in-progress outcome, restart eligibility, terminal lifecycle events, reset hash `fnv1a64:d0c05bd05488e8a5`, enemy-defeated lifecycle hash `fnv1a64:5fbf190733451da1`, and player-defeated fixture hash `fnv1a64:32322a108d4f2767`. `requestSessionRestart` validates typed `runtime.restart_session_intent` proposals from HUD/programmatic sources, rejects stale session hashes or non-terminal-gated requests with typed receipts, and resets the reference session deterministically through the RuntimeSession restart path. This does not add save/load persistence, UI authority, demo-local lifecycle mutation, or arbitrary JSON commands. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4399 adds `runtime_session.ecrp_render_target.v0` identity metadata to `renderProjection` CapabilityState readouts. The target object binds runtime entity id, EntityDefinition stable id/source, inferred role, projection kind, render label, current transform, optional visual scale, visibility, and target hash so demos can bind runtime entities to renderer-neutral targets without hard-coded label guesses. `renderHandle` remains `null` until a render-frame owner assigns retained renderer handles. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4400 adds the package-root helper `readRuntimeSessionPlayableLoopState(session, request?)`. The helper derives current-epoch HUD counters, health summaries, target identity, and command gating from public `RuntimeSessionFacade` lifecycle, telemetry, and ECRP readouts. `shotsFired` and `actionTick` count only replay records after the latest restart/request-restart boundary, preventing reset screens from treating historical commands as current-loop state. A missing backend fails closed with explicit diagnostics instead of asking consumers to invent local counters or command authority. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4401/#4521 add package-root native Rust provider helpers: `createNativeRustRuntimeBridgeProvider()`, `installNativeRustRuntimeBridgeProvider()`, `resolveNativeRustRuntimeBridgeProvider()`, and `assertNativeRustRuntimeBridgeAuthority()`. Browser/standalone hosts can install `globalThis.ashaRuntimeBridge` with provider kind `asha.runtime_bridge.native_rust_provider.v1`, or resolve the current `asha-demo` compatibility alias, and the resolver fails closed for missing providers, spoofed reference metadata, missing bridge objects, and missing required RuntimeBridge operations. Packaged standalone apps should install the provider in host/preload bootstrap before app boot; product authority must not fall back to reference/mock RuntimeBridge when no native provider exists. Loaded sessions still verify ECRP/FPS provenance through `assertNativeRustRuntimeBridgeAuthority`; provider metadata alone is not authority. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4403 adds the package-root helper `readRuntimeSessionPlayableEncounterTick(session, request)`. The helper derives the enemy actor/position from ECRP readouts, accepts the current RuntimeSession camera handle plus camera position from the shell, applies pause/player-dead/enemy-dead/missing actor gates, and advances one generated-tunnel autonomous policy tick through RuntimeSession. Receipts expose movement/combat/lifecycle summaries while keeping browser timer scheduling outside runtime authority. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4284 adds typed voxel conversion operations to `RuntimeSessionFacade`: `planVoxelConversion`, `previewVoxelConversion`, `applyVoxelConversion`, and `exportVoxelConversionEvidence`. The signatures use generated `@asha/contracts` voxel-conversion DTOs. Reference sessions deliberately fail closed with `operation_unimplemented`; consumers must not bypass this with raw native bridge calls, private generated paths, renderer buffers, Studio-owned voxelization, or JSON method tunnels. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4479 wires the Rust-backed RuntimeSession voxel conversion methods through bounded runtime bridge operations: `plan_voxel_conversion`, `preview_voxel_conversion`, `apply_voxel_conversion`, and `export_voxel_conversion_evidence`. The native/runtime bridge calls `svc-voxel-conversion` for plan/preview/apply/evidence DTOs, preserves plan/preview hash guards, and commits accepted output through the existing generated voxel command authority path rather than Studio or TypeScript mutation. Unsupported target grids or stale hashes return classified conversion diagnostics. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4553 adds bounded voxel model-info readout DTOs and the `RuntimeSessionFacade.readVoxelModelInfo` method through the stable runtime bridge. Consumers can read applied conversion model identity, bounds, voxel count, optional material counts, source/evidence refs, plan/output hashes, session hash, replay hash, and typed diagnostics for missing/unknown models without private state access. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #5264 adds bounded voxel model-window readout DTOs and `RuntimeSessionFacade.readVoxelModelWindow`, backed by the stable `read_voxel_model_window` runtime bridge operation. Rust validates requested bounds, material filters, empty-cell inclusion, and max sample counts before returning sample windows, and malformed or oversized reads fail closed with typed conversion diagnostics instead of exposing private state or whole-volume dumps. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #5589 confirms that `readVoxelModelWindow` is present on the compiled N-API addon, the public native RuntimeBridge provider, RuntimeSession, and the manifest-derived browser-host RPC surface. Native regression coverage reads a converted model window, verifies bounded-query rejection diagnostics, and compares occupied/material samples after save, unload, and load. Consumers should rebuild the tracked native addon and host packages together; a bridge object missing this stable method is stale and must fail provider validation rather than trigger a private transport workaround. The compatibility marker remains `runtime-bridge.v0` because this restores the already-published operation end to end.
- #4629 adds `RuntimeSessionFacade.registerVoxelConversionSource`, a typed source-registration wrapper over the existing runtime bridge voxel source registry. Rust-backed sessions delegate to the native/runtime bridge authority surface; reference sessions fail closed with `operation_unimplemented`. Consumers can now keep voxel conversion setup and plan/preview/apply/model-info flows on the RuntimeSession facade instead of mixing facade calls with raw bridge registration. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4908 adds `RuntimeSessionFacade.exportVoxelVolumeAsset`, backed by the stable `export_voxel_volume_asset` runtime bridge operation and generated `VoxelVolumeAssetExportRequest` / `VoxelVolumeAssetExportReceipt` DTOs. Rust exports the complete resident converted voxel model as an Asha-native `VoxelVolumeAsset` with sparse runs, material palette, provenance refs, canonical JSON, and `svc-voxel-asset` hashes; missing resident models, stale session hashes, sparse-run limits, and unrepresentable material refs fail closed through typed voxel-asset diagnostics. Consumers must not reconstruct stored voxel assets from preview samples or private conversion state. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #5295 extends `VoxelAssetMaterialBinding` with durable `paletteEntryId`, nullable `displayName`, and nullable `materialCatalogBindingId` fields. Named voxel palette/catalog binding authoring belongs to the stored `VoxelVolumeAsset` surface; runtime SessionState still consumes compact material ids through Rust-validated export/save/load paths. Rust rejects duplicate palette/binding identifiers and invalid `material/...` references before save/load succeeds. Studio follow-up should build material chooser and named palette editing against these public DTOs instead of maintaining a private material-binding model.
- #5495 adds `RuntimeSessionFacade.updateVoxelVolumeAssetPalette`, backed by the stable `update_voxel_volume_asset_palette` operation and generated request/receipt/diff DTOs. Consumers submit the current stored asset, a bounded complete replacement palette, required optimistic canonical/voxel hashes, and a ProjectBundle target. Rust returns a canonical updated stored asset only after validating both source and replacement; the operation preserves voxel content and cannot mutate Runtime SessionState. The native public consumer proof covers update, reopen/load, stale hashes, and duplicate bindings.
- #4909 adds `RuntimeSessionFacade.loadVoxelVolumeAsset`, backed by the stable `load_voxel_volume_asset` runtime bridge operation and generated `VoxelVolumeAssetLoadRequest` / `VoxelVolumeAssetLoadReceipt` DTOs. Rust validates stored `.avxl.json` asset hashes/schema/material refs through `svc-voxel-asset`, commits accepted sparse runs through voxel command authority, and returns runtime readback evidence with model id, bounds, counts, provenance, session hash, and replay hash. Rejected loads leave runtime voxel state untouched. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4910 adds `RuntimeSessionFacade.registerVoxelConversionMeshAsset`, backed by the stable `register_voxel_conversion_mesh_asset` runtime bridge operation and generated `VoxelConversionMeshAssetRegistrationRequest` DTO. Rust validates ProjectBundle/catalog static mesh identity, primitive support, indexed triangle groups, material-slot bindings, and later source-hash matches before plan/preview/apply can use the source. Consumers should pass project mesh refs through this facade path instead of inlining proof geometry or bypassing Rust source authority. The compatibility marker remains `runtime-bridge.v0` because the change is additive.

- #5553 adds `RuntimeSessionFacade.importVoxelConversionMeshSource`, backed by the stable `import_voxel_conversion_mesh_source` operation and generated import request/receipt DTOs. Hosts provide GLB bytes; `svc-mesh-import` owns subset validation, canonical geometry, material/group extraction, and SHA-256 provenance before the bridge registers the source. Studio may display the returned geometry and bounded metadata but must not parse GLB, claim source hashes, import raw native transport, or invent a private registration path. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4913 adds `RuntimeSessionFacade.saveVoxelVolumeAsset`, backed by the stable `save_voxel_volume_asset` runtime bridge operation and generated `VoxelVolumeAssetSaveRequest` / `VoxelVolumeAssetSaveReceipt` DTOs. Rust packages an explicit ProjectBundle stored-asset diff plus canonical payload after validating the resident runtime model, target asset path, sparse-run representation, expected output hashes, export limits, and material refs. Host/Studio code can write the returned payload only after accepting the receipt; SessionState is never silently promoted into stored content. The compatibility marker remains `runtime-bridge.v0` because the change is additive.

- #5552 adds `RuntimeSessionFacade.unloadVoxelVolumeAsset`, backed by the stable `unload_voxel_volume_asset` runtime bridge operation and generated `VoxelVolumeAssetUnloadRequest` / `VoxelVolumeAssetUnloadReceipt` DTOs. Rust owns the hash-guarded resident-model removal, restores the model's recorded prior voxel footprint, rejects missing/stale/drifted/overlapping state, and preserves unrelated resident models. Durable `.avxl.json` ProjectBundle content remains host-owned and can be loaded again through the public facade. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4911 adds an engine-owned public consumer proof in `@asha/smoke`: `pnpm --filter @asha/smoke test:persisted-voxel-proof`. The proof imports only `@asha/contracts` and `@asha/runtime-bridge`, runs against the built native RuntimeBridge, converts a project mesh asset, exports and saves a `VoxelVolumeAsset`, reloads it, verifies model-info readback, and records current run evidence under `harness/smoke-out/persisted-voxel-asset-consumer-proof.json`. The negative matrix covers bad content hash, bad coordinate system, invalid material ref, unsupported schema, stale runtime snapshot, and missing source evidence.
- #5278/#5513 add an engine-owned public consumer proof for voxel annotation layers in `@asha/smoke`: `pnpm --filter @asha/smoke test:voxel-annotation-proof`. The proof imports `@asha/contracts`, `@asha/runtime-session`, and concrete construction from `@asha/runtime-bridge`; creates and loads a target voxel-volume asset; submits a hashless `VoxelAnnotationLayerDraft`; receives the Rust-normalized `VoxelAnnotationLayer`; and then validates, loads, queries, edits, and exports it. The negative matrix covers quota diagnostics, stale target session hash, and stale annotation layer hash. Consumers must not bypass these verbs through `@asha/native-bridge`, generated file paths, Studio private transports, Rust crates, or arbitrary JSON method tunnels.
- #5538 adds grid `2` to the bounded native RuntimeSession conversion-target fixture while preserving default grid `1` and authored grid `7`. The public voxel-annotation consumer proof now plans/applies conversion, exports and reloads its stored voxel asset, and loads the annotation layer on grid `2`; `check-native.sh` runs that proof against the freshly built N-API addon. This is an additive fixture capability under `runtime-bridge.v0`, not a generic arbitrary-grid registration API.
- #4287 records the Studio voxel conversion adoption boundary for the #4284 facade methods. Studio may consume generated DTOs from `@asha/contracts`, runtime methods from `@asha/runtime-bridge`, command/evidence metadata from `@asha/command-registry`, and optional renderer-neutral projection/readout surfaces only through approved package roots. Unavailable backend support remains a typed fail-closed `operation_unimplemented` result rather than permission to use raw native bridge calls, private generated imports, renderer buffers as authority, Rust crates, VoxelForge runtime code, or Studio-owned mesh voxelization. The compatibility marker remains `runtime-bridge.v0` because this is documentation of additive surfaces.

## Runtime session compatibility log

### `runtime-session.v0` — semantic RuntimeSession package split

Status: unstable transport-neutral facade contract package. #5506 made it the sole RuntimeSession semantic contract home.

Initial root exports:

- runtime action intent envelopes and receipt status types;
- generated tunnel fixture/readout shapes;
- combat readout and combat feedback projection helpers;
- nav/path/policy-view readouts;
- encounter director readouts and transition helpers;
- enemy policy proposal/view helper shapes;
- ECRP render target identity metadata.
- `RuntimeSessionFacade` plus focused core, ECRP, lifecycle, gameplay, animation, and shared operation contracts.

Compatibility posture:

- Consumers import all RuntimeSession contracts and semantic readout/proposal surfaces from `@asha/runtime-session` root.
- `@asha/runtime-bridge` does not provide a compatibility semantic re-export.
- Concrete bridge-backed `createRuntimeSessionFacade` construction, native transport access, reference helpers, launchers, render decode, and generated bridge operation conformance remain in `@asha/runtime-bridge`.
- #5533 requires callers of `applyCollisionConstrainedCameraInput` to select generated `movementMode`; FPS callers migrate to `grounded`, while intentional vertical navigation selects `freeFlight`. The RuntimeSession method remains transport-neutral and does not permit downstream pose correction.

## Browser host compatibility log

### `browser-host.v0` — native browser/dev RuntimeBridge provider host

Status: unstable ASHA Game Project host surface introduced by #4878.

Initial root exports:

- `describeNativeBrowserHostCommand()`;
- `installNativeBrowserHostProvider()`;
- `readNativeBrowserHostProviderStatus()`;
- `launchNativeBrowserHost()`;
- `startNativeBrowserHost()`.

The host command shape is:

```sh
asha-browser-host --ui-root dist/ui --host 0.0.0.0 --port 5173
```

The host injects `/asha/browser-host/native-provider.js` into served HTML before
downstream app boot. That script installs `globalThis.ashaRuntimeBridge` with
provider kind `asha.runtime_bridge.native_rust_provider.v1`, then routes typed
RuntimeBridge methods to the upstream host-owned native bridge endpoint. Provider
status reports `rust_authority` only after the runtime bridge resolver accepts
the provider and required operations. Missing or spoofed providers report
`missing_rust_backend` with typed diagnostics and no reference fallback.

#5587 makes the native build part of the required GitHub `Verify ASHA` path and
adds a real browser-host HTTP stress regression over movement, player defeat,
restart, and structured invocation failure. A host and addon must be rebuilt as
one engine revision; a previously loaded addon is not valid evidence for a newer
TypeScript host. Native invocation errors return structured HTTP failures, and
the host must remain healthy for subsequent authority reads.

#5611 fixes live-addon replacement after a fresh-build SIGBUS was traced to an
invalid file-backed instruction page at the composition-status N-API wrapper.
Native addon installation now writes a complete temporary file beside the
destination and publishes it with one atomic rename. Existing hosts retain their
original mapped inode; newly launched hosts load the replacement. Build or test
automation must never truncate or copy directly over a loaded `.node` file.

#5674 adds optional static gameplay-host composition without changing the
`browser-host.v0` command shape. A downstream host may supply the public
`GameplayRuntimeHostTransport`; the injected provider exposes its closed
load/advance/read/save/restore surface through host-owned HTTP transport. Games
must continue to consume the typed RuntimeSession facade rather than inventing
per-game bridge methods or browser-side authority.

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

Additive notes under `command-registry.v0`:

- #4285 adds Studio command metadata for `voxel_conversion.plan`, `voxel_conversion.preview`, `voxel_conversion.apply`, and `voxel_conversion.export_evidence`. These entries use generated voxel-conversion DTO contract refs, declare RuntimeSessionFacade method requirements rather than raw bridge operations, and expose plan/preview/receipt/evidence artifact posture for Studio UI/timeline projection. They do not execute conversion, smuggle native calls, claim renderer authority, or replace the Rust/runtime fail-closed behavior from #4284. The compatibility marker remains `command-registry.v0` because the change is additive.
- #4287 clarifies that the command registry is the Studio metadata/readout lane for voxel conversion, not the executor. Studio may use the root export to discover menu placement, typed input/output contracts, artifact posture, retry/idempotency, and known limitations for the four `voxel_conversion.*` commands. Runtime execution still goes through `RuntimeSessionFacade`, and Rust/runtime diagnostics remain authoritative. The compatibility marker remains `command-registry.v0` because the change is additive.

## Devtools protocol compatibility log

### `devtools-protocol.v0` — unstable attach/readout protocol

Status: unstable observational protocol for Studio and synthetic testing consumers.

Consumer behavior:

- Consumers import only from `@asha/devtools` root export.
- Devtools is observational: it formats projected diagnostics, attach protocol state, and readouts; it does not mutate authority.
- Consumers must fail closed on unsupported protocol versions or missing evidence instead of replacing the typed protocol with generic JSON method tunnels.

## Game workspace compatibility log

### `game-workspace.v0` — unstable consumer workspace manifest

Status: unstable typed manifest/workspace and prefab-draft package for consumer
repos.

Consumer behavior:

- Consumers import only from `@asha/game-workspace` root export.
- `asha-testing` uses it for synthetic conformance/proof workflows.
- The new `asha-demo` may use it for human-facing project workspace setup, but should keep product identity separate from proof harness machinery.
- Manifest validation rejects private transport hints, ASHA internals, generated paths, and unsupported backend/profile claims.
- Prefab helpers expose explicit draft create/replace/delete/instantiate
  commands, browser/selection/role/binding/configuration readouts, and canonical
  source serialization. They do not own runtime authority; Rust still accepts
  only a validated registry and authoritative placement commands.

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
  its preset hash is `fnv1a64:b39b8794318889a7`, tuning hash is
  `fnv1a64:acbf1766f55dcba9`, reference hash is
  `fnv1a64:aff976e05786ce21`, and catalog hash is
  `fnv1a64:45a696e1e10c562f`. Demo constants should migrate by replacing local
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

Additive notes under `render-projection.v0`:

- #4402 moves generated-tunnel viewport and room frame composition into `@asha/render-projection`. The package root now exports `createGeneratedTunnelViewportFrame`, `createGeneratedTunnelRoomFrame`, `summarizeFirstPersonTunnelViewport`, and structural generated-tunnel frame input types. The room frame has stable signature hash `fnv1a64:cf70df6dccdf1758` for the tiny generated tunnel fixture. These helpers emit renderer-neutral `RenderFrameDiff` data only; they do not own runtime authority, local generation, collision authority, Three.js objects, or browser mounting.

## Renderer host compatibility log

### `renderer-host.v0` - unstable backend-neutral browser render host

Status: unstable root-barrel package surface for browser demo renderer mounting.

Consumer behavior:

- Consumers import only from `@asha/renderer-host` root export.
- Consumers mount a browser surface with `mountAshaRendererSurface(canvas, options)` and receive backend-neutral lifecycle, pointer-lock, movement-status, projection-snapshot, and interaction handles.
- Consumers feed the host public `RenderFrameDiff` values or helper-built frames. They do not import `@asha/renderer-three`, `three`, `THREE`, `WebGLRenderer`, or `ThreeRenderer`.
- Consumers may pass structural `runtime_session.ecrp_render_target.v0` target identity through `surfaceTargetProjectionFromRenderTarget` or a mounted surface's `projectRenderTargetProjection` method; renderer-host stays structurally typed and does not import `@asha/runtime-bridge`.
- Backend identity is diagnostic metadata only. The current implementation uses the engine-owned Three.js backend internally, but downstream call sites should not change if ASHA later swaps to Babylon.js or a native Rust renderer host.
- The host does not own gameplay, collision, combat, runtime authority, or command validation. Runtime intents still go through `@asha/runtime-bridge` and Rust authority surfaces.

Additive notes under `renderer-host.v0`:

- #5606 adds `AshaLiveTelemetryCollector` and `AshaTelemetryOverlayHost`. The
  collector exposes a bounded generated snapshot to headless and visual
  consumers, omits unsupported counters with typed diagnostics, and preserves
  stable names and units. The retained G1 overlay renders that exact snapshot
  through an injected sink; visibility remains projection-local and a missing
  overlay host cannot block scene or other presentation application. The
  compatibility marker remains `renderer-host.v0` because the root change is
  additive.
- #5603 adds `AshaParticleHost` under the generated G1 frame. Rust validates
  catalog-bound Sprite/SpriteSheet descriptors, burst and retained lifecycles,
  curves, seeds, and projection budgets; the host owns bounded per-particle
  simulation and sends renderer-neutral billboards to an injected sink. Missing
  anchors/resources and budget drops are typed, domain-local diagnostics. A
  one-way cosmetic adapter is available without turning particle realization
  into authority or replay truth. The compatibility marker remains
  `renderer-host.v0` because the root change is additive.
- #5597 adds `AshaBillboardHost` under the same generated G1 frame application.
  It realizes retained world/entity anchors as localized text, values, or
  hash-validated texture icons; loads catalog Font bytes through `FontFace`;
  and owns distance, viewport, depth-order, and renderer-supplied occlusion
  culling. Missing billboard hosts/resources degrade independently after scene
  application. Public descriptors and readouts contain no DOM or Three.js
  types, so this browser realization remains replaceable.
- #5595 adds `AshaAudioHost` and `applyAshaRuntimeProjectionFrame`. The host
  consumes generated G1 audio operations, verifies resolved bytes against the
  catalog SHA-256, owns SFX/ambient/UI buses and retained Web Audio graphs, and
  exposes typed listener updates, diagnostics, and bounded readout. Scene
  projection applies before presentation-domain realization, so unavailable
  browser audio degrades independently. The compatibility marker remains
  `renderer-host.v0` because the package-root change is additive.
- #5537 adds the browser-safe animated-mesh resource and playback path. `mountAshaRendererAnimatedMeshSurface` accepts a typed resource manifest/resolver, verifies SHA-256 and named clips, mounts the engine-owned backend, applies subsequent `RenderFrameDiff` values, advances projection-only mixers from render-frame deltas, and exposes bounded playback diagnostics/readback. `createAshaRendererAnimatedMeshProjection` provides the same path without a canvas for integration tests. `RuntimeSessionAnimationIntentReadout.instanceHandle` carries the generated `RenderHandle` brand, so consumers importing only `@asha/runtime-session` and `@asha/renderer-host` can pass an intent directly to projection or surface playback readback without casts or a separate contracts import. The package ships the CC0 Kenney GLB and license behind `ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST`; consumers do not use `harness/` paths or import the concrete backend. The compatibility marker remains `renderer-host.v0` because existing surface callers are unaffected.

## Renderer Three unstable status

`@asha/renderer-three` is explicit but unstable. It is an engine-owned Three.js implementation package for smoke/testing and the internal backend used by `@asha/renderer-host`; it should not be treated as the cross-repo renderer contract. Studio and demos should prefer `@asha/render-projection` for renderer-neutral ASHA semantics and `@asha/renderer-host` when they need a browser render surface.

Additive notes under this unstable status:

- #4029 historically widened the engine manifest so `asha-demo` could import the package root for the static-room render path only. The public helper `createStaticRoomRenderFrame` emits a synthetic `RenderFrameDiff`; backend rendering is now engine-owned behind `@asha/renderer-host` or `@asha/renderer-three/backend`. Evidence lives in `harness/fixtures/render-diffs/static-room.json` and `harness/goldens/render-diffs/static-room.snapshot`. This is structural render evidence only: no gameplay loop, runtime attachment, authority mutation, collision simulation, or browser screenshot is claimed.
- #4067 historically added the first-person generated-tunnel viewport adapter at the `@asha/renderer-three` package root. As of #4402, renderer-neutral generated-tunnel frame helpers live at `@asha/render-projection`; `@asha/renderer-three` no longer exports them from its package root or backend declarations. Concrete browser rendering remains behind `@asha/renderer-three/backend` for engine-owned smoke/testing and behind `@asha/renderer-host` for demos. The adapter consumes structural generated-tunnel readout data plus `CameraProjectionSnapshot` and creates deterministic tunnel shell/spawn-marker `RenderFrameDiff` data. Current viewport fixture hashes are frame `fnv1a64:db081afd570c2f30` and structural snapshot `fnv1a64:3abd4f9fa73fea4c`; generated tunnel projection hashes remain render `fnv1a64:21eb8696f6f3b5c4` and collision `fnv1a64:627389be013a3154`. This is still projection-only: no runtime authority, collision authority, local generation, animation system, or pixel golden is claimed.
- #5551 corrects the generated-tunnel coordinate contract: advertised dimensions are playable interior dimensions, Rust generates the voxel shell around that interior, and the apply receipt publishes `runtimeFrame.worldOffset`, `playableMin`, and `playableMax`. Renderer-neutral projection consumes those values rather than a hardcoded offset, and the public nav fixture now reflects the same 5-by-9 playable floor. The public demo spawn `[0, 1.62, 1.5]` with full-body half extents `[0.25, 0.7, 0.25]` can move through open space and remains shell-blocked; movement receipts retain the exact apply source/projection identities.
- #4387 narrows `@asha/renderer-three`: `asha-demo` is no longer an allowed consumer role, concrete renderer/browser-surface helpers moved behind the approved `./backend` export, and the depgraph check now rejects bare `three`/`@types/three` use outside approved renderer backend packages.

## Editor Tools unstable status

`@asha/editor-tools` is explicit but unstable. It holds editor-local state helpers and previews only; it does not validate or mutate authority. Studio may consume it through root exports while the engine manifest records it as an unstable editor/tooling surface.

## UI DOM unstable status

`@asha/ui-dom` is explicit but unstable. It holds render-agnostic UI projection/control descriptors for engine-owned UI surfaces; it does not own authority, runtime command execution, native transport, policy behavior, or a DOM framework requirement.

Additive notes under this unstable status:

- #4043/#4522 add `buildHudProjection`, `buildGameHudProjection`, and `hudControlToIntent` for typed HUD/menu projection. The basic projection exposes health, status, non-claim text, and resume/restart/options/exit controls as pure data. The game HUD projection adds player/target health bars, combat counters, input lock/fire/pause labels, pose labels, event rows, and pause-menu controls for FPS demos. `hudControlToIntent` emits typed proposals such as `runtime.restart_session_intent`; runtime/session code must still validate and execute restart behavior. No arbitrary JSON payloads or UI authority are introduced.

## Consumer pinning guidance

Until ASHA has registry/package publication, downstream consumers pin by local path plus ASHA git commit:

```json
{
  "dependencies": {
    "@asha/contracts": "file:../asha-engine/ts/packages/contracts",
    "@asha/runtime-bridge": "file:../asha-engine/ts/packages/runtime-bridge"
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
