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
- generator `asha.tunnel.enclosed.v1` version `1`
- config hash `e1d156c6b55137a7`
- output hash `a9b504096397f5b4`
- replay hash `fnv1a64:0821a0c2aea17dff`
- render projection hash `fnv1a64:21eb8696f6f3b5c4`
- collision projection hash `fnv1a64:b2312fbcfb060db3`

The collision projection identity includes the generator-owned centered runtime
room offset, so camera, picking, and combat queries address the same coordinates
as the first-person room projection while canonical voxel coordinates stay intact.
- spawn markers `player_start` and `exit_hint`

On a Rust-backed session, call
`requestGeneratedTunnelOperation({ operation: 'apply_to_runtime_world', presetId: 'tiny-enclosed', seed: 17 })`
after loading the ECRP project. Rust regenerates the same `svc-levelgen` output
and atomically installs its voxel world as collision authority. The `applied`
receipt exposes the authoritative grid plus config, output, collision-source,
and runtime collision-projection hashes; consumers pass that grid to
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
