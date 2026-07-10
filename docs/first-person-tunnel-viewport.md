# First-Person Tunnel Viewport Adapter

Status: task #4067 upstream renderer-three projection adapter.

Public package root:

```ts
import {
  createGeneratedTunnelViewportFrame,
  renderFirstPersonTunnelViewport,
  summarizeFirstPersonTunnelViewport,
} from '@asha/renderer-three';
```

The adapter consumes public ASHA readouts only:

- `GeneratedTunnelReadout` from `@asha/runtime-bridge`
- `CameraProjectionSnapshot` from the RuntimeSession camera projection path
- optional collision debug hashes from RuntimeSession collision receipts

`createGeneratedTunnelViewportFrame()` creates a deterministic `RenderFrameDiff`
for the generated tunnel shell and spawn markers. `renderFirstPersonTunnelViewport()`
applies that frame through `RenderProjection` and the retained `ThreeRenderer`,
then returns a structural snapshot plus a `first_person_tunnel_viewport.v0`
summary.

Pinned evidence in the current fixture:

- fixture name: `generated-tunnel-first-person-viewport`
- generated tunnel output hash: `a9b504096397f5b4`
- generated tunnel render projection hash: `fnv1a64:21eb8696f6f3b5c4`
- generated tunnel collision projection hash: `fnv1a64:b2312fbcfb060db3`
- viewport frame hash: `fnv1a64:db081afd570c2f30`
- viewport structural hash: `fnv1a64:35ad3bca1a9f1667`

Evidence:

- Tests: `ts/packages/renderer-three/src/tunnel-viewport.test.ts`
- Existing generated tunnel fixture: `harness/fixtures/generated-levels/tiny-tunnel.snapshot.txt`

Non-claims:

- No runtime authority or state mutation.
- No collision authority.
- No local level generation.
- No browser pixel golden; the current proof is deterministic and GL-free.
