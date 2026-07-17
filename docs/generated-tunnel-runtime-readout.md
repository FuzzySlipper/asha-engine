---
status: current
audience: agent
tags: [tunnel, levelgen, readout, runtime]
supersedes: []
see-also: []
---

# Generated Tunnel Runtime Readout

Status: task #4050 public RuntimeSession readout for the #4038 tunnel fixture.

Public import path:

```ts
import {
  TINY_GENERATED_TUNNEL_READOUT,
  type GeneratedTunnelReadout,
} from '@asha/runtime-session';
import { createMockRuntimeSession } from '@asha/runtime-bridge/reference';
```

`RuntimeSessionFacade.readGeneratedTunnelReadout()` exposes the committed
`tiny-enclosed` generated tunnel evidence:

- seed `17`
- generator `asha.tunnel.enclosed.v2` version `2`
- config hash `e1d156c6b55137a7`
- output hash `1471496d88d70647`
- replay hash `fnv1a64:0821a0c2aea17dff`
- render projection hash `fnv1a64:21eb8696f6f3b5c4`
- collision projection hash `fnv1a64:627389be013a3154`

The generator publishes one runtime frame for collision, rendering, and authored
spawn correspondence:

- canonical voxel-world offset `[-3.5, -1, -5.5]`
- playable minimum `[-2.5, 0, -4.5]`
- playable maximum `[2.5, 4, 4.5]`

The advertised `5 x 4 x 9` dimensions are collision-free corridor dimensions.
Rust generates a one-voxel shell around that playable volume. The collision
projection identity includes the generator-owned offset, while renderer-neutral
projection consumes the same playable bounds and transforms canonical spawn
markers through the same offset.
- spawn markers `player_start` and `exit_hint`

On a Rust-backed session, call
`requestGeneratedTunnelOperation({ operation: 'apply_to_runtime_world', presetId: 'tiny-enclosed', seed: 17 })`
after loading the ECRP project. Rust regenerates the same `svc-levelgen` output
and atomically installs its voxel world as collision authority. The `applied`
receipt exposes the authoritative grid plus config, output, collision-source,
runtime collision-projection hashes, and runtime frame; consumers pass that grid to
`applyCollisionConstrainedCameraInput` instead of hardcoding it. `regenerate`
remains an unsupported authoring operation, and reference sessions do not claim
runtime collision authority. Collision-constrained camera movement uses a
continuous axis sweep so a command cannot tunnel through intervening voxels.
Each command is limited to 256 world units of travel per axis; larger proposals
fail closed as invalid input without mutating the camera.

Non-claims:

- No TypeScript generation algorithm.
- No demo-local generation, collision, or render authority.
- The readout itself does not mutate the runtime world; only the typed apply
  operation does.
- No generic JSON action tunnel.
