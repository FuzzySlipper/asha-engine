# Render Projection And Renderer Host Map

## Purpose

Route work where Rust produces renderer-neutral render diffs and TypeScript
renderer packages display those projections without owning authority.

## Owns

- Rust render-diff protocol and render projection generation.
- Renderer-neutral retained projection model.
- Browser renderer host facade for demos and product surfaces.
- Concrete Three.js backend isolation behind engine-owned packages.

## Does Not Own

- Runtime SessionState or gameplay authority.
- Consumer direct access to Three.js internals.
- Stored scene truth or asset catalog authority.

## Primary Paths

- [engine-rs/crates/render](../../engine-rs/crates/render)
- [engine-rs/crates/protocol/protocol-render](../../engine-rs/crates/protocol/protocol-render)
- [ts/packages/render-projection](../../ts/packages/render-projection)
- [ts/packages/renderer-host](../../ts/packages/renderer-host)
- [ts/packages/renderer-three](../../ts/packages/renderer-three)
- [render-protocol.md](../render-protocol.md)
- [static-room-render-path.md](../static-room-render-path.md)

## Public Downstream Surfaces

- `@asha/render-projection` for renderer-neutral retained projection state.
- `@asha/renderer-host` for demo/browser render surface hosting.
- `@asha/renderer-three` only for approved testing/backend use, not direct demo
  application coupling.

## Private Or Forbidden Paths

- Downstream demo code must not import Three.js directly as an engine bypass.
- Renderers must not mutate Rust authority or invent gameplay state.
- Rust render crates must not render directly or own UI/product decisions.

## Proof Gates And Goldens

- [check-render-goldens.sh](../../harness/ci/check-render-goldens.sh)
- [harness/fixtures/render-diffs](../../harness/fixtures/render-diffs)
- [harness/goldens/render-diffs](../../harness/goldens/render-diffs)
- [harness/goldens/render-projection](../../harness/goldens/render-projection)

## Common Agent Mistakes

- Fixing a missing render surface by adding Three.js code in a downstream repo.
- Treating render handles as Entity or Capability authority.
- Updating a render DTO without regenerating contracts and goldens.

## Follow-up Routing

- Missing renderer-neutral data: route to Rust render/protocol crates.
- Browser surface ergonomics: route to `@asha/renderer-host`.
- Concrete backend behavior: route to `@asha/renderer-three` while preserving
  facade boundaries.
