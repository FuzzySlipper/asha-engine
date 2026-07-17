---
status: current
audience: agent
tags: [adr, ui, editor]
supersedes: []
see-also: []
---

# ADR 0008 — Voxel UI / editor architecture

**Status:** Accepted

Voxel editing UI has three state categories: **authoritative** (Rust voxel/spatial session state — UI never
mutates), **transient DOM/render** (hover, drag — throwaway, component-local), and **persistent
editor tool context** (current tool/brush/size/material/snapping/selection/preview — durable TS
state, devtools-inspectable, *not* a shadow of authority).

The persistent editor tool context gets a dedicated `@asha/editor-tools` package: a small,
dependency-free observable store (state + pure-reducer actions + subscribe), importing
`@asha/contracts` only — no DOM, `three`, policy, or bridge. It produces protocol-typed
`VoxelCommand` *proposals*; it does not submit them.

`ui-dom` (panels/inspectors) and `devtools` (read-only) import `editor-tools`. `app` is the **only**
package that submits commands — `editor-tools` proposal → `@asha/runtime-bridge.submitCommands` →
Rust validates/applies. Preview is a non-authoritative `debug`-layer overlay, visually distinct,
mutating nothing. Inspectors read projections + editor diagnostics, never a hidden state copy.
Camera collision uses the shared `svc-collision` query service, not a UI-only system.

No full UI implementation lands here (design only); package boundaries + import rules are
forward-declared in `ownership.toml`/`dependency-policy.toml`. Full design + test plan:
`docs/voxel-ui-architecture.md`.
