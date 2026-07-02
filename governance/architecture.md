# Architecture overview

See docs/design.md for the full design document.

Core split: Rust owns authority. TypeScript owns expression and projection.
Generated contracts define the border.

Layer order (lowest to highest): foundation → state → protocol → sim/services/rules → render/wasm/tools.

## TypeScript package metadata axes

TypeScript package ownership has three independent axes in
`governance/ownership.toml`:

- `lane` is the agent assignment and review-routing scope.
- `type` is the package shape: `lib`, `shell`, `testing`, or `tool`.
- `layer` is the dependency-governance role: `protocol`, `transport`, `domain`,
  `renderer`, `components`, `shell`, `testing-fixtures`, or `tool`.

The depgraph verifier currently enforces that every TS package ownership entry
has valid `type` and `layer` metadata. Layer ordering is deliberately not
enforced yet; ordering rules should be reviewed as a separate governance change.

| Package(s) | Type | Layer | Notes |
|---|---:|---:|---|
| `@asha/contracts` | `lib` | `protocol` | Generated contract border. |
| `@asha/runtime-bridge`, `@asha/native-bridge`, `@asha/wasm-replay-bridge` | `lib` | `transport` | Runtime/replay transport facades and raw native wrapper. |
| `@asha/script-sdk`, `@asha/script-host`, `@asha/policy-*`, `@asha/catalog-*`, `@asha/editor-tools`, `@asha/command-registry`, `@asha/game-workspace` | `lib` | `domain` | Policy/catalog/editor/workspace domain logic and metadata. |
| `@asha/render-projection`, `@asha/renderer-three`, `@asha/cosmetic` | `lib` | `renderer` | Renderer-neutral projection state plus implementation packages. |
| `@asha/ui-dom` | `lib` | `components` | DOM component/readout layer. |
| `@asha/devtools` | `tool` | `tool` | Observational developer/operator tooling. |
| `@asha/app`, `@asha/electron-main` | `shell` | `shell` | Composition root and thin host wrapper. |
| `@asha/smoke` | `testing` | `testing-fixtures` | Launchable smoke/perf/testing harness. |

## Scene / world / asset foundation

- `core-assets` (foundation) — typed asset-reference vocabulary: `AssetKind`, validated
  kind-prefixed scoped-kebab-case `AssetId`, and typed `AssetRef<T>` / kind-erased
  `AssetReference`. Identity/validation only; the full catalog (resolution, DAG, locks,
  fallback) is deferred to the asset-registry work.
- `core-scene` (state) — authored scene documents and live world authority:
  - `SceneTree` (authoring/visualization) ⇄ `FlatSceneDocument` (canonical, flat, validated,
    deterministically serialized) with order-preserving round-trip.
  - `validate` — classified scene validation (duplicate/unknown-parent/cycle/transform/
    wrong-kind-asset-ref).
  - `WorldState` — live runtime authority; bootstrap copies initial transforms in, then
    runtime transforms are authority-owned and may diverge from the authored document.
  - `bootstrap` — atomic scene→authority initialization producing one `BootstrapRecord`
    replay unit with a deterministic world hash and a `scene node → entity` source trace.
- `SceneId` / `WorldId` / `SceneNodeId` live in `core-ids` and are distinct from
  `protocol-render::RenderHandle` (a derived projection handle, not authority).
- Authored scene documents and asset references are Rust-validated. Their cross-boundary
  schema lives in `protocol-scene` (protocol layer, `core-ids`-only): the flat scene
  document, classified validation report, `scene node → entity` source trace, and atomic
  bootstrap record, mirrored to `@asha/contracts` by `protocol-codegen`. Authority logic
  (validation, flatten, bootstrap allocation, serialization) stays in `core-scene`; TS can
  *author/inspect* a typed scene but never validates it (proven by the `@asha/contracts`
  scene smoke + the shared `harness/fixtures/scenes/` goldens Rust decodes/validates).

## World bundle / save serialization

- `svc-serialization` (services) — the inspectable world-bundle **format** and plans:
  - `WorldBundleManifest` — directory/manifest index with a classified artifact table
    (`durable` / `generated` / `cache`), bundle + protocol versions, world/scene identity,
    asset lock, generator metadata, and content hashes; validation fails closed on unknown
    newer versions. Std-only canonical JSON encode/decode.
  - `LoadPlan` — the deterministic, ordered, typed authority-load sequence with out-of-order
    and missing-prerequisite diagnostics.
  - `SavePlan` / `CompactionPlan` — the declarative, voxel-agnostic save + explicit-compaction
    description.
- `rule-world-bundle` (rules) — the **execution** that composes voxel persistence
  (`rule-voxel-edit`) the lower format crate cannot reach:
  - `compose` — fold chunk snapshots / edit logs into bundle sections with explicit save-time
    compaction; a compacted snapshot + retained edit log reconstructs identical chunk hashes.
  - `regen` — fail-closed generator-mismatch handling plus a development regenerate-and-replay
    conflict diagnostic (coordinate, old/new generated value, edit event id, suggested action)
    that never silently rewrites a save.
- The directory/manifest layout is canonical for development; a `.asha` archive is a future
  transport wrapper only. The cross-boundary schema lives in `protocol-world-bundle`
  (protocol layer, `core-ids`-only): manifest/artifact table, classified manifest +
  load-plan errors (version-compatibility findings), the ordered `LoadPlan`/`LoadStep`,
  save/compaction summaries, and the regenerate-and-replay generator diagnostic
  (`EditConflict`/`RegenConflictReport`), mirrored to `@asha/contracts` by
  `protocol-codegen`. Execution/validation stays in `svc-serialization`/`rule-world-bundle`;
  TS devtools *display* a manifest/load plan/generator diagnostic but cannot mutate bundle
  state (proven by the `@asha/contracts` smoke against `harness/fixtures/world-bundle/`).

## Asset registry / catalog validation

- `core-catalog` (state) — Rust-validated asset registry above the `core-assets` vocabulary:
  - `Catalog` / `CatalogEntry` + `validate` — catalog manifest validation (duplicate ids,
    material-payload placement, wrong-kind typed slots, missing dependencies) and a Rust-validated
    dependency `DependencyGraph` (DAG) with cycle-path diagnostics.
  - `AssetLock` + `generate_lock` / `validate_lock` — world-bundle asset locks and classified
    catalog-drift findings (missing / wrong-kind / stale version|hash / dependency drift /
    new-in-catalog); validation reports, never silently re-locks.
  - `MaterialDef` with the authority/style split: `collision_projection()` → `CollisionMaterial`
    (no visual fields), `render_projection()` → `RenderMaterial` (no collision class); plus
    `fallback_for(kind, context)` policy (collision-critical fails closed, cosmetic/overlay gets a
    debug placeholder).
  - `revalidate_asset` — single-asset change-impact diagnostics (reverse-DAG dependents; safe
    visual-only vs authority/structural change needing revalidation or full reload).
- TS may author catalog data; Rust owns validation. The cross-boundary schema lives in
  `protocol-assets` (protocol layer, foundation `core-assets` vocabulary only): catalog
  entries, classified `CatalogValidationError`/`LockFinding` reports, dependency cycle paths,
  the `FallbackDecision`, and the **disjoint** material projections — `RenderMaterial`
  (colour/texture/uv, no collision class) and `CollisionMaterial` (structural flags, no
  visual). A read-only devtools `MaterialProjection` may bundle both; the pure renderer path
  consumes only `RenderMaterial`. Mirrored to `@asha/contracts` by `protocol-codegen`; no TS
  package becomes a catalog validator (proven by the `@asha/contracts` smoke).

## Render assets / static meshes + sprites

- `protocol-render` (protocol) — the generated-contract border for authored render assets, mirrored
  to `@asha/contracts` by `protocol-codegen`:
  - `MeshPayloadDescriptor` gains a `MeshProvenance` tag (`voxelChunk` / `staticAsset` / `generated`
    / `debug`): voxel chunks and static meshes share one descriptor + one upload path.
  - `StaticMeshAsset` (shared geometry payload + `MeshMaterialSlot`s + `MeshCollisionPolicy`) and
    `StaticMeshInstanceDescriptor` (transform + per-slot material overrides) are separate; collision
    resolves via `resolve_collision()` to none / explicit proxy / AABB fallback (a physical mesh
    without a proxy is a classified error, not a silent non-physical mesh).
  - `SpriteInstanceDescriptor` (asset, frame, pivot, size + `SpriteSizeMode`, `BillboardMode`, tint,
    `renderOrder`, `SpriteDepthPolicy`, `SpriteShading`, `SpriteAttachment`) plus the `CreateSprite`
    / `UpdateSprite` diffs; `SpritePickHit` traces a pick to authority identity. Shading reserves
    lit/shadow/custom modes without forcing unlit-only.
- `@asha/renderer-three` (ts-shell) — the retained consumer: shares one `BufferGeometry` per static
  mesh asset with reference-counted disposal; renders sprites as pivot-shifted plane geometry (not
  `THREE.Sprite`); applies deterministic projection-driven sprite frame/tint updates; and exposes
  `pickSprite` returning an authority-facing trace. It validates nothing and imports no catalog or
  authority package.
- The renderer maps material asset ids → appearance via its slot/colour registry; full catalog
  `RenderMaterial` wiring, lit/shadow shaders, runtime buffer-handle mesh sources, and the offline
  glTF importer remain deferred.

## Scene / asset devtools diagnostics

- `protocol-diagnostics` (protocol) — the generated-contract border for diagnostic reports,
  mirrored to `@asha/contracts` by `protocol-codegen`. Types and **stable codes** only — no
  product logic, and (matching `protocol-render`) ids/coords are plain integers/strings at the
  border, so the crate needs no other crate:
  - `DiagnosticReport` / `DiagnosticReportSet` (scope + severity + stable `DiagnosticCode` +
    `DiagnosticSourceRef` + message + `SuggestedRemedy`), `SourceTrace` (render handle → scene
    node → entity → asset), and `RendererResourceReport` (observational counts).
  - `DiagnosticSeverity` ties to recovery policy: only `Fatal` blocks a load; `Error` degrades
    one node/entity/asset; `Warning`/`Info` never block. `DiagnosticCode` strings are a contract
    (added, never renamed); the codegen tables (`DIAGNOSTIC_*`) are sourced from the crate.
  - The scope set is consolidated (#2368) to cover every diagnostic-producing system: `scene`,
    `assetCatalog`, `worldBundle`, `renderProjection`, `rendererResources`, and `worldComposition`
    (load/save *execution* + save→reload equivalence, distinct from the bundle *format*). The
    composition codes `loadStageFailed`/`finalConsistencyMismatch` (Fatal) and `roundTripMismatch`
    (Error) give the runtime-composition work (#2361/#2362/#2364) stable codes to map into; a test
    asserts every scope is reachable by at least one code. Equivalence loss on a clean round-trip is
    `roundTripMismatch`, distinct from a genuinely corrupt artifact (`corruptBundleArtifact`).
- `scene-diagnostics` (tools) — observational emitters that map the existing classified
  validators (`core-scene`, `core-catalog`, `svc-serialization`, `rule-world-bundle`) and
  projection/resource state into those reports: scene/catalog/bundle diagnostics, render source
  traces, renderer resource reports, a deterministic text rendering for goldens, and a save→load
  **round-trip equivalence** check. Never mutates authority.
- Intentionally-broken fixtures + diagnostic goldens live under `harness/fixtures/diagnostics/`
  (regenerate with `BLESS=1 cargo test -p scene-diagnostics --test goldens`).
- **No Den coupling.** ASHA emits generic diagnostics/artifacts; an external workflow system may
  consume the codes/refs, but ASHA never imports or depends on Den. Enforced by
  `harness/ci/check-no-den-coupling.sh`.
- Deferred: TS devtools/UI panels that *display* these reports (the generated contracts are
  importable now — proven by the `@asha/contracts` smoke); a live renderer-side resource report
  built from the actual `renderer-three` handle registry; and explicit repair tooling (diagnostics
  report, they never auto-repair).
