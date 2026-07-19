---
status: current
audience: consumer
tags: [ecrp, runtime, readout, consumer]
supersedes: []
see-also: []
---

# ECRP RuntimeSession Readout

Status: public readout over canonical ProjectBundle admission. The handwritten
ECRP load input remains compatibility-only while `asha-demo` migrates.

`@asha/runtime-bridge` exposes `RuntimeSessionFacade.readEcrpRuntimeReadout()`
as the public, read-only ECRP inspection surface for consumers such as
`asha-demo`, Studio live inspection, and compatibility tests.

Ordinary consumers call `RuntimeSessionFacade.loadProject({ source })`. Rust
discovers EntityDefinitions, scenes, prefab instances, gameplay configuration,
and resources from the canonical manifest closure and returns the active
authority identities. Consumers do not assign runtime entity ids or submit a
parallel bootstrap registry.

`RuntimeSessionFacade.loadEcrpProject()` is the compatibility bootstrap surface
for the older handwritten ECRP input. It accepts:

- `ProjectBundle` identity plus the current compatibility `runtimeRequest`;
- `EntityDefinition[]`;
- `SceneDocument` placements with optional deterministic runtime entity ids.

It returns `runtime_session.ecrp_project_load_receipt.v0` with accepted/rejected
status, typed diagnostics, entity count, bootstrap hash, and before/after
session hashes. Invalid input is fail-closed and does not replace the live ECRP
project state.

## Surface

The readout kind is:

```text
runtime_session.ecrp_readout.v0
```

It includes:

- RuntimeSession sequence/tick/session hash;
- ASHA Game Project identity and current ProjectBundle compatibility request;
- live Entity summaries;
- attached typed CapabilityState summaries;
- renderProjection target identity metadata for binding runtime Entities to
  renderer-neutral visual targets;
- EntityDefinition/source traces;
- recent entity events;
- deterministic entity/capability/event hashes;
- non-claims that the readout is not raw StateStore access, authoring mode, or
  demo-local authority.

## Initial Capability Kinds

The first public readout covers the reference FPS loop shape:

- `transform`
- `collisionBody`
- `controller`
- `health`
- `weaponMount`
- `renderProjection`
- `policyBinding`
- `spawnMarker`
- `faction`

These are typed readout DTOs, not arbitrary JSON state bags. Consumers should
read them as projections of runtime authority and submit typed intents/commands
for changes.

## Current Behavior

The reference RuntimeSession starts with a compatibility ECRP project so older
fixtures continue to boot. Product consumers load canonical project sources;
compatibility fixtures may still call `loadEcrpProject()` until Demo migration.
After a successful load, `readEcrpRuntimeReadout()` derives Entity ids, stable ids, source paths,
CapabilityState, health, render visibility, recent events, and hashes from the
loaded runtime project state.

Accepted primary-fire runtime action updates the loaded enemy lifecycle/health
state, render visibility, recent event list, and readout hashes. The Rust
`rule-lifecycle` crate now owns the narrow FPS authority composition for this
path: ProjectBundle EntityDefinitions bootstrap through `svc-entity-authoring`,
health and hit resolution apply through `svc-combat`, and defeat drives lifecycle
plus render-projection state. The action receipt's combat readout is derived from
the same loaded player/enemy state: the target entity id, health
before/after/max, damage amount, event entity ids, fixture marker, health hash,
and replay hash agree with the ECRP readout. The older generated-tunnel fixture
remains available through `readCombatReadout()` for committed
golden/compatibility evidence, but it is no longer the source of truth for
loaded-project primary-fire receipts.

Each `renderProjection` CapabilityState now carries a
`runtime_session.ecrp_render_target.v0` target object. The target binds runtime
Entity id, EntityDefinition stable id, source path, inferred runtime role,
projection kind, render label, current transform, optional visual scale, and a
deterministic target hash. `renderHandle` is `null` until a concrete render-frame
owner assigns retained renderer handles; consumers should use `renderLabel` and
target identity rather than hard-coded demo label guesses.

For playable demo HUDs, `readRuntimeSessionPlayableLoopState()` derives
current-epoch counters and command gating from this ECRP readout, lifecycle, and
telemetry. It is a read-only projection helper: combat, lifecycle, restart, and
render visibility remain RuntimeSession authority, while consumers avoid local
shot/hit/restart counters that can drift across reset epochs.

## Non-Claims

This surface does not expose raw `EntityStore`, does not edit EntityDefinitions,
and does not replace Studio Definition Authoring Mode. It is a live runtime
inspection/control projection only.

The reference TypeScript facade is a labelled fixture surface, not product
authority. Product and downstream-composed sessions use the native Rust bridge;
removing the remaining Demo compatibility load is a downstream migration, not a
future engine integration requirement.
