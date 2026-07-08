# ASHA Vocabulary Compatibility Map

Status: implementation-facing compatibility map for Den task #4017.

Source planning docs:

- Den `asha/ecrp-vocabulary-taxonomy`
- Den `asha/m0-current-state-inventory-before-demo-fps-tasking`
- Den `asha/asha-demo-charter-start-checklist`

## Purpose

The accepted `asha-demo` campaign vocabulary uses **RuntimeSession**,
**SessionState**, and **ProjectBundle**. The current implementation still exposes
several active `World*` and project-bundle names. Those names are implementation
compatibility facts, not vocabulary for new demo-facing product docs.

New `asha-demo`, Studio, and campaign task prose should use the target
vocabulary. Refer to current `World*` names only when pointing at existing ASHA
main code, generated contracts, fixtures, or compatibility wrappers.

This document is a map, not a broad rename plan. Generated files must still be
changed only through protocol/codegen tooling.

## Compatibility Table

| Current symbol/path | Target vocabulary | Owner crate/package | Current consumers | Migration stance | Downstream demo implication |
|---|---|---|---|---|---|
| `core_scene::SpatialSessionState` in `engine-rs/crates/state/core-scene/src/spatial_session.rs` | Current precise name for the spatial runtime authority slice inside a broader future `RuntimeSession`/`SessionState` | `core-scene` / rust-state | `core-scene` bootstrap/tests, `render-bridge` scene projection, `rule-world-bundle` session-state snapshot/load, diagnostics/goldens | Migrated from `WorldState` as a breaking rename. Do not reintroduce a `WorldState` alias; a future full `SessionState` should compose this spatial slice rather than casually rename it. | `asha-demo` must not treat `SpatialSessionState` as the public game/runtime container. Demo docs should say RuntimeSession/SessionState and link to `SpatialSessionState` only as the current spatial implementation scope. |
| `SpatialSessionHash` / `SpatialSessionState::hash` in `core-scene` | Current precise hash for the spatial runtime authority slice | `core-scene` / rust-state | Session-state goldens, render/scene bootstrap tests, replay/equivalence docs | Migrated from `WorldHash`. Public hash readouts are split by authority domain: spatial session, voxel state, collision source, and runtime-session summary. | Demo proof language should avoid "world hash" for end-user project state; use replay/session hash language unless referencing current fixtures. |
| `ProjectId` / `RuntimeSessionId` in `core-ids`; `ProjectSection` in `svc-serialization` | `ProjectId` for stored ProjectBundle identity; `RuntimeSessionId` for live bootstrap/session handles | `core-ids`, `svc-serialization` | Manifests, bootstrap, serialization, bridge DTOs | ID taxonomy migrated as a breaking split. Serialization manifests now store a `ProjectSection`, and load plans derive an explicit `RuntimeSessionId` for bootstrap. | Do not introduce new demo-local IDs to paper over this. Use ASHA Game Project IDs in demo manifests and runtime-session IDs only for live loaded authority. |
| `ProjectBundleManifest` in `engine-rs/crates/services/svc-serialization/src/manifest.rs` | Current target service manifest vocabulary | `svc-serialization` / rust-service | JSON encode/decode, manifest/load-plan goldens, runtime bridge load/save | Migrated as a breaking rust-service rename. The service JSON section is now `project`, and service-owned goldens live under `harness/fixtures/project-bundle`. | A future `asha-demo` manifest should be described as a ProjectBundle or ASHA Game Project manifest. |
| `protocol-project-bundle` and generated `ts/packages/contracts/src/generated/projectBundle.ts` | Current target contract lane | `protocol-project-bundle`, `@asha/contracts` / contract-steward | `@asha/contracts` smoke, `@asha/devtools`, `@asha/runtime-bridge`, contract governance docs | Migrated as a breaking protocol/codegen rename. Do not reintroduce generated `worldBundle.ts` or legacy `WorldBundleManifest` aliases. | `asha-demo` may import `@asha/contracts` only at package root. It must not import generated file paths. |
| `rule-world-bundle` in `engine-rs/crates/rules/rule-world-bundle` | `rule-project-bundle` or `project-bundle load/save rules` | `rule-world-bundle` / rust-rule | Load executor tests, compaction/durability/regen fixtures, docs, `scene-diagnostics` | Future crate rename or wrapper after service rename. Execution remains Rust authority regardless of display name. | Demo load/save tasks should describe "ProjectBundle validation/load/save" and cite `rule-world-bundle` only as the current implementation lane. |
| `LoadStage::SessionStateSnapshot` / `RestoreSessionState` in serialization/rule load plans | Current target load stage vocabulary | `svc-serialization`, `rule-world-bundle` | Load-plan fixtures, devtools bundle panel | Migrated from `WorldStateSnapshot` / `RestoreWorldState`. Remaining lower-lane bundle naming belongs to the `svc-serialization` / `rule-world-bundle` ProjectBundle crate rename work. | Demo evidence should say "runtime/session restore" in prose. |
| `runtime_bridge_api::ProjectBundleLoadRequest`, TS `ProjectBundleLoadRequest` | Current target bridge DTO name | `runtime-bridge-api`, `@asha/runtime-bridge` / rust-bridge and ts-shell | Native bridge glue, runtime bridge mock/native/launcher, conformance tests, `@asha/devtools` bundle actions | Migrated in the bridge/native lane. Do not add compatibility aliases back to the old request names. Remaining lower-lane legacy work is service/rule bundle naming. | `asha-demo` should call package-root runtime facade methods and describe the behavior as loading a ProjectBundle into a RuntimeSession. |
| Bridge verbs `load_project_bundle` / `loadProjectBundle`, `save_project_bundle` / `saveProjectBundle`, `get_project_bundle_composition_status` / `getProjectBundleCompositionStatus`, `unload_project_bundle` | Current target bridge/native operation names | `runtime-bridge-api`, `native-bridge`, `@asha/runtime-bridge` | Bridge manifest, native fail-closed checks, runtime bridge generated operation tables, launcher, smoke/native checks | Migrated as a breaking bridge/native rename. Future work should not preserve the old verb names unless it is an explicitly documented external ABI bridge. | Demo code must use only package-root runtime facade methods and ProjectBundle/RuntimeSession language. |
| `@asha/devtools` bundle-panel project-bundle read models | ProjectBundle inspection/readout models | `@asha/devtools` / ts-tools | Studio/testing readouts; `asha-demo` is not an allowed direct consumer | Future rename after contract rename. No immediate alias needed for `asha-demo` because devtools should remain Studio/testing owned. | `asha-demo` must not import devtools directly to inspect bundles; Studio owns human readout. |
| `docs/launchable-voxel.md`, `docs/replay-model.md`, `harness/fixtures/project-bundle/*`, `harness/fixtures/session-state/*` | ProjectBundle/session replay fixtures | Docs/fixtures owned by their crate/package lanes | Rust/TS goldens and human operator docs | Doc labels can be clarified opportunistically, but fixture path renames should wait for code/protocol migration to avoid churn. | Demo tasks should not copy these proof fixtures as product skeleton. Use them only as upstream evidence references. |
| `ts/packages/game-workspace` manifest surface | ASHA Game Project manifest / ProjectBundle-facing workspace metadata | `@asha/game-workspace` / ts-shell or ts-tools boundary | `asha-testing`, `asha-demo`, `asha-studio` are allowed consumers by public-surface manifest | Already close to target vocabulary. Keep this as the downstream-facing name; do not backslide into project-bundle product wording. | `asha-demo` skeleton should use ASHA Game Project language and this package only through approved package-root imports. |

## Follow-Up Code Tasks

Current hash taxonomy:

- `spatialSessionHash` labels `SpatialSessionState::hash()` readouts and
  load/equivalence goldens.
- `voxelStateHash` labels voxel fixture/mesh state fingerprints.
- `collisionSourceHash` labels the voxel/collision source fingerprint used by
  camera collision receipts.
- `runtimeSessionSummaryHash` labels launcher/devtools projection summaries.

## Search Readback

Focused inventory command used for this map:

```sh
rg -n "\\bSpatialSessionState\\b|\\bProjectBundleManifest\\b|\\bProjectBundleLoadRequest\\b|\\bProjectBundleSaveSummary\\b|\\bloadProjectBundle\\b|\\bload_project_bundle\\b|\\bsaveProjectBundle\\b|\\bsave_project_bundle\\b|\\bgetProjectBundleCompositionStatus\\b|\\bget_project_bundle_composition_status\\b|\\bProjectSection\\b|\\bLoadStage::SessionStateSnapshot\\b|RestoreSessionState|project-bundle|project bundle" \
  engine-rs/crates/state/core-scene \
  engine-rs/crates/services/svc-serialization \
  engine-rs/crates/protocol/protocol-project-bundle \
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
`protocol-project-bundle`, `rule-world-bundle`, `runtime-bridge-api`,
`@asha/contracts`, `@asha/runtime-bridge`, `@asha/devtools`, repo docs, and
fixtures. These are live compatibility surfaces rather than stale prose.
