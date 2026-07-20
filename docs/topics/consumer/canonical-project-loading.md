---
status: current
audience: consumer
tags: [runtime-session, project-bundle, loading, downstream]
supersedes: []
see-also: [../authority/runtime-session-facade.md, ../authority/workspace-authoring-facade.md, ../bridge/runtime-session-composition.md]
---

# Canonical project loading

Ordinary downstream boot selects a project source and does not assemble a
runtime graph:

```ts
import { createAshaProjectDirectorySource } from '@asha/browser-host';
import { createRuntimeSessionFacade } from '@asha/runtime-bridge';

const runtimeSession = createRuntimeSessionFacade({ bridge, mode: 'rust' });
runtimeSession.initialize({
  sessionId: 'game.main',
  seed: 17,
  project: { gameId: 'game', workspaceId: 'workspace.local' },
});

const source = await createAshaProjectDirectorySource('/absolute/game/project');
const receipt = await runtimeSession.loadProject({ source });
if (!receipt.accepted) throw new Error(JSON.stringify(receipt.diagnostics));
```

The same call accepts `createAshaProjectPackageFileSource('/server/game.asha')`
from `@asha/browser-host` for a server-local package, or
`createPackagedAshaProjectSource(...)` when archive bytes are already owned by
the host. `createMemoryAshaProjectSource(...)` is the bounded test adapter. The
shared loader reads the canonical manifest first and then only the paths it declares.
Every body crosses the bridge through the manifest-bound binary resource
transport; callers do not encode large assets in JSON.

Rust revalidates the manifest closure, resolves saved scenes and project
content against the statically linked provider composition, derives runtime
entity/prefab/trigger/read/scheduler/resource plans, and commits once. The
accepted receipt contains the Rust-owned project, manifest, admission,
content-set, composition, scene/entity/voxel, generation, and revision
identities. A rejected receipt leaves project authority unactivated.

A downstream provider that supports a product domain installs its typed Rust
adapter when it constructs the deferred RuntimeSession. For example, the FPS
provider uses `with_project_domain(RuntimeProjectDomainAdapter::Fps)`. The
adapter is both available and required for that provider: canonical admission
must satisfy its typed topology or fail without publication. Rust never selects
a domain by scanning saved controller, faction, policy, or projection strings,
and TypeScript does not repeat that decision. Domain rejections retain the
stable ProjectContent document ID and independent manifest source path down to
the relevant definition field, so authors never need to reverse-map runtime
entity numbers to repair stored content.

Stored `EntityDefinition` documents use a closed typed capability schema. The
current playable FPS slice recognizes transform, bounds/collision, controller,
health, weapon mount, render projection, policy binding, spawn marker, and
faction declarations. Rust validates those declarations, creates scene
entities once during canonical admission, binds rule-owned FPS state to those
same entity ids, and rejects the entire activation if domain topology is
incomplete or contradictory. `loadProject({ source })` therefore produces
immediate FPS, gameplay-module, trigger, collision, voxel, input, time-control,
and restart authority without a second handwritten bootstrap call.
The active-project readout publishes the installed domain and Rust-resolved
entity roles so render and UI projection consume the same classification used
by runtime authority.

The development directory and packaged archive are two transports for the same
ProjectBundle closure. They are not separate content pipelines. Given the same
manifest and bodies, both produce the same project, content-set, composition,
scene/entity/resource, provider, and active-authority identities. The active
identity also publishes Rust-selected voxel asset/grid bindings so collision
calls never depend on a downstream replica of the asset-to-grid map.

## Workspace manifest versus ProjectBundle manifest

An ASHA Game Project may also contain a workspace manifest such as
`asha.game.toml`, read through `@asha/game-workspace`. That file describes the
repository/tooling environment: compatibility pins, source roots, host commands,
and Studio attach information. It is not runtime content and is not packaged as
authority.

`asha.project-bundle.json` is the canonical stored-content manifest. It closes
over the scenes, ProjectContent, assets, and resource bodies admitted by Rust.
Runtime and authoring project-source loading follows this manifest only. A game
must not derive another list of source roles in boot code, and moving, adding,
splitting, or deleting a manifest artifact must not require a boot-code edit.

## Authoring and saving the same project

Editors use the same project source with `WorkspaceAuthoringFacade.openProject`.
That opens an independent Rust authoring cell, decodes all manifest scenes and
ProjectContent, and loads stored voxel assets for authoring projection without
starting a gameplay session. Typed commands update the Engine-owned working set.
Each ProjectContent envelope declares a stable `documentId`; the manifest path
is supplied separately as `sourcePath` and is retained in the validated working
set. Renaming a ProjectContent file therefore updates `sourcePath` through a
typed upsert without changing references to its stable identity. A document id
must never be inferred from, or rewritten to match, its filesystem path.

Saving is one revision-bound handshake:

1. The trusted server host calls `observeAshaProjectStore(projectRoot)`.
2. The authoring facade calls `prepareProjectWrite(...)` with that observation
   and any requested non-ProjectContent path relocations. ProjectContent paths
   already belong to the typed working set through `upsert.sourcePath`.
3. Rust derives the complete next manifest topology, canonical scene and
   ProjectContent bodies, content hashes, writes, moves, and deletes. The host
   cannot add an undeclared write or substitute a body.
4. `applyAshaProjectWriteCandidate(...)` stages the returned bytes, verifies the
   complete next store identity, atomically swaps the directory, and supplies
   the publication to `confirmProjectWrite(...)`.
5. Rust consumes the exact single-use candidate. A stale revision, changed host
   store, wrong publication, or replay rejects; failed confirmation rolls the
   host directory back.

The host owns filesystem mechanics. Rust owns what the new ProjectBundle means.
TypeScript owns typed edit expression, requested paths, and transport of opaque
Rust buffers; it does not hand-maintain the manifest, content hashes, or role
table. Asset-specific authoring transactions remain explicit where a working
asset has not yet joined the whole-project working set.

The committed walking consumer in
`harness/fixtures/canonical-project-consumer` is the focused boundary check. It
boots a multi-scene project with prefab/entity references, gameplay config, a
stored voxel house, collision, and visible projection; opens those same files
for authoring; performs add/move/split/delete; atomically saves; and fresh-loads
matching development-directory and packaged sources. Its output is an ordinary
consumer result, not a reusable proof-artifact format.

Handwritten bootstrap registries, caller-assembled prefab/spatial/trigger/
scheduler arrays, and caller-computed configuration bytes, hashes, runtime
entity numbers, or provider codecs are not public consumer surfaces. Engine
tests may use the generated raw batch to isolate source admission. Games must
use `loadProject({ source })`. Explicit `closeProject()` is required before a
future replacement load; the facade supplies lifecycle binding itself.
