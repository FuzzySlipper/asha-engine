---
status: current
audience: agent
tags: [ecrp, capability, entity, authority, rust]
supersedes: []
see-also: [runtime-session-facade.md, ecrp-runtime-session-readout.md, prefab-contracts.md]
---

# ECRP: Entity, Capability, Relationship, Prefab

ECRP is ASHA's content and runtime object model. Entities are typed IDs with attached capabilities. Relationships are explicit typed references. Prefabs are reusable part compositions with stable roles.

## Capability Activation

Typed collision/controller activation with lifecycle interaction, persistence, and owner gates. See `docs/capability-activation.md`.

## Capability / Rule Ownership Matrix

The rule-owner matrix maps ECRP capabilities to their owning Rust rule crates. See `docs/ecrp-capability-rule-ownership.md`.

## FPS Object Model

The generated-tunnel loop maps roles to ECRP capabilities and runtime surfaces. See `docs/ecrp-fps-object-model.md`.

## Entity Definition Schema

Stored capability defaults used when authority creates an Entity. See `docs/entity-definition-schema.md`.

## ECRP RuntimeSession Readout

`RuntimeSessionFacade.readEcrpRuntimeReadout()` exposes live Entity/CapabilityState/event readouts. `loadEcrpProject()` bootstraps ProjectBundle-shaped ECRP content. See `docs/ecrp-runtime-session-readout.md`.
