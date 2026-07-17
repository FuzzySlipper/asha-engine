---
status: current
audience: agent
tags: [input, camera, navigation, authority]
supersedes: []
see-also: [runtime-session-facade.md, named-input-actions.md, camera-modes.md]
---

# Input and Camera

Input and camera authority are Rust-owned. Browser hosts normalize platform samples; Rust resolves named actions and owns camera state.

## Named Input Actions

Rust owns the named-input catalog, context stack, raw resolution, and platform-free semantic replay surface. Browser samples converge on typed RuntimeSession proposals. See `topics/authority/named-input-actions.md`.

## Camera Modes

First-person, orbit, and top-down controllers with expected-revision commands, terrain constraints, and renderer-only transitions. See `topics/authority/camera-modes.md`.

## First-Person Tunnel Viewport

The generated-tunnel FPS viewport uses collision-constrained camera input through the typed bridge surface. See `topics/authority/first-person-tunnel-viewport.md`.
