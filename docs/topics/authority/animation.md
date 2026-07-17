---
status: current
audience: agent
tags: [animation, authority, projection, render]
supersedes: []
see-also: [render-protocol.md, renderer-host-animated-mesh.md]
---

# Animation

Animation authority is Rust-owned through the animation controller. Projection emits render diffs for animated meshes.

## Animation Controller Authority

The animation controller owns skeletal/state machine animation authority. See `docs/animation-controller-authority.md`.

## Animation Controller Projection

The projection emits typed render diffs for animated mesh instances. See `docs/animation-controller-projection.md`.

## Animation Timing Semantics

Animation timing is deterministic and replay-aware. See `docs/animation-timing-semantics.md`.
