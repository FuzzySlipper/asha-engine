---
status: current
audience: agent
tags: [voxel, annotation, regions]
supersedes: []
see-also: []
---

# Voxel Annotation And Semantic Region Model

Status: proposed by #5265. This is an implementation proposal, not yet a
shipped protocol surface.

## Decision

Asha should support semantic voxel regions and annotations, but not as a clone of
VoxelForge project tools and not as Studio-local state.

The model should be an ASHA-native pair of surfaces:

- Stored ProjectBundle/catalog data: a durable `VoxelAnnotationLayer` artifact
  that references one `VoxelVolumeAsset` and its expected voxel-data hash.
- Runtime SessionState overlay: a Rust-owned loaded annotation layer that can be
  queried, edited through typed receipts, and explicitly exported back to stored
  ProjectBundle data.

Annotations describe authored/editor/gameplay semantics over voxel cells. They do
not own voxel occupancy, material mapping, collision, rendering, or gameplay
authority by themselves.

## Non-Goals

- No `.vforge` import/export promise.
- No VoxelForge MCP, sidecar service, or region-tree API compatibility layer.
- No arbitrary JSON metadata blob attached to cells.
- No TypeScript-owned validation or direct SessionState mutation.
- No silent promotion from runtime overlay edits into stored ProjectBundle data.

## Stored Representation

Add a future `protocol-voxel-annotation` contract family with a top-level
`VoxelAnnotationLayer` DTO:

- `layerId`: stable asset id, such as `voxel-annotation/...`.
- `schemaVersion` and `mediaType`: fail-closed version/media gates.
- `targetVoxelVolumeAssetId`: the `voxel-volume/...` asset this layer describes.
- `targetVoxelDataHash`: expected `VoxelVolumeAsset.contentHashes.voxelData`.
- `targetBounds`: inclusive stored voxel-space bounds copied from the target
  volume at validation time.
- `regions`: ordered `VoxelAnnotationRegion[]`.
- `provenance`: authored/imported/runtime-export refs.
- `contentHashes`: canonical JSON hash and membership-data hash.
- `validationDiagnostics`: persisted observational diagnostics.

Authored input uses a distinct `VoxelAnnotationLayerDraft` with the same
identity, target, bounds, regions, and provenance fields but no content hashes
or validation diagnostics. `validateVoxelAnnotationLayer` accepts a tagged
`draft` or `finalized` input. A valid draft returns a normalized
`VoxelAnnotationLayer` whose hashes are computed by Rust; a finalized input is
checked against its submitted hashes. Unknown or mixed lifecycle fields fail
closed at the generated protocol border.

Each `VoxelAnnotationRegion` should carry:

- `regionId`: stable id unique inside the layer.
- `label`: non-empty human label, display only.
- `kind`: closed vocabulary string for known semantics, initially
  `selection`, `room`, `portal`, `spawn_area`, `cover`, `hazard`, `nav_hint`,
  and `custom`.
- `tags`: sorted unique lightweight labels.
- `parentRegionId`: optional tree parent. Validation rejects cycles and unknown
  parents.
- `bounds`: inclusive bounds enclosing every assigned cell.
- `selection`: compact membership payload.

The first membership representation should mirror stored voxel assets: sorted
sparse runs along +X, where absence means not in the region. This avoids a full
cell dump for typical authored areas and gives Rust deterministic hashing. Later
schema versions may add compressed bitsets or generated predicates, but schema v1
should stay inspectable JSON.

## Runtime Representation

Runtime annotation state is loaded explicitly from a validated
`VoxelAnnotationLayer` and attached to a runtime voxel model by target volume id
plus expected voxel-data/session hash.

Runtime overlays are useful for:

- inspection queries;
- Studio edit previews;
- agent read sets;
- runtime-to-stored export proposals.

They do not grant gameplay authority. If a downstream game wants a region to
affect gameplay, bootstrap or game-specific Rust rules must translate accepted
stored annotations into explicit EntityDefinitions, SceneDocument placements,
catalog entries, or rule-owned runtime data. Policies may read generated region
views and propose intents; Rust rules validate any resulting authority changes.

## Save, Load, And Reopen

ProjectBundle load order should validate annotation layers after voxel-volume
assets and before any consumer that references region ids.

Load should fail closed when:

- the target voxel volume is missing;
- the target voxel-data hash does not match;
- region membership lies outside target bounds;
- region ids duplicate;
- parent links cycle;
- quotas are exceeded.

Runtime edits should be exportable only through an explicit receipt:

1. Read current runtime layer hash.
2. Submit a typed annotation edit request with expected layer/session hash.
3. Rust validates and applies the edit to the runtime overlay.
4. Export produces a proposed `VoxelAnnotationLayer` plus ProjectBundle stored
   diff and canonical payload.
5. Host or Studio writes only that returned payload.

Reopen durability is then ordinary ProjectBundle durability: the saved annotation
layer reloads only if its target voxel asset/hash still matches or a future Rust
migration/repair operation accepts the mismatch.

## Minimum Public DTOs

The first implementation should define these generated DTOs:

- `VoxelAnnotationLayer`
- `VoxelAnnotationLayerDraft`
- `VoxelAnnotationLayerValidationInput`
- `VoxelAnnotationRegion`
- `VoxelAnnotationSelection`
- `VoxelAnnotationSparseRun`
- `VoxelAnnotationProvenanceRef`
- `VoxelAnnotationDiagnostic`
- `VoxelAnnotationLayerValidationRequest`
- `VoxelAnnotationLayerValidationReport`
- `VoxelAnnotationLayerLoadRequest`
- `VoxelAnnotationLayerLoadReceipt`
- `VoxelAnnotationQueryRequest`
- `VoxelAnnotationQueryReadout`
- `VoxelAnnotationEditRequest`
- `VoxelAnnotationEditReceipt`
- `VoxelAnnotationLayerExportRequest`
- `VoxelAnnotationLayerExportReceipt`

The runtime bridge exposes stable verbs rather than a generic JSON method:

- `validate_voxel_annotation_layer`
- `load_voxel_annotation_layer`
- `read_voxel_annotation_query`
- `apply_voxel_annotation_edit`
- `export_voxel_annotation_layer`

`RuntimeSessionFacade` exposes the matching semantic camelCase contract through
the `@asha/runtime-session` package root; concrete session construction remains
on `@asha/runtime-bridge`. Consumers use generated annotation DTOs from
`@asha/contracts` and these public package roots only; they must not import
generated file paths, raw native transports, Rust crates, Studio private
transports, or arbitrary JSON method tunnels. Pure readout/helper projections can later move to
`@asha/runtime-session` if they become transport-neutral.

## Validation Rules

Rust validation should enforce:

- closed schema/media versions;
- valid asset id prefixes and stable ids;
- target voxel asset id/hash match;
- ordered inclusive bounds;
- region bounds inside target bounds;
- sparse runs sorted by z, then y, then x;
- positive run lengths;
- no duplicate cell membership within one region;
- optional overlap policy, initially `allow_overlap`;
- quotas for max regions, max runs per region, max total assigned cells, max tag
  count, and max label length;
- parent tree acyclic and same-layer only;
- known `kind` vocabulary or `custom`;
- no unknown top-level fields in stored JSON;
- deterministic canonical JSON and membership hashes.

Suggested initial quotas should be generous but bug-catching:

- max regions per layer: 4096;
- max sparse runs per region: 16384;
- max total assigned cells per layer: 8,388,608;
- max tags per region: 32;
- max label length: 128 UTF-8 bytes.

## Relationship To #5264

#5264 adds bounded voxel model window reads. Annotation queries should compose
with that surface instead of replacing it:

- window reads answer "what cells/materials are here?";
- annotation queries answer "which validated semantic regions reference these
  cells or bounds?";

Both must remain quota guarded and Rust-owned.

## Implementation Path

1. Add `protocol-voxel-annotation` plus generated TypeScript contracts. **Done.**
2. Add `svc-voxel-annotation` for validation, canonical hashing, sparse-run
   normalization, and query helpers.
   **Done.**
3. Integrate annotation layer artifacts into ProjectBundle load/save metadata.
   **Done.**
4. Add runtime bridge verbs and RuntimeSession facade wrappers. **Done.**
5. Add a package-root provider regression and compatibility docs. **Done.** The regression is
   `pnpm --filter @asha/smoke test:voxel-annotation-provider`; when the native Rust
   bridge is built, it validates, loads, queries, edits, and exports annotations
   through `@asha/contracts` and `@asha/runtime-bridge` roots only.
6. Let Studio build UI only after the engine DTOs and receipts exist.
