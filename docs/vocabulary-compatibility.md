# ASHA Vocabulary Compatibility Map

Status: implementation-facing compatibility map for Den task #4017.

Source planning docs:

- Den `asha/ecrp-vocabulary-taxonomy`
- Den `asha/m0-current-state-inventory-before-demo-fps-tasking`
- Den `asha/asha-demo-charter-start-checklist`

## Purpose

The accepted `asha-demo` campaign vocabulary uses **RuntimeSession**,
**SessionState**, and **ProjectBundle**. The current implementation still exposes
several active `World*` and world-bundle names. Those names are implementation
compatibility facts, not vocabulary for new demo-facing product docs.

New `asha-demo`, Studio, and campaign task prose should use the target
vocabulary. Refer to current `World*` names only when pointing at existing ASHA
main code, generated contracts, fixtures, or compatibility wrappers.

This document is a map, not a broad rename plan. Generated files must still be
changed only through protocol/codegen tooling.

## Compatibility Table

| Current symbol/path | Target vocabulary | Owner crate/package | Current consumers | Migration stance | Downstream demo implication |
|---|---|---|---|---|---|
| `core_scene::WorldState` in `engine-rs/crates/state/core-scene/src/world.rs` | `SessionState` scope, specifically the spatial runtime authority slice inside a future broader `RuntimeSession` | `core-scene` / rust-state | `core-scene` bootstrap/tests, `render-bridge` scene projection, `rule-world-bundle` world-state snapshot/load, diagnostics/goldens | Future rename or wrapper, not alias-only. The current type is spatial-only and excludes non-spatial/logical entities, so a full `SessionState` likely composes it rather than merely renames it. | `asha-demo` must not treat `WorldState` as the public game/runtime container. Demo docs should say RuntimeSession/SessionState and link to `WorldState` only as the current spatial implementation scope. |
| `WorldHash` / `WorldState::hash` in `core-scene` | `SessionStateHash` or `SpatialSessionHash` | `core-scene` / rust-state | World-state goldens, render/scene bootstrap tests, replay/equivalence docs | Future rename after hash semantics are split between spatial runtime state and whole-session replay/state hashes. | Demo proof language should avoid "world hash" for end-user project state; use replay/session hash language unless referencing current fixtures. |
| `WorldId` in `core-ids` and `WorldSection` in `svc-serialization` | `RuntimeSessionId` for live handles; `ProjectBundle`/project identity for stored bundle metadata | `core-ids`, `svc-serialization` | Manifests, bootstrap, serialization, generated contracts, bridge DTOs | Blocked by protocol compatibility. Needs an explicit ID taxonomy decision before code rename. | Do not introduce new demo-local IDs to paper over this. Use ASHA Game Project IDs in demo manifests and let upstream choose session/project ID names. |
| `WorldBundleManifest` in `engine-rs/crates/services/svc-serialization/src/manifest.rs` | `ProjectBundleManifest` | `svc-serialization` / rust-service | JSON encode/decode, manifest/load-plan goldens, `protocol-world-bundle`, runtime bridge load/save, TS contracts/devtools | Future protocol rename. Not alias-only because the generated contract module and fixture paths encode the current wire vocabulary. | A future `asha-demo` manifest should be described as a ProjectBundle or ASHA Game Project manifest, even if the initial upstream compatibility call still consumes a `WorldBundleManifest` DTO. |
| `protocol-world-bundle` and generated `ts/packages/contracts/src/generated/worldBundle.ts` | `protocol-project-bundle` / generated `projectBundle` contract | `protocol-world-bundle`, `@asha/contracts` / contract-steward | `@asha/contracts` smoke, `@asha/devtools`, `@asha/runtime-bridge`, contract governance docs | Blocked by generated contract compatibility. Rename requires Rust protocol source changes, codegen, generated TS updates, fixture updates, and consumer migration. | `asha-demo` may import `@asha/contracts` only at package root. It must not import generated file paths or hand-edit generated contracts to get target names. |
| `rule-world-bundle` in `engine-rs/crates/rules/rule-world-bundle` | `rule-project-bundle` or `project-bundle load/save rules` | `rule-world-bundle` / rust-rule | Load executor tests, compaction/durability/regen fixtures, docs, `scene-diagnostics` | Future crate rename or wrapper after protocol/service rename. Execution remains Rust authority regardless of display name. | Demo load/save tasks should describe "ProjectBundle validation/load/save" and cite `rule-world-bundle` only as the current implementation lane. |
| `LoadStage::WorldStateSnapshot` / `restoreWorldState` in protocol and serialization load plans | `restoreSessionState` or `restoreSpatialSessionState` | `svc-serialization`, `protocol-world-bundle`, `rule-world-bundle` | Load-plan fixtures, generated contracts, devtools bundle panel | Future wire migration. The snapshot currently restores runtime-diverged spatial entity state over bootstrapped scene baseline. | Demo evidence should say "runtime/session restore" in prose, but tests must expect current `restoreWorldState` wire labels until a protocol migration lands. |
| `runtime_bridge_api::ProjectBundleLoadRequest`, TS `ProjectBundleLoadRequest` | Current target bridge DTO name | `runtime-bridge-api`, `@asha/runtime-bridge` / rust-bridge and ts-shell | Native bridge glue, runtime bridge mock/native/launcher, conformance tests, `@asha/devtools` bundle actions | Migrated in the bridge/native lane. Do not add compatibility aliases back to the old request names. Remaining legacy work is protocol/source crate naming such as `protocol-world-bundle`. | `asha-demo` should call package-root runtime facade methods and describe the behavior as loading a ProjectBundle into a RuntimeSession. |
| Bridge verbs `load_project_bundle` / `loadProjectBundle`, `save_project_bundle` / `saveProjectBundle`, `get_project_bundle_composition_status` / `getProjectBundleCompositionStatus`, `unload_project_bundle` | Current target bridge/native operation names | `runtime-bridge-api`, `native-bridge`, `@asha/runtime-bridge` | Bridge manifest, native fail-closed checks, runtime bridge generated operation tables, launcher, smoke/native checks | Migrated as a breaking bridge/native rename. Future work should not preserve the old verb names unless it is an explicitly documented external ABI bridge. | Demo code must use only package-root runtime facade methods and ProjectBundle/RuntimeSession language. |
| `@asha/devtools` bundle-panel world-bundle read models | ProjectBundle inspection/readout models | `@asha/devtools` / ts-tools | Studio/testing readouts; `asha-demo` is not an allowed direct consumer | Future rename after contract rename. No immediate alias needed for `asha-demo` because devtools should remain Studio/testing owned. | `asha-demo` must not import devtools directly to inspect bundles; Studio owns human readout. |
| `docs/launchable-voxel.md`, `docs/replay-model.md`, `harness/fixtures/world-bundle/*`, `harness/fixtures/world-state/*` | ProjectBundle/session replay fixtures | Docs/fixtures owned by their crate/package lanes | Rust/TS goldens and human operator docs | Doc labels can be clarified opportunistically, but fixture path renames should wait for code/protocol migration to avoid churn. | Demo tasks should not copy these proof fixtures as product skeleton. Use them only as upstream evidence references. |
| `ts/packages/game-workspace` manifest surface | ASHA Game Project manifest / ProjectBundle-facing workspace metadata | `@asha/game-workspace` / ts-shell or ts-tools boundary | `asha-testing`, `asha-demo`, `asha-studio` are allowed consumers by public-surface manifest | Already close to target vocabulary. Keep this as the downstream-facing name; do not backslide into world-bundle product wording. | `asha-demo` skeleton should use ASHA Game Project language and this package only through approved package-root imports. |

## Follow-Up Code Tasks

Smallest useful follow-ups, in order:

1. Plan a contract-steward protocol migration for `protocol-world-bundle` to a
   ProjectBundle vocabulary, including generated TS contracts, fixture path
   decisions, bridge manifest changes, and compatibility windows.
2. Plan a rust-state/rust-rule split for the broader `RuntimeSession` model: keep
   the current spatial `WorldState` semantics either as a renamed spatial
   sub-scope or as an explicitly composed field inside `SessionState`.

## Search Readback

Focused inventory command used for this map:

```sh
rg -n "\\bWorldState\\b|\\bWorldBundleManifest\\b|\\bProjectBundleLoadRequest\\b|\\bProjectBundleSaveSummary\\b|\\bloadProjectBundle\\b|\\bload_project_bundle\\b|\\bsaveProjectBundle\\b|\\bsave_project_bundle\\b|\\bgetProjectBundleCompositionStatus\\b|\\bget_project_bundle_composition_status\\b|\\bWorldSection\\b|\\bLoadStage::WorldStateSnapshot\\b|restoreWorldState|world-bundle|world bundle" \
  engine-rs/crates/state/core-scene \
  engine-rs/crates/services/svc-serialization \
  engine-rs/crates/protocol/protocol-world-bundle \
  engine-rs/crates/rules/rule-world-bundle \
  engine-rs/crates/bridge/runtime-bridge-api \
  ts/packages/runtime-bridge \
  ts/packages/devtools \
  ts/packages/game-workspace \
  ts/packages/contracts/src \
  docs governance harness/public-surface \
  --glob '!**/dist/**' --glob '!**/target/**' --glob '!**/node_modules/**'
```

The readback confirmed active references in `core-scene`, `svc-serialization`,
`protocol-world-bundle`, `rule-world-bundle`, `runtime-bridge-api`,
`@asha/contracts`, `@asha/runtime-bridge`, `@asha/devtools`, repo docs, and
fixtures. These are live compatibility surfaces rather than stale prose.
