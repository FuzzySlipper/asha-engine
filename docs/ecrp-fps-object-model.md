# FPS ECRP Object Model

Status: upstream public catalog surface for #4164.

`@asha/catalog-core` exposes `readFpsEcrpObjectModel()` as a typed map from
the generated-tunnel FPS loop to the current ECRP runtime surface. The readout is
catalog/projection metadata only. It does not own runtime authority and does not
introduce a framework ECS.

## Surface

The readout kind is:

```text
fps_ecrp_object_model_readout.v0
```

It includes:

- the `asha.generated_tunnel.fps_ecrp_object_model.v0` model id;
- player and enemy runtime roles;
- EntityDefinition stable ids and ProjectBundle source paths;
- CapabilityState kind lists aligned with `RuntimeSessionFacade.readEcrpRuntimeReadout`;
- Rule owner labels for bootstrap, lifecycle, movement, collision, combat,
  policy, nav, encounter, and render projection ownership;
- policy/event/projection references used by the current playable FPS loop;
- public runtime surfaces that consumers may call;
- stable hashes for model, player entry, enemy entry, and public surfaces.

## Current Entries

| Role | EntityDefinition | Core Capabilities | Public Surfaces |
| --- | --- | --- | --- |
| player | `actor/demo-player` | `transform`, `collisionBody`, `controller`, `health`, `weaponMount`, `renderProjection`, `faction` | ECRP readout, collision-constrained camera input, runtime action intent, camera projection, combat/lifecycle readouts, browser FPS input, renderer surface |
| enemy | `actor/generated-tunnel-enemy` | `transform`, `collisionBody`, `health`, `renderProjection`, `policyBinding`, `spawnMarker`, `faction` | ECRP readout, action intent, combat feedback, lifecycle, generated tunnel, autonomous policy tick, nav projection, renderer surface |

## Ownership

ASHA runtime authority owns runtime entity lifecycle, CapabilityState mutation,
collision resolution, combat damage application, policy validation, nav/path
projection, and render projection state.

Consumers such as `asha-demo` own browser input collection, HUD placement,
pointer-lock shell behavior, and render canvas mounting. They should use the
object model to select public ASHA surfaces and should not recreate entity,
collision, combat, policy, or lifecycle authority locally.

## Non-Claims

This is not runtime state, not demo-local authority, not an arbitrary JSON
payload hatch, and not a generic component framework. It is a public catalog map
for wiring product content to existing ASHA runtime surfaces.
