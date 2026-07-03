# Generated Tunnel Runtime Readout

Status: task #4050 public RuntimeSession readout for the #4038 tunnel fixture.

Public import path:

```ts
import {
  createMockRuntimeSession,
  TINY_GENERATED_TUNNEL_READOUT,
  type GeneratedTunnelReadout,
} from '@asha/runtime-bridge';
```

`RuntimeSessionFacade.readGeneratedTunnelReadout()` exposes the committed
`tiny-enclosed` generated tunnel evidence:

- seed `17`
- generator `asha.tunnel.enclosed.v1` version `1`
- config hash `e1d156c6b55137a7`
- output hash `a9b504096397f5b4`
- replay hash `fnv1a64:0821a0c2aea17dff`
- render projection hash `fnv1a64:21eb8696f6f3b5c4`
- collision projection hash `fnv1a64:78b242163cf67524`
- spawn markers `player_start` and `exit_hint`

`RuntimeSessionFacade.requestGeneratedTunnelOperation()` is the typed fail-closed
path for unsupported operations. `regenerate` and `apply_to_runtime_world` return
`status: 'unsupported'` with `reason: 'generated_tunnel_operation_not_wired'`.

Non-claims:

- No TypeScript generation algorithm.
- No demo-local generation, collision, or render authority.
- No runtime world mutation from this readout.
- No generic JSON action tunnel.
