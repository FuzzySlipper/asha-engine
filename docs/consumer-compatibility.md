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

Tier 1 public packages carry `asha.compatibility` in `package.json` and a package-local
`compatibility.json` file. Some unstable surfaces carry package-local compatibility
metadata while their consumer role is still being ratified.

| Surface | Status | Metadata file | Compatibility version | Role |
|---|---|---|---|---|
| `@asha/contracts` | `public` | `ts/packages/contracts/compatibility.json` | `contracts.v0` | Generated semantic DTO/type border from Rust protocol crates. |
| `@asha/runtime-bridge` | `public` | `ts/packages/runtime-bridge/compatibility.json` | `runtime-bridge.v0` | Transport-neutral runtime facade, manifest-backed operation vocabulary, typed errors. |
| `@asha/command-registry` | `unstable` | `ts/packages/command-registry/src/manifest.golden.json` | `command-registry.v0` | Studio command/evidence metadata registry. |
| `@asha/devtools` | `unstable` | `ts/packages/devtools/compatibility.json` | `devtools-protocol.v0` | Observational attach/readout protocol for tools and testing harnesses. |
| `@asha/game-workspace` | `unstable` | `ts/packages/game-workspace/compatibility.json` | `game-workspace.v0` | Typed game/workspace manifest validation for consumer repos. |
| `@asha/render-projection` | `unstable` | `ts/packages/render-projection/compatibility.json` | `render-projection.v0` | Renderer-neutral retained render-diff application model. |

Additional unstable package statuses:

- `@asha/editor-tools` is an unstable Studio/editor helper package. It is editor-local state only, not authority.
- `@asha/renderer-three` is an unstable Three.js implementation package for engine smoke/testing and the approved `asha-demo` static-room render path. It is not the long-term public renderer contract; consumers should prefer `@asha/render-projection` for renderer-neutral retained semantics unless a task explicitly approves the root package binding.

Internal packages, including `@asha/native-bridge`, `@asha/wasm-replay-bridge`, `@asha/app`, `@asha/electron-main`, `@asha/ui-dom`, policy/catalog packages, and `@asha/smoke`, are not downstream public surfaces.

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
| `@asha/game-workspace` | `unstable` | Allowed for manifest/workspace validation | The current typed ASHA Game Project manifest/workspace surface. This is the preferred first skeleton dependency. |
| `@asha/render-projection` | `unstable` | Allowed for renderer-neutral projection state only | Consumers may use retained render-diff projection semantics through the root package. This is not permission to mutate authority or decode arbitrary JSON. |
| `@asha/renderer-three` | `unstable` | Allowed for the static-room renderer path approved in #4029 | Consumers may import only from the package root and must treat it as an implementation binding over public render diffs/projection state, not as authority or a stable renderer contract. |
| `@asha/command-registry` | `unstable` | Optional, only for declared command/readout metadata | Useful for Studio-compatible typed command/evidence metadata. The skeleton should not require it unless it has a concrete manifest/readout need. |

The first skeleton must not import these ASHA surfaces directly:

| Forbidden surface | Decision |
|---|---|
| `@asha/devtools` | Remains Studio/testing-only. Studio owns live/runtime readouts; `asha-demo` should not make devtools a direct product dependency. |
| `@asha/script-sdk`, `@asha/script-host`, `@asha/policy-core`, `@asha/policy-examples` | Remain internal. Demo-owned policy packs are deferred until ASHA main exposes a public policy-authoring/packaging surface. `@asha/game-workspace` already classifies policy source authoring as reserved/deferred. |
| `@asha/native-bridge`, `@asha/wasm-replay-bridge` | Remain internal. Runtime access goes through `@asha/runtime-bridge`; replay/WASM proof paths stay engine/testing-owned. |
| ASHA package `src/*` or `dist/generated/*` paths | Forbidden. Consumers use package roots only. |
| Rust crate paths or generated contract hand edits | Forbidden. Protocol changes go through Rust protocol source plus `protocol-codegen`. |

Renderer decision for this gate: the initial `asha-demo` skeleton still must not
claim a Three.js-rendered game, runtime attachment, motion, collision, or gameplay.
Task #4029 approves a narrow upstream path for a synthetic static-room renderer
binding: `asha-demo` may import `@asha/renderer-three` from the package root only,
using public render diffs / `@asha/render-projection` semantics and the
`createStaticRoomRenderFrame` / `renderProjectedFrame` evidence path. This remains
unstable and does not promote `@asha/renderer-three` to the long-term renderer
contract.

Policy decision for this gate: no demo-owned TypeScript policy package is
allowed yet. Catalog or policy directories may exist as documented placeholders
only if they do not import internal ASHA policy packages and do not claim runtime
policy execution.

No manifest change was made for #4018 because the current engine manifest already
encodes the intended roles: `asha-demo` may use the allowed package roots above,
while renderer-three, devtools, raw transports, replay bridge, and policy authoring
packages remain outside the demo boundary.

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
- #4028 adds a semantic `RuntimeSession` facade exported from `@asha/runtime-bridge`: `createMockRuntimeSession` plus `RuntimeSessionFacade` types for initialize/load, typed command submission, deterministic tick, projection readout, telemetry/replay/hash summary, and restart. It wraps the existing public bridge without adding raw transports or arbitrary JSON calls. The compatibility marker remains `runtime-bridge.v0` because the change is additive.
- #4030 adds browser FPS input collection and RuntimeSession camera input methods at the package root. `BrowserFpsInputCollector` maps structural keyboard/mouse/pointer inputs to a typed `runtime.apply_first_person_camera_input` command carrying `FirstPersonCameraInputEnvelope`, plus typed pointer-lock shell intents. `RuntimeSessionFacade` now exposes `createCamera`, `applyFirstPersonCameraInput`, and `readCameraProjection` wrappers over the existing public camera bridge operations. Primary fire remains an explicit `unsupported_primary_fire` readout until a public action/fire protocol exists. The compatibility marker remains `runtime-bridge.v0` because the change is additive.

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

## Render projection compatibility log

### `render-projection.v0` — unstable renderer-neutral retained projection

Status: unstable root-barrel package surface for renderer-neutral render-diff application.

Consumer behavior:

- Consumers import only from `@asha/render-projection` root export.
- Consumers feed it decoded `RenderFrameDiff` / `RenderDiff` values from public contracts or runtime facade helpers; it does not decode arbitrary JSON or call raw transports.
- Consumers bind returned neutral application instructions or retained snapshots into their renderer of choice. Three.js is one possible binding, not the public ASHA contract.
- The projection fails closed on duplicate/stale handles, unsupported diff operations, malformed mesh payloads, and in-use static mesh redefinitions.

## Renderer Three unstable status

`@asha/renderer-three` is explicit but unstable. It is an engine-owned Three.js implementation package for smoke/testing and should not be treated as the cross-repo renderer contract. Studio and demos should prefer `@asha/render-projection` for renderer-neutral ASHA semantics and keep Three.js code as a local binding.

Additive notes under this unstable status:

- #4029 widens the engine manifest so `asha-demo` may import the package root for the static-room render path only. The public helper `createStaticRoomRenderFrame` emits a synthetic `RenderFrameDiff`; `renderProjectedFrame` applies that frame through `@asha/render-projection` and the retained `ThreeRenderer`. Evidence lives in `harness/fixtures/render-diffs/static-room.json` and `harness/goldens/render-diffs/static-room.snapshot`. This is structural render evidence only: no gameplay loop, runtime attachment, authority mutation, collision simulation, or browser screenshot is claimed.

## Editor Tools unstable status

`@asha/editor-tools` is explicit but unstable. It holds editor-local state helpers and previews only; it does not validate or mutate authority. Studio may consume it through root exports while the engine manifest records it as an unstable editor/tooling surface.

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
