# Architecture overview

See docs/design.md for the full design document.

Core split: Rust owns authority. TypeScript owns expression and projection.
Generated contracts define the border.

Layer order (lowest to highest): foundation → state → protocol → sim/services/rules → render/wasm/tools.

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
- Authored scene documents and asset references are Rust-validated; no `protocol`/codegen
  border surface exists for them yet — it lands when scene/bootstrap shapes cross to TS.

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
  transport wrapper only. No `protocol`/codegen border surface is added yet.

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
- TS may author catalog data; Rust owns validation. No `protocol`/codegen surface added yet — it
  lands when catalog/descriptor shapes cross to TS (static-mesh/sprite rendering, devtools).
