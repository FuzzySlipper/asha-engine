# Procedural environment authoring

Procedural environments enter a project through a Rust-owned authoring
transaction. Generation is a way to create ordinary inspectable project
content; it is not a second runtime-only level format. The first registered
provider is `asha.tunnel.enclosed.v2` with preset `tiny-enclosed`.

## Stored result

The transaction produces two canonical files:

- a schema-4 `SceneDocument` containing one voxel-volume node at the authored
  placement plus typed marker children; and
- a local-space `VoxelVolumeAsset` with generated provenance and canonical
  voxel-content hashes.

The asset grid always starts at local origin. Scene hierarchy owns placement.
The current shared rendering and collision admission supports composed
translation only, so Rust rejects voxel-volume nodes whose local or inherited
transform produces `nonIdentityRotation` or `nonUnitScale`.

Generation provenance identifies the provider, provider version, preset, seed,
configuration hash, and output hash. Source readout identifies the exact voxel
data used for collision and navigation derivation. A fresh RuntimeSession loads
the saved scene and voxel asset and derives rendering and collision from those
files; it does not rerun the procedural recipe.

The complete recipe identity is durable authoring evidence on the generated
asset's `provenance` entry. Its canonical `asha-generator://` URI carries the
provider, preset, provider version, seed, and configuration hash; the entry's
content hash is the generator output hash. Materialization consumes the matching
scene bootstrap generator binding while preserving any catalog bindings. Runtime
admission neither resolves nor registers the generator for the saved artifacts.

## Public workflow

The Rust-backed `WorkspaceAuthoringFacade` exposes this sequence:

1. `open` starts a generation-bound authoring workspace without starting
   gameplay.
2. `decodeSceneDocument` strictly decodes and installs the current canonical
   scene into the Engine-owned workspace scene set.
3. `previewProceduralEnvironment` supplies the provider, preset, seed, explicit
   scene/asset/node/marker identities, caller bounds, workspace generation and
   revision, and the Engine-issued scene content hash.
4. An accepted preview returns canonical files, typed scene and asset values,
   provenance, source readout, and a renderer-neutral preview frame. It does
   not mutate voxel, scene, revision, collision, or RuntimeSession authority.
5. `applyProceduralEnvironment` consumes the returned candidate identity at
   the same workspace generation and revision. Rust installs its retained
   candidate and advances the working revision once. Missing, foreign, stale,
   and reused candidates are rejected before mutation.
6. A trusted host writes both returned canonical files and calls
   `confirmStored` with the accepted artifact-set hash. The host must not write
   preview bytes or caller-substituted content.

The provider registry is closed and statically linked in
`svc-environment-authoring`. Requests cannot register providers or inject
generator callbacks. Provider and preset failures, recipe mismatch, stale scene
identity, invalid targets, output limits, and invalid generated artifacts are
returned as typed diagnostics without partially changing the workspace.

## Current limits

- Only `asha.tunnel.enclosed.v2` / `tiny-enclosed` is registered.
- The preset requires explicit durable target identities for the generated
  voxel node and its `player_start` and `exit_hint` marker nodes.
- Preview frames are renderer-neutral diffs, not a renderer-owned scene or a
  gameplay RuntimeSession.
- File selection, atomic multi-file replacement, and recovery UI belong to the
  trusted downstream host. Rust owns candidate bytes and acceptance; it does
  not directly choose host paths or perform filesystem writes.
