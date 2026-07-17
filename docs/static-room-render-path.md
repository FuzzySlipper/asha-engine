---
status: current
audience: agent
tags: [render, static-room, projection]
supersedes: []
see-also: []
---

# Static Room Render Path

Status: task #4029 upstream evidence for the first `asha-demo` visual path.

Public package roots:

- `@asha/render-projection`
- `@asha/renderer-three`

Entry points:

- `createStaticRoomRenderFrame()` returns the synthetic static-room `RenderFrameDiff`.
- `renderProjectedFrame(frame)` applies the frame through `RenderProjection` and the retained `ThreeRenderer`, then returns the projection, renderer, and deterministic structural snapshot.
- `ThreeRenderer` remains the implementation binding over render diffs. It does not read authority state or raw runtime transports.

Evidence:

- Fixture: `harness/fixtures/render-diffs/static-room.json`
- Structural render golden: `harness/goldens/render-diffs/static-room.snapshot`
- Tests: `ts/packages/renderer-three/src/static-room.test.ts` and `ts/packages/renderer-three/src/golden.test.ts`

Non-claims:

- No playable game loop.
- No runtime/native attachment.
- No motion, collision, or physics evidence.
- No authority mutation from TypeScript.
- No browser screenshot or pixel golden; the current renderer gate is a deterministic GL-free structural snapshot.
