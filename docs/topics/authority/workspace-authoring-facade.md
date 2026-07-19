---
status: current
audience: consumer
tags: [workspace, authoring, facade, consumer]
supersedes: []
see-also: []
---

# Workspace authoring authority

`WorkspaceAuthoringFacade` is the public engine surface for editors and asset
tools that need Rust validation and mutable working state without attaching a
gameplay `RuntimeSession`.

## The four planes

| Plane | Meaning | Example |
| --- | --- | --- |
| Stored | Durable ProjectBundle or host-file truth | `assets/voxels/door.avxl.json` |
| Authoring working state | Rust-owned mutable state used while editing | An opened voxel volume with unsaved edits |
| Live runtime | Gameplay SessionState after bootstrap | The door loaded into a running game |
| Projection | Derived display/tooling output | Mesh preview, inspector readout, renderer frame |

The authoring facade owns only the second plane and explicit transactions at the
stored boundary. It has no tick, gameplay module, combat, camera, scheduler, or
live-session lifecycle methods.

## Lifecycle

1. A consumer obtains a fresh public `RuntimeBridge` provider cell.
2. `createWorkspaceAuthoringFacade({ bridge })` constructs a narrow authoring
   facade.
3. `openProject({ source, ... })` reads the canonical ProjectBundle manifest
   closure through the shared source loader, initializes the Rust authoring
   cell, and installs every accepted scene and ProjectContent document. Stored
   voxel assets are loaded as authoring working copies. It does **not** call a
   runtime-project loader, activate gameplay module state, or construct a
   `RuntimeSessionFacade`. The lower-level `open(...)` remains for focused
   asset tools and compatibility adapters that do not open a project source.
4. The consumer creates, imports, converts, edits, inspects, and exports assets
   through the typed authoring operations.
5. `readProjection()` projects dirty Rust-owned voxel chunks through the same
   generated `RenderFrameDiff` / `MeshPayloadDescriptor` surface consumed by
   engine render hosts. The summary binds the frame to workspace identity,
   generation, and working revision; it does not start gameplay. The first read
   in each generation is marked `delivery: 'replace'` so a retained renderer
   clears handles from the prior workspace. Later reads are incremental `apply`
   deliveries.
   A scene-aware editor first calls `configureVoxelProjectionInstances(...)`
   with its registry digest and voxel scene-node bindings. Rust creates one
   retained root per instance and parents asset-local chunk meshes below it;
   moving one scene node therefore emits one root transform update rather than
   rebasing voxel cells or rebuilding another instance.
6. ProjectContent edits use `applyProjectContentCommand(...)`; the facade fills
   the current workspace, generation, revision, and Engine-owned set hash.
7. A whole-project save uses `prepareProjectWrite(...)`. Rust derives one opaque,
   revision-bound candidate containing the next manifest plus canonical writes,
   moves, and deletes. The trusted host applies it atomically, then
   `confirmProjectWrite(...)` consumes the matching publication once.
   `saveVoxelVolumeAsset(...)` / `confirmStored(...)` remain explicit
   asset-specific transactions for working assets that have not joined that
   project candidate.
8. `close(...)` rejects unsaved working state unless the caller explicitly opts
   into discarding it. Workspace identity and generation mismatches fail closed.

Opening a validated stored `VoxelVolumeAsset` through
`loadVoxelVolumeAsset(...)` creates an authoring working copy and records it as a
clean stored baseline. Loading that asset into gameplay is a separate
`RuntimeSession` operation performed by the live-runtime consumer.

## Authority posture

- Rust continues to own voxel validation, conversion, edit application,
  inspection, canonical serialization, quotas, stale hashes, and history.
- TypeScript owns lifecycle orchestration and the explicit host-write
  transaction. It does not own a parallel voxel model, manifest topology, role
  table, or content hash implementation.
- The server-local host may observe files, request a relocation, stage Rust
  buffers, and perform an atomic directory swap. It cannot mint a candidate or
  confirm a different store identity.
- Authoring lifecycle hashes describe orchestration state. Content/session,
  replay, canonical JSON, and voxel-data hashes remain Rust-issued evidence.
- Existing resource limits and malformed-input rejection are inherited from the
  same bridge operations used by runtime consumers; the facade does not bypass
  them.
- `pickVoxelInstance(...)` accepts a world ray and an optional renderer-derived
  local cell/face observation, but Rust inverse-transforms and re-casts the ray
  against asset-local collision authority. The returned cell, face, and place
  anchor are local coordinates bound to workspace generation, working revision,
  registry digest, instance identity, transform set, and voxel-world hash.
- Any working voxel edit invalidates the prior pick binding. The retained roots
  continue to receive remeshed chunk payloads, while a consumer must configure
  the current revision before it can obtain another edit anchor.

## Non-claims

- Opening workspace authoring does not start or attach a game.
- A Rust save receipt is not proof of host persistence.
- A workspace/repository manifest is tooling metadata, not the canonical
  `asha.project-bundle.json` runtime closure.
- Authoring working state is not silently promoted into ProjectBundle truth.
- Renderer projections are not authority.
- Renderer intersections are hints, not voxel-coordinate authority.
