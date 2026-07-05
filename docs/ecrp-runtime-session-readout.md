# ECRP RuntimeSession Readout

Status: public readout plus ProjectBundle-shaped load surface for #4163/#4189.

`@asha/runtime-bridge` exposes `RuntimeSessionFacade.readEcrpRuntimeReadout()`
as the public, read-only ECRP inspection surface for consumers such as
`asha-demo`, Studio live inspection, and compatibility tests.

`RuntimeSessionFacade.loadEcrpProject()` is the public bootstrap/load surface
for ProjectBundle-shaped ECRP content. It accepts:

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
consumers continue to boot. Consumers can then call `loadEcrpProject()` to load
their ProjectBundle/EntityDefinition/SceneDocument content. After a successful
load, `readEcrpRuntimeReadout()` derives Entity ids, stable ids, source paths,
CapabilityState, health, render visibility, recent events, and hashes from the
loaded runtime project state.

Accepted primary-fire runtime action updates the loaded enemy lifecycle/health
state, render visibility, recent event list, and readout hashes. The combat
fixture still supplies the narrow primary-fire outcome in the reference bridge,
but the state mutation is applied to the loaded enemy entity id.

## Non-Claims

This surface does not expose raw `EntityStore`, does not edit EntityDefinitions,
and does not replace Studio Definition Authoring Mode. It is a live runtime
inspection/control projection only.

The reference TypeScript facade is still the browser/mock public RuntimeSession
surface. The Rust `svc-entity-authoring` ProjectBundle bootstrap substrate
exists separately; a future native/protocol integration can route this public
load surface through the compiled runtime without changing downstream demo code.
