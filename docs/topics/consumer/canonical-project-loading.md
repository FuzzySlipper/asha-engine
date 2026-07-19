---
status: current
audience: consumer
tags: [runtime-session, project-bundle, loading, downstream]
supersedes: []
see-also: [../authority/runtime-session-facade.md, ../bridge/runtime-session-static-composition.md]
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

The following are compatibility or adapter-internal surfaces, not ordinary
consumer APIs:

- `loadEcrpProject` and its handwritten bootstrap registry;
- `GameplayRuntimeProjectInput` and explicit prefab/spatial/trigger/scheduler arrays;
- `loadProjectBundle` and the low-level source resource/batch verbs;
- caller-computed canonical configuration bytes, hashes, runtime entity numbers, or provider codecs.

Engine tests may use the generated raw batch to isolate source admission. Games
must use `loadProject({ source })`. Explicit `closeProject()` is required before
a future replacement load; the facade supplies lifecycle binding itself.
