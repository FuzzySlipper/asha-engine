# Navigation Runtime Readout

Status: task #4052 public RuntimeSession and PolicyView nav/pathfinding readout.

Public import path:

```ts
import {
  GENERATED_TUNNEL_NAV_POLICY_VIEW,
  GENERATED_TUNNEL_REACHABLE_PATH,
  createMockRuntimeSession,
  type NavPathReadout,
} from '@asha/runtime-bridge';
```

`RuntimeSessionFacade.readNavProjection()` exposes the #4041 generated-tunnel
navigation projection:

- projection id `generated_tunnel_nav_projection`
- walkable cells `66`
- projection hash `d1f6ac3e051d6b6e`
- fixture `harness/fixtures/nav/generated-tunnel-path.snapshot.txt`

`RuntimeSessionFacade.queryNavPath()` exposes two typed readouts:

- `generated_tunnel_reachable`: reached, visited `21`, path length `9`, path hash
  `e8e1ea7a09811ced`
- `generated_tunnel_no_path`: no path, rejection reason `blocked`, empty path hash
  `a8c7f832281a39c5`

`RuntimeSessionFacade.readNavPolicyView()` returns a read-only/proposal-only view
shape for future policy fixtures. It exposes projection and latest path evidence
only; it has no movement, mutation, or apply-path method.

Non-claims:

- No enemy AI or policy behavior.
- No movement authority.
- No demo wiring.
- No mutation through PolicyView.
