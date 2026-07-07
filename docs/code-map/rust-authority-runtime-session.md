# Rust Authority And RuntimeSession Map

## Purpose

Route work that validates, applies, replays, or reads authoritative runtime
state. Rust owns accepted truth; TypeScript proposes commands and displays
projections.

## Owns

- RuntimeSession authority and readouts through Rust services/rules.
- Command validation, accepted state mutation, deterministic replay, and hashes.
- FPS lifecycle, combat, nav, camera, game-rules, voxel, and world-bundle
  authority paths when they are reusable engine substrate.

## Does Not Own

- Browser UI layout, Studio panels, or demo product composition.
- TypeScript policy behavior beyond validating proposals at the generated
  boundary.
- Renderer implementation details or Three.js objects.

## Primary Paths

- [engine-rs/crates/state](../../engine-rs/crates/state)
- [engine-rs/crates/services](../../engine-rs/crates/services)
- [engine-rs/crates/rules](../../engine-rs/crates/rules)
- [engine-rs/crates/sim](../../engine-rs/crates/sim)
- [runtime-session-facade.md](../runtime-session-facade.md)
- [ecrp-runtime-session-readout.md](../ecrp-runtime-session-readout.md)

## Public Downstream Surfaces

- `@asha/runtime-session` and `@asha/runtime-bridge` package roots.
- Generated protocol DTOs from `@asha/contracts`.
- Runtime bridge operations declared in
  [bridge-manifest.toml](../../engine-rs/crates/bridge/runtime-bridge-api/bridge-manifest.toml).

## Private Or Forbidden Paths

- Downstream repos must not import [engine-rs/crates](../../engine-rs/crates).
- TypeScript must not mutate SessionState or capability state directly.
- RuntimeSession must not expose raw stores, ad hoc JSON tunnels, or native
  transport handles as consumer API.

## Proof Gates And Goldens

- [check-rust.sh](../../harness/ci/check-rust.sh)
- [check-bridge.sh](../../harness/ci/check-bridge.sh)
- [check-replays.sh](../../harness/ci/check-replays.sh)
- [harness/fixtures/game-rules](../../harness/fixtures/game-rules)
- [harness/fixtures/combat](../../harness/fixtures/combat)
- [harness/fixtures/nav](../../harness/fixtures/nav)

## Common Agent Mistakes

- Adding a TypeScript-side shortcut because the Rust facade is missing.
- Treating a readout or telemetry object as authority.
- Duplicating health, collision, replay, or lifecycle mutation in a consumer
  repo instead of opening the upstream Rust surface.

## Follow-up Routing

- Missing or too-narrow RuntimeSession behavior: create an `asha-engine` task
  tagged `runtime-session`, `rust-rule`, or `rust-service`.
- Consumer ergonomics only: route to the downstream repo after the public
  engine surface exists.
- Cross-lane dependency changes: update [ownership.toml](../../governance/ownership.toml)
  and run depgraph checks.
