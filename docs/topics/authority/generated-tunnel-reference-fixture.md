---
status: current
audience: agent
tags: [tunnel, levelgen, readout, runtime]
supersedes: []
see-also: []
---

# Generated Tunnel Reference Fixture

Status: bounded reference data for focused combat, navigation, and projection
tests. It is not a runtime loading or generation surface.

Public import path:

```ts
import {
  TINY_GENERATED_TUNNEL_READOUT,
  type GeneratedTunnelReadout,
} from '@asha/runtime-session';
```

`TINY_GENERATED_TUNNEL_READOUT` exposes committed `tiny-enclosed` fixture data:

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

Authored projects use the Rust workspace transaction in
`docs/procedural-environment-authoring.md` to materialize and save a canonical
scene plus local-space voxel asset. A fresh RuntimeSession then consumes those
saved artifacts without a generator registry. Reference fixture data cannot be
applied to a live RuntimeSession.

Non-claims:

- No TypeScript generation algorithm.
- No demo-local generation, collision, or render authority.
- The fixture does not mutate the runtime world.
- No generic JSON action tunnel.
