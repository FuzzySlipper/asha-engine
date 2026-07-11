# Asha Engine — Unity Gap Analysis (2026-07-10)

What Unity provides as fundamental infrastructure that Asha Engine currently
lacks or only partially covers. Scoped to what matters for OSHApunk: a voxel
factory builder + Rimworld-style simulation with an FPS exploration mode.
**Not** scoped to Unity-specific features like lightmap baking, Mecanim,
terrain heightmaps, asset store integration, or multi-vendor platform support.

Assessment basis: `/home/dev/asha-engine` (93K LOC Rust, 78 crates, 257 files)
+ `/home/dev/asha-studio` (60K LOC TypeScript, Nx monorepo).

---

## Summary Table

| # | Subsystem | Status | Priority for OSHApunk |
|---|---|---|---|
| 1 | Scene graph / entity hierarchy | **EXISTS** | — |
| 2 | Animation | **PARTIAL** | Medium |
| 3 | Audio | **MISSING** | High |
| 4 | Input handling | **PARTIAL** | High |
| 5 | Physics (dynamics) | **STUB** | Medium |
| 6 | Navigation / pathfinding | **EXISTS** | — |
| 7 | Particle systems / VFX | **MISSING** | Medium |
| 8 | World-space UI | **MISSING** | High |
| 9 | Prefabs / templates | **PARTIAL** | High |
| 10 | Serialization / persistence | **EXISTS** | — |
| 11 | Resource / asset management | **EXISTS** | — |
| 12 | Scripting / hot-reload | **MISSING** | Low |
| 13 | Networking / multiplayer | **MISSING** | Deferred |
| 14 | Profiling / stats | **PARTIAL** | Low-Medium |
| 15 | Coroutines / scheduling | **PARTIAL** | Medium |
| 16 | Camera system | **PARTIAL** | Medium |
| 17 | Material system | **BASIC** | Medium |
| 18 | LOD system | **MISSING** | Low-Medium |
| 19 | Gameplay event bus | **PARTIAL** | Medium |
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
- Playback operations: play/pause/stop/resume a named clip on a `RenderHandle`
- Pose sampling (root transform + hierarchy summary) for readout
- Fixture manifest with content-hash validation

**What's missing (and matters):**

| Unity Concept | Asha Gap | Why It Matters |
|---|---|---|
| Animation Controller / State Machine | No blend trees, no transitions, no parameter-driven state | Factory machines cycle through idle→active→idle; characters transition walk→run→idle. Doing this with raw clip-name calls per tick requires ad-hoc state in every consumer. |
| Animation events | No callback/event at specific keyframes | Footstep sounds, attack hitboxes, machine activation sparks — all need keyframe-anchored callbacks. |
| Blend trees / layered animation | Single clip at a time | Upper-body aiming while lower-body runs. Walk speed blending between slow and fast gaits. |
| Animation curves / parameter binding | No float curves driving material/property changes | Machine glow intensity pulsing with a cycle. Conveyor belt UV scroll rate. |
| IK / procedural animation | None | Foot placement on stairs/slopes. Looking at target. Hand reaching for lever. |

**Recommendation:** Don't build a Mecanim clone. But define a minimal
`AnimationController` protocol type and a simple state-machine evaluator in
Rust that takes named float parameters → selects clip + blend weight. This
keeps animation logic deterministic and replayable. Keyframe events can be a
separate concern (or deferred to the audio system landing first).

**Current workaround:** Consumers call `playClip(handle, 'idle')` directly.
Fine for proofs. Won't scale past ~5 animated entity types.

### 3. Audio — MISSING ❌ (High Priority)

**Nothing exists.** Zero audio crates. Zero audio TS packages. No AudioSource,
AudioListener, mixer, spatial audio, or even a "play sound" API.

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

**What's needed (minimal):**
1. **AudioSource concept** — attachable to entities/positions. Plays a clip
   (sound file reference), with volume, pitch, spatial blend, looping, and
   play-on-awake flag. Deterministic: playback state is authority-owned.
2. **AudioListener** — single camera-attached listener for spatialization.
3. **AudioClip asset type** — catalog entry for sound files (wav/ogg). Asset
   pipeline validates and hashes.
4. **Simple mixer** — master volume, maybe 2-3 buses (SFX, ambient, UI).
   No need for Unity's full mixer graph with snapshots and effects.
5. **Spatial audio** — distance attenuation + stereo panning. No need for
   reverb zones or occlusion — defer to post-v1.

**Architecture note:** Audio playback is inherently non-deterministic at the OS
level (audio buffer timing, hardware latency). The engine should treat audio
commands as fire-and-forget from the replay perspective — they're logged for
debugging but not replayed for state verification. The authority owns *what*
should play; the renderer owns *when* the buffer reaches the DAC.

**Rust/TS split:** `protocol-audio` defines border types. `svc-audio` validates
play requests against catalog. Render bridge emits `PlayAudioCommand` diffs.
TS `renderer-host` maps them to Web Audio API nodes.

### 4. Input Handling — PARTIAL ⚠️ (High Priority)

**What it has:**
- `browser-fps-input.ts`: raw DOM keyboard/mouse → FPS camera movement
  (WASD + mouse look + pointer lock)
- `FirstPersonMotionInput` / `FirstPersonMotionCommand` in Rust
- Camera collision constraints in the bridge authority

**What's missing:**

| Unity Concept | Asha Gap | Why It Matters |
|---|---|---|
| Input Action Maps | No action abstraction — raw key codes | "Jump" is Space in FPS mode but should be a UI button in editor mode. "Interact" is E near machines but Enter in dialogs. Without action maps, every mode switch requires if/else on raw key codes. |
| Input rebinding | No rebinding infrastructure | Not urgent for v1, but OSHApunk's factory controls will want customizable hotkeys (tool palette, camera bookmarks, overlay toggles). |
| Gamepad support | None | Factory builder on Steam Deck? Deferrable but worth not closing the door. |
| Input consumption / priority | No focus/layer system | When an IMGUI panel is open, should WASD move the camera or type in the text field? Current code has no concept of input focus. |
| Editor vs gameplay modes | Raw key codes hardcoded in FPS input | Tool mode switch (place/remove/paint/select) vs FPS movement vs UI panel interaction — three input contexts with no abstraction. |

**Recommendation:** Define a small `InputAction` protocol: named actions
("move_forward", "jump", "interact", "tool_place", "tool_select") with
platform-agnostic bindings. Actions map to contexts ("fps", "editor",
"menu"). The bridge validates and routes per context. No need for Unity's
full input system with processors and interactions — just named actions +
context switching + deterministic binding lookup.

**Why this matters now:** The `editor-tools` package already has a `ToolMode`
concept. The FPS pipeline has `FirstPersonMotionInput`. These are two separate
input systems with no shared vocabulary. Adding a third (UI panel interaction)
would create a three-way ad-hoc input routing problem. A small input action
layer prevents that.

### 5. Physics (Dynamics) — STUB ⚠️ (Medium Priority)

**What it has:**
- `svc-collision`: collision queries and projections via `parry3d-f64`.
  AABB, raycasting, collision world as derived projection from voxel state.
- `svc-physics`: exists but minimal (depends on core-error, core-math,
  core-time — no rapier dynamics, no rigid bodies, no joints).

**What's missing:**
- Rigid body dynamics (gravity, velocity integration, force application)
- Joints/constraints (hinges, sliders for machine parts)
- Trigger volumes (enter/exit callbacks for factory zones)
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

### 7. Particle Systems / VFX — MISSING ❌ (Medium Priority)

**Nothing exists.** The `cosmetic` TS package exists but is likely a stub.
Zero particle infrastructure in Rust or TS.

**What matters for OSHApunk:**
- Machine operation feedback (sparks from assembler, smoke from smelter)
- Conveyor belt item visuals (items sliding along, stacking at junctions)
- Combat feedback (muzzle flash, impact sparks, damage numbers)
- Environmental ambience (dust motes in factory, steam vents)
- UI overlays (placement preview glow, belt direction arrows)

**Recommendation:** Minimal particle system:
1. `ParticleEmitter` capability on entities — position, emission rate,
   lifetime, velocity range, color gradient, size curve, sprite sheet.
2. Render bridge emits `CreateParticleEmitter` / `UpdateParticleEmitter` /
   `DestroyParticleEmitter` diffs.
3. Three.js renderer implements GPU particle simulation (transform feedback
   or compute shader — or CPU for v1 simplicity).
4. Deterministic seed for replay (particle positions are authoritative but
   visual-only — replay can skip or approximate).

This is not urgent for simulation prototyping but becomes critical the moment
the factory needs to "feel alive." A factory with silent, motionless machines
is a spreadsheet, not a game.

### 8. World-Space UI — MISSING ❌ (High Priority)

**Nothing exists.** There's `ui-dom` (DOM-based UI for studio) and
`studio-panels` (Angular panels), but zero world-space UI infrastructure:
no health bars, no nameplates, no belt throughput indicators, no interaction
prompts ("Press E to interact"), no damage numbers.

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

**Recommendation:** Add a `Billboard` concept to the render protocol:
- Position in world space, screen-space size, anchor point
- Text content + optional icon
- Visibility conditions (distance, occlusion, mode)
- Render layer (always-on-top, depth-tested, etc.)

This is a render-only concern — Rust authority owns *what* text should display
(status text, value), TS renderer owns *how* it's rendered (billboard quad
with canvas texture, CSS3D, sprite).

The `hud-menu-projection.md` doc exists, suggesting this is already on the
radar as planned-but-not-built.

### 9. Prefabs / Templates — PARTIAL ⚠️ (High Priority)

**What it has:**
- `FlatSceneDocument` + `SceneNodeRecord`: scene files are flat node lists
  with parent-child, kind, asset reference, transform
- `game-workspace` TS package: manifest + assets + authoring
- Scene objects have explicit create/delete/reparent commands

**What's missing:**
- **Prefab concept:** A reusable template that can be instantiated multiple
  times with per-instance overrides. Current scene files are single-use
  documents — you can't say "place another Assembler Mk1 here."
- **Prefab variants:** "Assembler Mk2 is Mk1 with different material + faster
  processing speed." Without variants, every machine tier duplicates the full
  definition.
- **Nested prefabs:** "Factory Wing contains 4 Assemblers + 2 Conveyors."
  Without nesting, factory blueprints are flat monster documents.
- **Prefab overrides:** Per-instance property changes that survive prefab
  updates. "This specific assembler is painted red because it's in the danger
  zone."
- **Instantiation at runtime:** Policy/scripting can say "spawn prefab X at
  position Y" without knowing the internal node structure.

**Why this is high priority for OSHApunk specifically:**

OSHApunk is a factory builder. The core gameplay loop is placing machines.
Every machine type is a prefab. Every conveyor segment is a prefab. Every
decorative prop is a prefab. Without prefabs:
- The scene document for a 100-machine factory is a flat list of 500+ nodes
  with repeated structure
- "Upgrade all Mk1 Assemblers to Mk2" is a find-and-replace nightmare
- "Share this factory layout" has no serializable unit smaller than the
  entire scene

**Recommendation:** Define `PrefabDefinition` as a first-class protocol type:
- `PrefabId`, base `FlatSceneDocument` (the template), override slots
- Instantiation produces a set of `SceneNodeRecord`s with instance IDs
- Variants are prefabs that reference a base prefab + delta
- Instance overrides stored in a separate table (not baked into the node
  records)

This is a data model concern, not a rendering concern. It lives in `core-scene`
and flows through the same scene document pipeline. The studio gets a "save as
prefab" command and a prefab browser panel. The game gets "instantiate prefab"
as an authoritative command.

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
- `core-assets`: `AssetReference`, `AssetKind` (Mesh, Sprite, Audio, VoxelVolume,
  Material, Font, etc.)
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

**What's missing:**
- **Real-time stats overlay:** Frame time, draw calls, entity count, chunk
  count, memory usage. The harness is an offline tool — nothing shows live
  stats during gameplay.
- **In-engine profiler:** Per-system timing breakdown (physics took X ms,
  pathfinding took Y ms, render projection took Z ms).
- **Memory tracking:** Entity count by capability type. Chunk memory by state
  (generated, meshed, uploaded). Asset reference counts.

**Recommendation:** The harness is great for CI regression detection. Add a
lightweight devtools overlay (toggle with F3) that shows the top 5-10
counters. The render protocol already has a diagnostics channel — extend it
with `TelemetrySnapshot` at a low frequency (once per second).

### 15. Coroutines / Async Task Scheduling — PARTIAL ⚠️ (Medium Priority)

**What it has:**
- `rule-scheduler`: deterministic work scheduling for chunk ops with priority
  queues and version checking (Generate→Mesh→Collision→Upload)
- `core-time::TickInterval`: periodic cadence ("every N ticks")
- Work items are `Ord`, enabling parallel execution with deterministic apply

**What's missing:**
- **Yield-over-time for game logic:** "Wait 3 seconds, then activate machine"
  currently requires manually tracking elapsed ticks in game rule state.
  Unity's `yield return new WaitForSeconds(3)` is sugar, but the underlying
  pattern (register a callback at tick + N) is genuinely useful.
- **Condition-based waiting:** "Wait until belt has 5 items, then start
  assembler." Currently requires polling every tick.
- **Parallel composition:** "Start these 3 machines simultaneously, proceed
  when all are done."

**Recommendation:** Don't build a full coroutine system. Add a `DeferredAction`
concept to the rule layer: `{ execute_at: Tick, action: Command }`. The rule
scheduler checks each tick for due deferred actions and feeds them back into
the command pipeline. This is 90% of the value for 10% of the complexity.

### 16. Camera System — PARTIAL ⚠️ (Medium Priority)

**What it has:**
- FPS camera: `FirstPersonMotionInput` → collision-constrained movement
- `CameraCreateRequest`, `CameraProjectionRequest`, `CameraCollisionSnapshot`
- Single active camera at a time

**What's missing:**
- **Multiple camera types:** The factory builder needs an orbit/top-down camera
  for building mode, an FPS camera for walking around, and potentially a
  cinematic camera for cutscenes. Currently only FPS exists.
- **Camera stack / layering:** Overlay camera for UI in world space, main
  camera for world, minimap camera. Unity's camera stacking.
- **Camera transitions:** Blend from orbit to FPS when entering first-person
  mode. Currently a hard cut.
- **Post-processing volumes:** Entering a smokey factory wing triggers
  different color grading / fog than outdoor areas. Pure renderer concern
  but needs camera-relative triggers.

**Recommendation:** The FPS camera works. Add an `OrbitCameraController` as a
parallel mode (not replacing FPS, just adding a second mode). The
`CameraMode` enum in the protocol already has room for this. Post-processing
can be deferred to the material/renderer upgrade pass.

### 17. Material System — BASIC ⚠️ (Medium Priority)

**What it has:**
- `protocol-render::Material`: flat Rgba color + wireframe flag
- `RenderMaterialDescriptor`: catalog-driven material with slots and UV strategy
- `MaterialUvStrategy`: Flat, Planar, Atlas
- Material slots on static meshes, resolved from catalog

**What's missing:**
- **PBR materials:** Albedo, metallic, roughness, normal map, emission.
  Currently everything is flat-shaded with a single color.
- **Material instances:** "This specific wall has a dirt overlay" without
  creating a new material asset.
- **Shader graph / custom materials:** Defer — not needed for stylized voxel.
- **Texture atlasing:** Planned but not built. Important for voxel face
  texturing to avoid thousands of draw calls.
- **Runtime material parameter changes:** "Turn the warning light red when
  machine jams." Currently requires a material swap or a new render handle.

**Why this matters for OSHApunk:** The voxel aesthetic depends on materials
to communicate state. A factory where all machines are flat-shaded single-color
blocks is illegible. Color coding (green = running, yellow = starved, red =
jammed) is the minimum viable visual language.

**Recommendation:** Extend `Material` to include:
- `emission: Option<Rgba>` — for glow effects (warning lights, engine heat)
- `texture_tint: Option<Rgba>` — multiply base texture by tint (color variation)

Keep PBR for post-v1. Emission alone enables 80% of the state-communication
use cases. Add a `SetMaterialParameter` render diff variant for per-instance
parameter overrides (tint, emission intensity) without material duplication.

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

### 19. Gameplay Event Bus — PARTIAL ⚠️ (Medium Priority)

**What it has:**
- `core-events::DomainEvent`: authoritative state change events (EntityCreated,
  EntityTagAdded, ModeDefined, etc.) — 15 variants
- `EventBatch`: per-tick ordered collection
- Events are typed enum variants, not a generic message bus

**What's missing:**
- **Game-level events:** "MachineCompletedCrafting", "BeltBackedUp",
  "CharacterEnteredZone", "PowerGridOverloaded". The current DomainEvent
  variants are low-level (entity created, tag added) — you can't subscribe
  to "assembler finished its recipe."
- **Event subscriptions:** Game rules, UI, and audio need to react to game
  events without polling state every tick. "When any assembler in zone 3
  completes a craft, play a sound and increment the production counter."
- **Event routing:** Scoped subscriptions (by entity, by zone, by tag).
  "Notify me about crafting events, but only from Assembler Mk2s."

**Why this matters:** The game-rules modifier system already models effects
applied to entities. But there's no way for a modifier to say "when the target
entity emits Event X, apply Effect Y." This is the core pattern for:
- Status effect propagation (machine jam → adjacent machines slow down)
- Achievement/stat tracking (crafted 100 items → unlock upgrade)
- Audio/visual feedback (assembler completed craft → play clank sound)
- Tutorial triggers (first time player places conveyor → show tip)

**Recommendation:** Add a `GameEvent` enum alongside `DomainEvent` — separate
type, same authority layer. `DomainEvent` is "what changed in state."
`GameEvent` is "what this change means in game terms." The rule layer
translates domain events into game events. Add an `EventReaction` table to
the game-rules modifier system: `{ trigger: Matcher<GameEvent>, action: ModifierId }`.

This is the biggest architectural gap for the simulation layer. Without it,
every game system must poll state every tick to detect changes.

### 20. Component Lifecycle — PARTIAL ⚠️ (Medium Priority)

**What it has:**
- `EntityLifecycle`: Active, Disabled, Destroyed
- `EntitySource`: Scene, Runtime, Generated
- Lifecycle commands: Create, Delete, Enable, Disable
- Capability tables (TransformCapability, etc.) — not behavior components

**What's missing:**
- **Behavior components:** There's no `Update()` method. No per-entity-per-tick
  behavior execution. Capabilities are pure data, not behavior. This is by
  design (ECS-like separation of data from systems), but it means there's no
  standard pattern for "this entity does X every tick."
- **Component enable/disable:** Individual capabilities can't be toggled.
  An entity either projects to render or doesn't — you can't disable just
  collision while keeping the render projection.
- **Initialization phases:** No Awake/Start equivalent. Bootstrapping an
  entity from a scene definition into runtime state is handled ad-hoc by each
  service.

**Is this a gap?** It depends on the architectural philosophy. The current
design is system-oriented (services process entities in bulk) rather than
component-oriented (entities own their behavior). For a factory sim with
thousands of entities doing the same thing (belt items moving, assemblers
processing), the system-oriented approach is more efficient. Unity's
`MonoBehaviour.Update()` per instance would be wasteful here.

**Recommendation:** This isn't a gap to "fix" — it's a legitimate architectural
choice. The system-oriented model is better for OSHApunk's simulation scale.
The missing piece is a *declared lifecycle contract*: "these are the hooks a
game rule can register for" (OnEntityCreated, OnTick, OnEntityDestroyed) so
game rules don't each implement their own polling loop.

The rule crates (`rule-lifecycle`, `rule-process`, `rule-scheduler`) are the
right place to define these hooks. Make them explicit as a `RuleHooks` trait.
No need for per-entity Update() — that way lies ECS fragmentation and
determinism headaches.

---

## Overall Assessment

**What Asha Engine does exceptionally well:**
- Authority/render separation with generated protocol contracts
- Deterministic simulation with replay verification
- Voxel pipeline (generation → meshing → collision → rendering)
- Entity/capability data model
- Serialization with schema versioning and hash verification
- CI governance and agent lane enforcement

**The critical gaps for a playable OSHApunk v1:**

1. **Audio** — zero infrastructure. A silent factory is a spreadsheet.
2. **Input action system** — raw keycodes don't survive mode switches.
3. **World-space UI** — no way to show machine status at a glance.
4. **Prefabs** — no reusable templates for factory pieces.
5. **Gameplay event bus** — no way to react to game events without polling.
6. **Component lifecycle hooks** — no standard pattern for entity behavior.

**The "nice to have" gaps:**
- Animation state machines (manual clip control works for now)
- Material upgrades (emission, texture tint)
- Particle systems (visual juice)
- Camera modes beyond FPS
- Deferred action scheduling
- Real-time profiling overlay

**The defer-to-v1+ gaps:**
- Physics dynamics (rigid bodies, joints)
- LOD system
- Multiplayer/networking
- Hot-reload scripting
- Incremental/differential saves
- PBR/shader graph
- IK/procedural animation

**Architecture risk:** The engine is strong at infrastructure but has mostly
been proven on single-feature proofs (place a voxel, load a scene, play an
animation). The integration-risk systems — audio, input routing, world-space
UI, prefab instantiation — are untested. Each is individually tractable, but
building all six while keeping the authority/render contract clean is where
the real integration complexity lives. The `rule-scheduler`'s work queue
pattern (abstract over execution, deterministic ordering, version-checked
staleness) is a good model to follow for the other systems. Consider whether
audio, particles, and world-space UI should share a common "render-side
projection" abstraction rather than each inventing their own bridge pattern.
