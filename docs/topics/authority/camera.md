---
status: current
audience: agent
tags: [camera, input, navigation, authority]
supersedes: []
see-also: [runtime-session-facade.md, named-input-actions.md]
---

# Camera

Camera authority is Rust-owned. Controllers are typed with expected-revision commands, terrain constraints, and renderer-only transitions.

## Camera Modes

First-person, orbit, and top-down controllers. `applyCameraModeCommand` validates expected revision and controller target. Accepted receipts expose the authoritative endpoint and optional renderer transition endpoints. Stale, invalid, incompatible, and terrain-blocked proposals reject atomically. See `docs/camera-modes.md`.

## Collision-Constrained Camera Input

`applyCollisionConstrainedCameraInput` applies first-person motion/look through the typed collision bridge surface. Grounded movement derives forward/right from yaw and rejects nonzero `moveUp`; free flight retains pitch-aware and vertical locomotion. Receipts echo the mode with collided, blocked axes, and world/collision projection hashes. See `docs/camera-modes.md`.

## First-Person Tunnel Viewport

The generated-tunnel FPS viewport uses collision-constrained camera input through the typed bridge surface. See `docs/first-person-tunnel-viewport.md`.
