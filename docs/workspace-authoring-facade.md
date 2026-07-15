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
3. `open(...)` initializes the Rust authority process and loads the active
   ProjectBundle composition. It does **not** call `loadFpsRuntimeSession` or
   construct a `RuntimeSessionFacade`.
4. The consumer creates, imports, converts, edits, inspects, and exports assets
   through the typed authoring operations.
5. `readProjection()` projects dirty Rust-owned voxel chunks through the same
   generated `RenderFrameDiff` / `MeshPayloadDescriptor` surface consumed by
   engine render hosts. The summary binds the frame to workspace identity,
   generation, and working revision; it does not start gameplay. The first read
   in each generation is marked `delivery: 'replace'` so a retained renderer
   clears handles from the prior workspace. Later reads are incremental `apply`
   deliveries.
6. `saveVoxelVolumeAsset(...)` returns a validated canonical payload and stored
   diff proposal. It does not claim that the host wrote a file.
7. After the host successfully writes the canonical payload,
   `confirmStored(...)` binds the host path and canonical hash to the current
   workspace generation. Until confirmation, the authoring state remains dirty.
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
  confirmation. It does not own a parallel voxel model.
- Authoring lifecycle hashes describe orchestration state. Content/session,
  replay, canonical JSON, and voxel-data hashes remain Rust-issued evidence.
- Existing resource limits and malformed-input rejection are inherited from the
  same bridge operations used by runtime consumers; the facade does not bypass
  them.

## Non-claims

- Opening workspace authoring does not start or attach a game.
- A Rust save receipt is not proof of host persistence.
- Authoring working state is not silently promoted into ProjectBundle truth.
- Renderer projections are not authority.
