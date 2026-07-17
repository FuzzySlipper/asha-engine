---
status: current
audience: agent
tags: [ecrp, capability, ownership]
supersedes: []
see-also: []
---

# ECRP Capability Rule Ownership

Status: implementation note for #4162/#4190/#4224.

ASHA's ECRP model uses typed Capabilities, not generic ECS components. Runtime
authority still changes through explicit commands, validation, and accepted
events; Rules do not run through a hidden scheduler or ambient mutable bus.

The closed matrix is federated, not engine-monolithic. A statically composed
downstream Rust gameplay module may own mutation of its own typed module state
and may contribute a narrow decision owner for its own domain authority. It
cannot write an engine capability directly. Shared proposals still resolve to
one registry owner and route through that owner's existing Rule/service.

## Owner Matrix

`svc-entity-authoring` exposes `validate_and_apply_rule_owned` for Rule paths.
It checks a closed owner/mutation matrix before applying a command to
`EntityStore`.

| Rule owner | Allowed mutations |
|---|---|
| `EntityBootstrap` | lifecycle create plus initial `transform`, `bounds`, `render`, and `collision` capability attachment |
| `LifecycleRule` | lifecycle create/destroy/enable/disable/labels |
| `TransformRule` | set transform |
| `MovementRule` | move |
| `CollisionRule` | attach collision and bounds; activate/deactivate collision participation |
| `ControllerRule` | activate/deactivate an attached controller association |
| `RenderProjectionRule` | attach render projection |
| `RelationRule` | transform parent, containment, and source-ancestry relations |

Forbidden mutations return `RuleOwnedEntityAuthoringOutcome::Forbidden` with an
`EcrpRuleMutationDiagnostic`. The live store is not mutated.

The separate `apply_rule_owned_capability_activation` path returns generated
typed activation outcomes and applies the same closed ownership posture. See
`docs/capability-activation.md` for the absence/inactivity/lifecycle semantics.

## Current Boundary

The existing `validate_and_apply` function remains the operator/devtools
proposal path. Rule implementation should use `validate_and_apply_rule_owned`
so capability writes are explicit and reviewable.

TypeScript policies, renderer code, UI code, and downstream demos do not receive
raw `EntityStore` access. They propose commands or consume read-only projections
through public RuntimeSession surfaces.

Gameplay modules receive the same boundary at a more expressive Rust altitude:
typed event payloads, authored configuration, declared frozen views or bounded
owner-query receipts, and proposal/module-fact helpers. The closed gameplay
registry proves module, provider, read, output, state-owner, and shared-owner
identity before activation.

Named input contexts are deliberately outside this Entity owner matrix.
`rule-input` validates the generated `protocol-input` catalog and owns the
RuntimeSession context stack, priority/consumption decision, snapshots, and
resolution hashes. Browser hosts normalize platform samples; entities do not
carry key state or input-context CapabilityState. See
[`named-input-actions.md`](named-input-actions.md).

## FPS RuntimeSession Authority Slice

The current generated-tunnel FPS loop has a narrow Rust-owned RuntimeSession
authority slice in `rule-lifecycle`, composed over `svc-entity-authoring` and
`svc-combat`:

- `load_fps_project_bundle()` bootstraps ProjectBundle-shaped
  EntityDefinitions through `svc-entity-authoring` before seeding FPS role,
  health, weapon, policy-binding, and render-projection runtime state.
- Health CapabilityState is seeded from loaded definitions and then updated by
  accepted primary-fire proposals through `svc-combat::apply_fire_intent()`.
- Enemy death updates lifecycle status, recent entity events, and render
  projection visibility; the defeated entity is disabled and its render
  projection is made invisible through the owning Rules.
- WeaponMount, Controller, PolicyBinding, SpawnMarker, and Faction are loaded
  as typed CapabilityState/source refs and remain read-only in this slice unless
  routed through their specific RuntimeSession methods.

`@asha/runtime-bridge` exposes this authority through the public
`RuntimeSessionFacade` product/live path when a Rust-capable bridge is selected.
Reference/mock RuntimeSession helpers remain available only from
`@asha/runtime-bridge/reference` for tests, compatibility fixtures, and offline
smoke baselines. Reference receipts are labelled as fixture/reference evidence;
Rust-backed receipts carry `rust_bridge` / `native_rust` provenance and named
authority surfaces such as `runtime_session.fps.primary_fire.v0`.

## Non-Claims

This note does not introduce a scheduler, generic component registry, ambient
event bus, or framework ECS. The Rust `svc-entity-authoring` owner matrix still covers the
generic entity/transform/render/collision/relation substrate. Some reference
readouts remain compatibility fixtures, and any still-unwired Rust operation must
fail closed rather than silently falling back to reference behavior.
