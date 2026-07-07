# Downstream Repo Roles Map

## Purpose

Route questions about `asha-demo`, `asha-studio`, and `asha-testing` without
moving downstream planning truth into this engine repository.

## Owns

- Engine-side public package policy for downstream roles.
- Repo-family orientation and compatibility expectations.
- The distinction between human-facing demo, editor/product tooling, and
  synthetic conformance consumer.

## Does Not Own

- Current downstream task queues.
- Downstream product UX implementation.
- Private imports into engine internals.

## Primary Paths

- [repo-family-deployment.md](../repo-family-deployment.md)
- [consumer-compatibility.md](../consumer-compatibility.md)
- [game-agent-code-organization.md](../game-agent-code-organization.md)
- [harness/public-surface/ts-packages.json](../../harness/public-surface/ts-packages.json)
- [pack-public-artifacts.mjs](../../ts/scripts/pack-public-artifacts.mjs)

## Public Downstream Surfaces

- `asha-demo`: human-facing ASHA Game Project; uses approved package roots and
  engine render/runtime facades.
- `asha-studio`: editor/product tooling; uses approved package roots for
  authoring and live inspection.
- `asha-testing`: synthetic proof/conformance consumer; may consume testing
  surfaces approved by the public-surface manifest.
- Public package tarballs and their generated manifest are emitted under
  `ts/artifacts/public-packages/`; that directory is a generated local output,
  not a checked-in Atlas link target.

## Private Or Forbidden Paths

- Downstream repos must not import [engine-rs/crates](../../engine-rs/crates).
- Downstream repos must not import `@asha/*/src/*` or generated contract file
  paths.
- `asha-demo` must not own upstream engine machinery such as collision,
  pathfinding, RuntimeSession internals, or renderer backend internals.

## Proof Gates And Goldens

- [harness/public-surface/check-public-boundary.py](../../harness/public-surface/check-public-boundary.py)
- [harness/fixtures/smoke](../../harness/fixtures/smoke)
- [harness/smoke-out](../../harness/smoke-out)
- [docs/consumer-compatibility.md](../consumer-compatibility.md)

## Common Agent Mistakes

- Treating `asha-testing` proof harness work as demo product work.
- Solving a missing public engine API by importing private engine files.
- Updating engine docs as if they were current downstream task state.

## Follow-up Routing

- Missing public package root or compatibility marker: create an `asha-engine`
  public-surface task.
- Demo product behavior: assign to the `asha-demo` agent after the engine surface
  exists.
- Studio UX/editor behavior: assign to the Studio agent; keep engine work limited
  to public surfaces and authority substrate.
