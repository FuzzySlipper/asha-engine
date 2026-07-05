# ECRP Capability Rule Ownership

Status: implementation note for #4162/#4190.

ASHA's ECRP model uses typed Capabilities, not generic ECS components. Runtime
authority still changes through explicit commands, validation, and accepted
events; Rules do not run through a hidden scheduler or generic event bus.

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
| `CollisionRule` | attach collision and bounds |
| `RenderProjectionRule` | attach render projection |
| `RelationRule` | transform parent, containment, and source-ancestry relations |

Forbidden mutations return `RuleOwnedEntityAuthoringOutcome::Forbidden` with an
`EcrpRuleMutationDiagnostic`. The live store is not mutated.

## Current Boundary

The existing `validate_and_apply` function remains the operator/devtools
proposal path. Rule implementation should use `validate_and_apply_rule_owned`
so capability writes are explicit and reviewable.

TypeScript policies, renderer code, UI code, and downstream demos do not receive
raw `EntityStore` access. They propose commands or consume read-only projections
through public RuntimeSession surfaces.

## FPS RuntimeSession Authority Slice

The current generated-tunnel FPS loop has a reference RuntimeSession authority
slice in `@asha/runtime-bridge`:

- `loadEcrpProject()` validates ProjectBundle-shaped EntityDefinitions and
  SceneDocument placements before replacing live ECRP project state.
- Health CapabilityState is derived from loaded EntityDefinitions and then
  updated by accepted `runtime_action_intent.v0` primary-fire proposals.
- Enemy death updates lifecycle status, recent entity events, and render
  projection visibility in `readEcrpRuntimeReadout()`.
- WeaponMount, Controller, PolicyBinding, SpawnMarker, and Faction are loaded
  as typed CapabilityState/source refs and remain read-only in this slice unless
  routed through their specific RuntimeSession methods.

This is not a generic ECS or browser-side authority store. It is the current
public RuntimeSession reference authority for the human-facing FPS demo while
the compiled Rust protocol grows the corresponding native capability records.

## Non-Claims

This note does not introduce a scheduler, generic component registry, or
framework ECS. The Rust `svc-entity-authoring` owner matrix still covers the
generic entity/transform/render/collision/relation substrate; the FPS-specific
RuntimeSession authority slice should be promoted into native protocol/state
lanes through narrow follow-up work rather than by exposing raw store access.
