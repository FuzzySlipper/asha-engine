# Asha Engine — Unity Gap Analysis (2026-07-10; reconciled 2026-07-13)

What Unity provides as fundamental infrastructure that Asha Engine currently
lacks or only partially covers. Scoped to what matters for OSHApunk: a voxel
factory builder + Rimworld-style simulation with an FPS exploration mode.
**Not** scoped to Unity-specific features like lightmap baking, Mecanim,
terrain heightmaps, asset store integration, or multi-vendor platform support.

Assessment basis: the committed ASHA engine and Studio repositories, reconciled
after Batteries-for-Agents Wave 1 and the #5746 architecture-refinement
campaign. Structural counts belong to the generated code atlas rather than
this narrative snapshot.

---

## Summary Table

| # | Subsystem | Status | Priority for OSHApunk |
|---|---|---|---|
| 1 | Scene graph / entity hierarchy | **EXISTS** | — |
| 2 | Animation | **PARTIAL** | Medium |
| 3 | Audio | **BASELINE IMPLEMENTED** | Medium |
| 4 | Input handling | **BASELINE IMPLEMENTED** | Medium |
| 5 | Physics (dynamics) | **STUB** | Medium |
| 6 | Navigation / pathfinding | **EXISTS** | — |
| 7 | Particle systems / VFX | **BASELINE IMPLEMENTED** | Medium |
| 8 | World-space UI | **BASELINE IMPLEMENTED** | High |
| 9 | Prefabs / templates | **SUBSTANTIAL WAVE 1** | High |
| 10 | Serialization / persistence | **EXISTS** | — |
| 11 | Resource / asset management | **EXISTS** | — |
| 12 | Scripting / hot-reload | **MISSING** | Low |
| 13 | Networking / multiplayer | **MISSING** | Deferred |
| 14 | Profiling / stats | **PARTIAL** | Low-Medium |
| 15 | Coroutines / scheduling | **PARTIAL** | Medium |
| 16 | Camera system | **PARTIAL** | Medium |
| 17 | Material system | **FEEDBACK SLICE IMPLEMENTED** | Medium |
| 18 | LOD system | **MISSING** | Low-Medium |
| 19 | Gameplay fabric | **IMPLEMENTED (Wave 1 static)** | Medium |
| 20 | Component lifecycle | **PARTIAL** | Medium |

---

## Detailed Analysis

### 1. Scene Graph / Entity Hierarchy — EXISTS ✅

**What it has:**
- `core-entity`: entity identity, lifecycle (Active/Disabled/Destroyed),
  source provenance (Scene/Runtime/Generated), typed capability tables
  (TransformCapability, RenderProjectionCapability, CollisionCapability,
  etc.)
- `core-scene`: `FlatSceneDocument` with `SceneNodeRecord` (parent-child via
  `parent: Option<SceneNodeId>`, child ordering)
- Render bridge projects scene graph into retained render diffs
- Scene objects have explicit hierarchy commands (create/delete/rename/reparent)

**What's solid:** The entity/capability separation is well-designed. Entities
don't carry position — they acquire capabilities. The flat document model with
parent-child is clean. Render handles are derived projections, never persisted.

**Gap:** No runtime hierarchy traversal API exposed to policy/scripting. No
"find child by name" or "get components on parent." This is fine for now but
will matter when game logic needs to reason about entity relationships at
runtime.

### 2. Animation — PARTIAL ⚠️

**What it has:**
- `renderer-host/animated-mesh-host.ts`: GLB skeletal mesh loading with named
  clips (`idle`, `run`, `jump`)
- a Rust `rule-animation-controller` authority over versioned graphs, float,
  bool, and trigger parameters, deterministic transitions, fixed-point blend
  weights, playback speed, snapshots, and verification replay;
- states with either one clip or a linear two-clip blend, plus transition
  crossfades projected as bounded weighted clip sets;
- gameplay-origin timing facts that bind controller transitions to accepted
  owner facts without making renderer mixer time authoritative;
- renderer-local typed clip/keyframe cues for cosmetic audio/VFX, explicitly
  excluded from replay truth and forbidden from mutating gameplay;
- compatibility playback operations for direct play/pause/stop/resume of a
  named clip on a `RenderHandle`;
- Pose sampling (root transform + hierarchy summary) for readout
- Fixture manifest with content-hash validation

**What's missing (and matters):**

| Unity Concept | Asha Gap | Why It Matters |
|---|---|---|
| Nested blend trees / layered animation | One linear two-clip blend per state; no layers or masks | Upper-body aiming while lower-body runs and richer locomotion spaces need explicit future graph vocabulary. |
| Gameplay-critical keyframe actions | Renderer markers are cosmetic only by design | Attack hitboxes and damage must originate from gameplay authority; authored timing can request presentation but cannot become the fact source. |
| Animation curves / parameter binding | No float curves driving material/property changes | Machine glow intensity pulsing with a cycle. Conveyor belt UV scroll rate. |
| IK / procedural animation | None | Foot placement on stairs/slopes. Looking at target. Hand reaching for lever. |

**Current posture:** author gameplay-facing graphs and parameters through the
implemented controller; use sampled keyframe cues only for disposable
presentation. Extend blend/layer/curve vocabulary when product graphs prove the
need rather than building a Mecanim clone. See
[`animation-controller-authority.md`](animation-controller-authority.md) and
[`animation-timing-semantics.md`](animation-timing-semantics.md).

### 3. Audio — BASELINE IMPLEMENTED ✅ (Wave 1)

The original analysis found no audio path. Task #5595 now provides a generated
G1 audio contract, Rust catalog/descriptor/lifecycle validation, stable native
`RuntimeProjectionFrame` delivery, and a Web Audio host on the public
`@asha/renderer-host` root. See
[`audio-projection.md`](audio-projection.md).

**Why this is high priority despite being "non-visual":**

Audio in OSHApunk is not cosmetic. It is the primary feedback channel for:
- Machine state changes (conveyor running vs jammed vs idle)
- Factory throughput (density of clanks = production rate)
- Alert conditions (alarm when power drops, when belt backs up)
- Spatial orientation (which direction is the jammed assembler?)
- Ambient presence (the factory "breathes")

Without audio infrastructure, the factory simulator has no non-visual feedback
layer. Every status change must be communicated through UI panels or world-space
text — both expensive in screen real estate and attention.

**Implemented baseline:**

1. **AudioSource projection** — one-shot `emit` plus retained
   create/update/destroy, with global 2D, world 3D, and entity-attached emitters.
2. **AudioListener realization** — the downstream shell supplies its projected
   camera position/forward/up to the host without making listener state
   authority.
3. **AudioClip asset type** — `AssetKind::AudioClip`, closed-catalog lookup,
   projected hash validation, and browser-side SHA-256 verification before
   decode.
4. **Simple buses** — fixed SFX, ambient, and UI gain groups.
5. **Spatial audio** — equal-power panning and distance attenuation through
   `PannerNode`.

**Architecture note:** Audio operations are disposable projections with
`excludedFromReplayTruth`; their owner/gameplay origin remains inspectable and
replayable. `protocol-presentation` owns the shared generated envelope,
`render-audio` validates the audio domain, and `renderer-host` realizes it.

**Still deferred:** reverb zones, occlusion, mixer snapshots/automation,
custom HRTF, streaming, and procedural synthesis.

### 4. Input Handling — BASELINE IMPLEMENTED ✅ (Medium Priority)

**What it has:**
- generated named action, binding, context, receipt, and replay contracts;
- Rust-owned catalog validation, priority/consumption resolution, active
  context stack, and deterministic recorded-action replay;
- `BrowserInputHost` as the DOM keyboard/pointer/wheel adapter, with raw browser
  details removed before gameplay/editor consumers see the action;
- separate FPS and editor consumers over the same platform-neutral action
  vocabulary, including high-priority menu/dialog/camera-navigation contexts;
- RuntimeSession readout and commands for configuration, context changes, raw
  sample submission, resolved-action replay, and inspection;
- collision-constrained first-person movement remains Rust authority.

**What's missing:**

| Unity Concept | Asha Gap | Why It Matters |
|---|---|---|
| Input rebinding UI | Catalogs are typed and replaceable, but there is no player-facing capture/conflict/save workflow | OSHApunk will eventually want customizable tool, camera, and overlay bindings. |
| Gamepad support | None | Factory builder on Steam Deck? Deferrable but worth not closing the door. |
| Touch/gesture/accessibility devices | No approved raw adapters yet | These should map into the same named-action resolver without changing gameplay consumers. |
| Text/IME composition | Deliberately outside semantic action replay | Dialog and text widgets need a separate focused text-input lane rather than pretending characters are gameplay actions. |

**Current posture:** extend the adapter and authoring surfaces, not the
authority model. New devices produce normalized raw samples; new modes declare
contexts/actions; Rust retains priority, consumption, replay, and Session-state
ownership. See [`named-input-actions.md`](named-input-actions.md).

### 5. Physics (Dynamics) — STUB ⚠️ (Medium Priority)

**What it has:**
- `svc-collision`: collision queries and projections via `parry3d-f64`.
  AABB, raycasting, collision world as derived projection from voxel state.
- `svc-physics`: exists but minimal (depends on core-error, core-math,
  core-time — no rapier dynamics, no rigid bodies, no joints).
- Kinematic trigger volumes publish deterministic enter/exit owner facts from
  authoritative transforms and persist overlap lifecycle across snapshots.

**What's missing:**
- Rigid body dynamics (gravity, velocity integration, force application)
- Joints/constraints (hinges, sliders for machine parts)
- Continuous collision detection (fast-moving belt items)
- Physics materials (friction, restitution)

**Is this needed?** For v1: mostly no. Voxel terrain is static. Characters
move on a navmesh/grid. Machines are anchored to grid. The main dynamics use
case is *belt items* — hundreds of small objects sliding along conveyor paths.
But belt items follow deterministic paths, not physics simulation.

The one place physics dynamics would help: *ragdoll death animations* and
*knockback from combat*. Both are visual-only (cosmetic layer). Defer.

**Recommendation:** Keep `parry3d` for collision queries (working well).
Defer `rapier3d` dynamics until belt items or physics props are a confirmed
product need. The collision world projection pattern is solid — extend it
when needed.

### 6. Navigation / Pathfinding — EXISTS ✅

**What it has:**
- `svc-pathfinding`: A*/grid-based pathfinding over voxel walkability
- Depends on `svc-spatial` spatial index and `core-space` coordinate types
- Well-scoped: path queries, not navmesh generation

**What's solid:** Good foundation. The voxel grid IS the navmesh. Pathfinding
queries are deterministic service calls. No gap here for v1 needs.

**Future gap:** Multi-floor pathfinding (ladders/stairs connecting vertical
levels). Rimworld-style room graph traversal (path through doors, avoid locked
rooms). Both are extensions of the current service, not a replacement.

### 7. Particle Systems / VFX — BASELINE IMPLEMENTED ✅ (Wave 1)

The generated G1 presentation frame now carries particle burst emits and
retained emitter create/update/destroy operations. Rust validates catalog-bound
sprites, anchors, rate/burst, lifetime and velocity ranges, acceleration,
ordered size/color curves, flipbook rate, seed, visibility, handle lifecycle,
and explicit budgets. The renderer host owns bounded per-particle simulation
and billboard realization through an injected sink.

**What matters for OSHApunk:**
- Machine operation feedback (sparks from assembler, smoke from smelter)
- Conveyor belt item visuals (items sliding along, stacking at junctions)
- Combat feedback (muzzle flash, impact sparks, damage numbers)
- Environmental ambience (dust motes in factory, steam vents)
- UI overlays (placement preview glow, belt direction arrows)

Unlike a Unity `ParticleSystem` component, the stored/runtime identity does not
own thousands of mutable particle records. Gameplay or owner code selects a
typed effect and anchor; `render-particle` projects it beside scene diffs; the
host realizes it. The seed supports stable debugging, while particle positions
remain explicitly outside replay truth. This preserves the useful Unity author
experience without importing component callbacks or renderer state into Rust
authority.

`@asha/cosmetic` also exposes a one-way adapter from particle bursts into its
existing `hit_spark` view model. Local UI-only screen effects remain outside
the Rust border. See [Particle projection](particle-projection.md).

**Remaining gap:** richer authoring and realization: GPU/compute paths, mesh
particles, collision, sub-emitters, ribbons, lights, and a VFX graph. None is a
prerequisite for the Wave 1 gameplay feedback path.

This is not urgent for simulation prototyping but becomes critical the moment
the factory needs to "feel alive." A factory with silent, motionless machines
is a spreadsheet, not a game.

### 8. World-Space UI — BASELINE IMPLEMENTED ✅ (Wave 1)

The generated G1 presentation frame now carries retained billboard
create/update/destroy operations with world/entity anchors, localized
text/value/icon content, explicit Font/Texture asset posture, screen-space size,
distance culling, and display layers. `render-billboard` validates the contract
and `AshaBillboardHost` realizes it without making host state authoritative.
See [`billboard-projection.md`](billboard-projection.md).

**Why this is high priority:**

World-space UI is how OSHApunk communicates state that isn't visible in the
voxel geometry:
- Machine status (producing / starved / jammed / off)
- Belt throughput (items/minute, backed up)
- Character state (health bar, current task, mood)
- Interaction prompts (what can I do with this thing I'm looking at)
- Zone overlays (machine range, logistics coverage, pollution radius)

Without world-space UI, all status information is either invisible or requires
opening inspector panels. For a factory builder, that's unplayable — you need
to see your factory's state at a glance.

**Implemented baseline:** the billboard domain provides:
- Position in world space, screen-space size, anchor point
- Text content + optional icon
- Visibility conditions (distance, occlusion, mode)
- Render layer (always-on-top, depth-tested, etc.)

This is a render-only concern — Rust authority owns *what* text should display
(status text, value), TS renderer owns *how* it's rendered (billboard quad
with canvas texture, CSS3D, sprite).

Interactive billboards, rich text/widgets, automatic batching, damage-number
helpers, and an engine-owned occlusion query remain future work.

### 9. Prefabs / Templates — SUBSTANTIAL WAVE 1 ✅

**What it has:**
- `FlatSceneDocument` + `SceneNodeRecord`: scene files are flat node lists
  with parent-child, kind, asset reference, transform
- `game-workspace` TS package: manifest + assets + authoring
- Scene objects have explicit create/delete/reparent commands
- Public prefab draft create/replace/delete/instantiate commands, browser and
  selection readouts, stable part-role inspection, binding/configuration
  readouts, and canonical source serialization
- Validated Rust registry loading, deterministic authored/player placement,
  one-level variants, typed per-instance overrides, stable role resolution,
  provenance, save/restore/replay, and public RuntimeSession readouts
- A downstream two-instance multi-part proof with distinct overrides, typed
  prefab-part gameplay execution, and visible world-space placement

**What remains:**
- **Nested prefabs:** "Factory Wing contains 4 Assemblers + 2 Conveyors."
  Without nesting, factory blueprints are flat monster documents.
- **Propagating definition edits:** Existing accepted live instances do not
  silently update when a stored definition changes.
- **Richer Studio UX:** The public data/readout path exists, but a full visual
  hierarchy editor, drag placement, preview, undo integration, and variant diff
  interface remain product work.

**Why this is high priority for OSHApunk specifically:**

OSHApunk is a factory builder. The core gameplay loop is placing machines.
Every machine type is a prefab. Every conveyor segment is a prefab. Every
decorative prop is a prefab. Without prefabs:
- The scene document for a 100-machine factory is a flat list of 500+ nodes
  with repeated structure
- "Upgrade all Mk1 Assemblers to Mk2" is a find-and-replace nightmare
- "Share this factory layout" has no serializable unit smaller than the
  entire scene

**Current design:** `PrefabDefinition` is a first-class ProjectBundle protocol
type. Consumer tools prepare stored drafts, while Rust validation and
`PrefabInstanceAuthority` own expansion into normal Session Entity authority.
Gameplay binds to stable prefab-part roles rather than hierarchy paths. See
[`prefab-authoring-and-placement.md`](prefab-authoring-and-placement.md).

### 10. Serialization / Persistence — EXISTS ✅

**What it has:**
- `svc-serialization`: save/load for scenes with asset validation
- `core-snapshot`: `StateSnapshot` with round-trip + stable hash
- `sim-replay`: `ReplayRecord` with step hashing and divergence detection
- `core-entity/persist`: `encode_snapshot` / `decode_snapshot` with schema
  version

**What's solid:** This is well-architected. Deterministic hashing. Schema
versioning. Replay verification. Project bundles with ordered load plans.

**Gap:** No differential/incremental save. Every save is a full snapshot.
For OSHApunk with thousands of entities, this will become expensive. But
this is an optimization, not an architectural gap — the foundation supports
adding delta-based saves later.

### 11. Resource / Asset Management — EXISTS ✅

**What it has:**
- `core-assets`: `AssetReference`, `AssetKind` (Material, StaticMesh, Sprite,
  SpriteSheet, Texture, AudioClip, Font, VoxelVolume, VoxelObject, Script, Scene)
- `core-catalog`: catalog entries with lifecycle + fallback decisions
- `svc-voxel-asset`: validation, canonicalization, hashing for voxel assets
- `protocol-assets`: border types for asset drift detection
- Asset binding on entities via `AssetBindingCapability`

**What's solid:** Good separation of asset identity from asset data. Catalog
with fallback chains. Hash-based drift detection across the bridge.

**Gap:** No async loading with priority. Everything is assumed loaded at
bootstrap. Streaming assets (load the next factory wing's textures while the
player approaches) doesn't exist. Defer to v1+.

### 12. Scripting / Hot-Reload — MISSING ❌ (Low Priority)

**Nothing exists.** Policy packs are authored as TypeScript packages that must
be compiled and loaded explicitly. No file watcher. No runtime script editing.
No live policy iteration.

**Is this needed?** For OSHApunk v1 as a standalone game: no. Policy is baked
at build time. But for the *development workflow* of iterating on game rules
("make assemblers 20% faster and see how the factory responds"), a live-reload
capability would dramatically speed up tuning.

**Recommendation:** Defer. The policy script host already has the right
architecture (load a pack, invoke policies). Adding a file watcher that
reloads packs on change is a thin layer on top. Do it when the tuning
bottleneck becomes painful, not before.

### 13. Networking / Multiplayer — MISSING ❌ (Deferred)

**Nothing exists.** Zero networking infrastructure. No replication, sync,
prediction, rollback, RPC, or netcode. Architecture is single-player/local.

**Is this needed?** For OSHApunk v1: no. Factory builder as single-player
experience. Even "visit friend's factory" can be implemented as "share save
file." Co-op factory building is a v2 feature at best.

**Architecture note:** The authority/replay model is multiplayer-friendly (one
authoritative simulation, clients render projected state). If multiplayer
becomes a goal, the command→validate→event→diff pipeline maps cleanly to
server-authoritative with client-side prediction. But that's a massive
undertaking and not a gap for v1.

### 14. Profiling / Stats — PARTIAL ⚠️ (Low-Medium Priority)

**What it has:**
- `ts/packages/smoke`: CLI perf harness with phase timings, structural counters,
  JSON(L) output, trend tracking, GPU perf lane
- Deterministic benchmark fixtures (edit→render cycles, replay divergence checks)
- Perf metadata: schema version, commit, branch, host label, runtime mode
- A generated `LiveTelemetrySnapshot` read independently by headless tools and
  the `AshaTelemetryOverlayHost`; bounded frame-time history and unavailable
  counters are explicit instead of fabricated
- A live downstream overlay showing real authority/projection/feedback-host
  gauges through the G1 telemetry-overlay lifecycle

**What's missing:**
- **In-engine profiler:** Per-system timing breakdown (physics took X ms,
  pathfinding took Y ms, render projection took Z ms).
- **Memory tracking:** Entity count by capability type. Chunk memory by state
  (generated, meshed, uploaded). Asset reference counts.
- **Unavailable live owners:** Draw calls and chunk-state counts remain
  diagnosed as unavailable until approved public owner adapters expose them.

**Recommendation:** Keep the live snapshot compact and owner-derived. Add
per-system timing and memory counters only with stable units and public owner
adapters; route the eventual shared keyboard binding through the input work in
#5642 rather than giving the overlay ambient input access. See
[Live telemetry snapshot and overlay](live-telemetry-overlay.md).

### 15. Coroutines / Async Task Scheduling — PARTIAL ⚠️ (Medium Priority)

**What it has:**
- `rule-scheduler`: deterministic work scheduling for chunk ops with priority
  queues and version checking (Generate→Mesh→Collision→Upload)
- `core-time::TickInterval`: periodic cadence ("every N ticks")
- Work items are `Ord`, enabling parallel execution with deterministic apply
- a separate gameplay action scheduler for tick- and event-conditioned work,
  with stable IDs/priorities, full retained proposal envelopes, canonical owner
  routing, exactly-once event delivery, interruption recovery, snapshots, replay,
  and public RuntimeSession readout;
- a scoped Rust scheduler port as the command-authority boundary, rather than a
  caller-supplied owner string or TypeScript callback.

**What's missing:**
- **Arbitrary coroutine continuations:** there is no general suspended stack or
  callback capture, intentionally. Gameplay schedules typed moments/proposals.
- **Composite joins:** "run three actions, continue when all complete" needs an
  explicit rule/state-machine owner; the scheduler does not invent a hidden
  orchestration graph.
- **Wall-clock waits:** gameplay timing remains tick-based. Browser timers do
  not become simulation authority.

**Current posture:** the implemented gameplay scheduler is the governed
`DeferredAction` equivalent. Add new typed conditions/moments through the
gameplay fabric when real gameplay demands them; do not add ambient callbacks
or a general coroutine runtime. See
[`gameplay-action-scheduler.md`](gameplay-action-scheduler.md).

### 16. Camera System — PARTIAL ⚠️ (Medium Priority)

**What it has:**
- one Rust-owned camera controller with first-person, orbit, and top-down modes;
- revision-guarded mode changes plus orbit/top-down pan, rotate, and zoom;
- voxel collision/terrain clearance, typed atomic rejection, stable controller
  state/readout, and accepted endpoint hashes;
- named-input context exclusion so FPS movement and pivot navigation cannot run
  together;
- renderer-only interpolation between accepted endpoints.

**What's missing:**
- **Camera stack / layering:** Overlay camera for UI in world space, main
  camera for world, minimap camera. Unity's camera stacking.
- **Cinematic rails/follow/shake:** There is no authored target-follow,
  cutscene rail, or camera-shake system.
- **Multiple active viewports:** One accepted controller is active per current
  Session surface; minimap and editor split-view composition remain future work.
- **Post-processing volumes:** Entering a smokey factory wing triggers
  different color grading / fog than outdoor areas. Pure renderer concern
  but needs camera-relative triggers.

**Current posture:** use the shared controller for ordinary embodied play and
factory/editor navigation. Add cinematic or multi-viewport projection as
separate typed concerns without creating another active camera authority. See
[`camera-modes.md`](camera-modes.md).

### 17. Material System — FEEDBACK SLICE IMPLEMENTED ✅ (Medium Priority)

**What it has:**
- `protocol-render::Material`: flat Rgba color + wireframe flag
- `RenderMaterialDescriptor`: catalog-driven material with slots and UV strategy
- `MaterialUvStrategy`: Flat, Planar, Atlas
- Material slots on static meshes, resolved from catalog

**What is now implemented:**
- Versioned material descriptors with texture tint and explicit emission
  colour/intensity, including legacy defaults.
- Per-instance, per-slot material feedback updates on stable render handles.
- `MeshStandardMaterial` realization with instance isolation and reset.
- Authority-derived structural goldens and a live two-state WebGL proof.

**What's still missing:**
- **PBR materials:** Albedo, metallic, roughness, normal map, emission.
  Currently everything is flat-shaded with a single color.
- **Material instances:** "This specific wall has a dirt overlay" without
  creating a new material asset.
- **Shader graph / custom materials:** Defer — not needed for stylized voxel.
- **Texture atlasing:** Planned but not built. Important for voxel face
  texturing to avoid thousands of draw calls.
- **Parameter animation/blending:** Runtime changes are discrete retained
  operations; renderer-owned interpolation remains deferred.

**Why this matters for OSHApunk:** The voxel aesthetic depends on materials
to communicate state. A factory where all machines are flat-shaded single-color
blocks is illegible. Color coding (green = running, yellow = starved, red =
jammed) is the minimum viable visual language.

**Recommendation:** Use the implemented typed material-feedback operation for
discrete gameplay states. Keep PBR maps and parameter animation/blending for a
later renderer pass; do not replace this narrow contract with a generic shader
or plugin vocabulary. See `docs/material-feedback.md`.

### 18. LOD System — MISSING ❌ (Low-Medium Priority)

**Nothing exists.** No level-of-detail for meshes, chunks, or entities.

**Is this needed?** For OSHApunk v1: not critical. Voxel chunks are already
roughly uniform density. Chunk meshing produces optimized geometry. But as
factory scale grows (viewing 50+ chunks simultaneously), LOD becomes relevant.

The natural LOD for voxels is: distant chunks use larger voxels (2x2x2 or 4x4x4
marching cubes) or switch to impostor meshes. Entities can use simpler meshes
at distance.

**Recommendation:** Defer to v1+. The chunk scheduler already has priority
queues — add a `lod_level` field to `WorkItem` and let the mesher produce
simpler geometry for distant chunks. The render protocol already has handles,
so swapping a handle's mesh at LOD transition is straightforward.

### 19. Gameplay Fabric — IMPLEMENTED (Wave 1 static) ✅

The old recommendation for a closed `GameEvent` enum and `EventReaction` table
has been superseded. It would move the bottleneck without solving downstream
ownership: every new game meaning would still require an engine edit.

**What exists now:**

- open, versioned `GameplayContractRef` identities and typed Rust codecs;
- immutable static providers, subscriptions, Guard/Transform/React decision
  participants, owner registrations, budgets, and ordering;
- typed semantic adapters for lifecycle, capability activation, triggers,
  combat, state machines, processes, modifiers, and scheduled moments;
- declared frozen event/capability/relationship/prefab/module-state reads plus
  bounded owner queries;
- ProjectBundle-authored module configuration and bindings to Session,
  EntityDefinition, Prefab, and stable prefab-part scopes;
- module-owned typed persistent state, facts, snapshots, migration, playback,
  and verification replay;
- public static SDK, conformance kit, runtime host, browser transport, and
  bounded reaction/decision evidence;
- real consumer proofs in `asha-demo` and `asha-rulebench`.

This supports the gameplay problems the old section identified without
polling or a central enum. A machine-completion owner can publish a namespaced
typed event; one downstream module can update its production state or propose a
shared mutation; disposable audio/UI/telemetry projections can follow the same
causation identity. None of those presentations becomes authority.

**Current limit:** Wave 1 is statically linked. There is no dynamic module
loading or TypeScript handler authority, and further engine-owned shared
proposal routes are added owner by owner. See
`gameplay-fabric-growth-recipes.md` for the paved road.

### 20. Component Lifecycle — PARTIAL ⚠️ (Medium Priority)

**What it has:**
- `EntityLifecycle`: Active, Disabled, Destroyed
- `EntitySource`: Scene, Runtime, Generated
- Lifecycle commands: Create, Delete, Enable, Disable
- Capability tables (TransformCapability, etc.) — not behavior components

**What exists now:**
- individual typed capability activation/deactivation under closed Rule owners;
- authority-owned EntityDefinition/ProjectBundle validation and atomic
  bootstrap;
- named tick and scheduler moments published through the gameplay fabric;
- statically composed downstream Rust modules with Session/entity/prefab-scoped
  state and explicit initialization adapters;
- trigger enter/exit facts and persistent overlap lifecycle.

**Is this a gap?** It depends on the architectural philosophy. The current
design is system-oriented (services process entities in bulk) rather than
component-oriented (entities own their behavior). For a factory sim with
thousands of entities doing the same thing (belt items moving, assemblers
processing), the system-oriented approach is more efficient. Unity's
`MonoBehaviour.Update()` per instance would be wasteful here.

**Current posture:** This remains a deliberate non-Unity shape. There is no
per-entity `Update()` or `RuleHooks` callback trait. Compiled Rules/services own
high-frequency simulation. Modules observe named accepted facts or scheduled
moments through a closed manifest and return typed outputs. Capability
activation and module bindings replace component enablement and `Awake/Start`
where those concepts are actually needed, while keeping invocation and replay
explicit.

---

## Overall Assessment

**What Asha Engine does exceptionally well:**
- Authority/render separation with generated protocol contracts
- Deterministic simulation with replay verification
- Voxel pipeline (generation → meshing → collision → rendering)
- Entity/capability data model
- Serialization with schema versioning and hash verification
- CI governance and agent lane enforcement

**The critical integration work for a playable OSHApunk v1:**

1. **Gameplay fabric adoption** — the substrate and one-cell RuntimeSession
   path exist; individual game owners still need to publish rich semantic facts
   and build real modules.
2. **World-space UI authoring/content** — the billboard path exists; machine,
   belt, character, prompt, and zone projections still need product content and
   Studio workflows.
3. **Input catalog/product coverage** — named contexts and browser FPS/editor
   routing exist; the game still needs its complete action catalog, rebinding
   UX, and eventual non-keyboard adapters.
4. **Domain lifecycle design** — use named facts/moments, the gameplay
   scheduler, and compiled modules rather than per-entity callback emulation.

**The "nice to have" gaps:**
- Animation layers, nested blends, curves, and IK beyond the controller baseline
- Material PBR maps and continuous parameter animation beyond feedback tint/emission
- Particle/VFX authoring beyond the baseline emitters
- Cinematic, multi-viewport, and camera-stack behavior beyond FPS/orbit/top-down
- Composite scheduling/join authoring beyond the typed deferred-action baseline
- Per-system profiling and memory overlay beyond the delivered live stats baseline

**The defer-to-v1+ gaps:**
- Physics dynamics (rigid bodies, joints)
- LOD system
- Multiplayer/networking
- Hot-reload scripting
- Incremental/differential saves
- PBR/shader graph
- IK/procedural animation

**Architecture risk:** The engine is strong at infrastructure but many systems
still have more isolated proof than varied product use. Audio now has one real
downstream interaction and prefabs have a typed instantiation path, but input
routing and the remaining presentation domains still need integrated gameplay
  pressure. Audio, billboards, particles, and the live telemetry overlay now
  share the accepted G1 frame;
later domains must preserve its typed closed-envelope and independent failure
behavior rather than growing a generic host event bus.
