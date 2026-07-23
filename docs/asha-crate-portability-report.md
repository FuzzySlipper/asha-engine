# Asha Engine crate portability report

## Recommendation in brief

`rusty-engine` should treat Asha Engine as a donor library and a source of behavioral evidence, not as a workspace to copy wholesale. The useful extraction boundary is below the Asha runtime spine:

```text
portable values and algorithms
        -> successor-owned state and named services
        -> successor-owned typed post-commit events
        -> successor-owned projection or host adapters
```

The portable core is concentrated in the foundation crates and a small group of voxel/spatial services. The successor should initially port or reference:

- `core-ids`, `core-math`, `core-space`, `core-time`, `core-voxel`, and the small utility portions of `core-assets`, `core-collections`, and `core-error`;
- `svc-volume`, `svc-spatial`, `svc-collision`, `svc-pathfinding`, `svc-rng`, and `svc-mesh`;
- selected algorithms from `svc-levelgen` and `svc-combat`, with successor-owned state and APIs;
- selected entity storage/lifecycle ideas from `core-entity`, but not its movement, replay, or persistence policy;
- selected retained-render and asset/import code only when a concrete successor consumer exists.

The successor should not use these as its center:

- `core-state`, `core-commands`, `core-events`, and the `sim-*` pipeline;
- `rule-gameplay-fabric`, `svc-gameplay-fabric`, `gameplay-module-sdk`, and `gameplay-runtime-host`;
- `rule-project-bundle`, `svc-project-content`, the broad `runtime-bridge-api`, and the native bridge;
- Asha’s universal protocol/code-generation surface, reaction/receipt machinery, and mandatory replay/hash workflow.

This does not mean removing all validation, events, snapshots, or transport. It means keeping those mechanisms local to the successor capability that needs them. Rusty Engine can retain fail-closed decoding, named service input validation, atomic state transitions, save/reopen snapshots, and typed committed events without inheriting a universal Asha `Command -> Validate -> DomainEvent -> Apply -> Replay` control plane.

## Scope and method

The donor snapshot used for this review is Asha Engine commit [`a431974330589761c9e35fc4f8a55996a1b5ee48`](https://github.com/FuzzySlipper/asha-engine/tree/a431974330589761c9e35fc4f8a55996a1b5ee48). The working tree was clean when inspected. The Rust workspace contains 97 crate manifests under [`engine-rs/crates`](../engine-rs/crates); 96 packages are in the default Cargo metadata graph because [`native-bridge`](../engine-rs/crates/bridge/native-bridge) is deliberately excluded from the default workspace. This report covers both.

The review used:

- the workspace manifest and Cargo metadata to enumerate every crate and inspect direct dependency edges;
- each crate’s public module surface, implementation shape, tests, and documentation;
- dependency-hotspot analysis to find crates that pull the runtime/fabric/replay structure into otherwise small features;
- the successor’s current design and migration records in [`/home/dev/rusty-engine/asha-object-centric-successor-spike.md`](../../rusty-engine/asha-object-centric-successor-spike.md), [`migration-cluster-ledger.md`](../../rusty-engine/migration-cluster-ledger.md), [`docs/donor-provenance.md`](../../rusty-engine/docs/donor-provenance.md), and [`docs/experiment-results.md`](../../rusty-engine/docs/experiment-results.md).

The resulting recommendation is about portability and ownership. A `Reference unchanged` row means the code can remain a dependency or close mechanical donor. It does not mean the successor must retain Asha naming or every test fixture. An `Adapt/extract` row means the behavior is valuable, but the successor must own the public types, state, and integration. `Feature later` means there is useful code, but porting it before a real consumer would recreate unnecessary architecture. `Evidence only` means preserve behavioral lessons or tests, not the implementation. `Exclude` means the crate primarily embodies the structural spine or an Asha-specific border.

## What the successor should preserve

### A small object-centric runtime

The current successor direction is a Rust-owned object-centric runtime with explicit entities/data, named services, typed events, data-driven definitions, and thin host/render adapters. That is compatible with the following Asha ideas:

- typed IDs, coordinates, time, math, voxel values, and deterministic collections;
- bounded voxel storage with hidden representation;
- explicit spatial residency and dirty tracking;
- derived collision, navigation, and mesh projections over authoritative voxel data;
- deterministic scoped randomness;
- typed entity tables and explicit lifecycle transitions, after the successor chooses its own state model;
- direct in-process service calls and typed post-commit events.

The successor should not turn those ideas back into a generic ECS, a universal command union, or a giant runtime facade. The target’s own [`engine-spatial`](../../rusty-engine/rust/crates/engine-spatial), [`entity-state`](../../rusty-engine/rust/crates/entity-state), and [`game-host`](../../rusty-engine/rust/crates/game-host) already demonstrate the preferable direction: successor-owned adapters over bounded donors, rather than a dependency on `RuntimeSession`.

### Local validation instead of a universal validation spine

Asha’s validation discipline is worth retaining where it protects a named boundary:

- validate service inputs before mutation;
- make invalid transitions atomic and fail closed;
- validate references and canonical encodings at load/admission boundaries;
- keep derived collision/navigation/mesh data tied to source hashes or source revisions;
- test accepted, rejected, readback, and deterministic behavior.

The part to leave behind is the assumption that every gameplay operation must be represented by a closed global command enum, passed through a universal validator and applier, emitted as a global event batch, recorded as a replay receipt, and routed through a fabric. In the successor, a movement system, voxel editor, combat service, or content loader may each have its own narrow input and event model.

### Snapshots are useful; replay certification is an assurance profile

Save/reopen snapshots can be a normal product capability. Certified replay hashes, reaction frames, decision receipts, and the `sim-*` recorder can remain optional tools for a deterministic-assurance profile. They should not be required for the baseline gameplay path or baked into every entity/service API. This distinction matters because several otherwise portable Asha crates acquire their coupling primarily through `replay_hash`, state hashing, or replay-record types.

### Protocols belong at actual borders

The Asha protocol crates are generated TypeScript contracts, not neutral domain libraries. Rusty Engine should define a smaller successor-owned protocol only when a Rust/TypeScript, renderer, persistence, or external-host boundary actually exists. It should port individual DTO shapes selectively, not the protocol umbrella or the Asha-wide code generator.

## Structural evidence

### Dependency hotspots

Cargo metadata shows that the largest coupling is concentrated in a few public-height crates:

| Crate | Approximate direct internal dependencies | Why this matters |
| --- | ---: | --- |
| `runtime-bridge-api` | 59 | One facade spans runtime sessions, project content, voxel editing, input, render, telemetry, time control, and most protocol/rule families. |
| `rule-project-bundle` | 27 | Project loading, prefab/content admission, serialization, gameplay fabric, and bootstrap are one Asha control plane. |
| `protocol-codegen` | 22 | The generator assumes the entire Asha protocol family and a fixed generated TypeScript destination. |
| `gameplay-runtime-host` | 21 | Runtime lifecycle is coupled to project content, gameplay modules/fabric, scheduler, triggers, authoring, serialization, and receipts. |
| `rule-gameplay-fabric` | 15 | Owner routing, reads, proposals, reactions, facts, and topology define Asha’s gameplay execution model. |
| `svc-project-content` | 13 | Content decoding/compiler/admission carries the project and gameplay provider model into runtime. |
| `render-bridge` | 13 | A valuable renderer projection is nevertheless tied to Asha scene, catalog, level-generation, and protocol surfaces. |

These edges are more informative than line count alone. A large algorithmic crate can still be a good donor when it terminates at foundation/state types. A small crate can be a poor donor when it imports the runtime host, fabric, project bundle, or universal replay model.

### The Asha spine to avoid

The following dependency and behavior chain is the main portability hazard:

```text
project/content admission
  -> gameplay module SDK and provider/owner topology
  -> gameplay fabric / runtime host
  -> universal command, reaction, fact, receipt, and event routing
  -> sim validator/applier/runner/replay
  -> broad RuntimeSession / native bridge / generated protocol facade
```

`core-state`, `core-commands`, `core-events`, and `sim-kernel` make the older authority pipeline explicit. `rule-gameplay-fabric`, `svc-gameplay-fabric`, `gameplay-runtime-host`, and `runtime-bridge-api` make the newer public topology explicit. `protocol-codegen`, `native-bridge`, and the broad project-bundle crates turn that topology into an external contract. Porting any one of these as a foundation tends to pull in the others.

By contrast, the voxel/spatial path terminates cleanly:

```text
core-space / core-voxel
  -> svc-volume
  -> svc-spatial
  -> svc-collision / svc-pathfinding / svc-mesh
  -> successor-owned engine-spatial and projection adapters
```

That closure is why the existing Rusty Engine spatial transplant is a good pattern. The successor’s migration notes also show that direct Rust service calls were preferable to introducing an executable TypeScript runtime, a second authority, or a bridge-heavy comparison harness.

## Porting rules by category

### Reference unchanged

Keep the implementation or a close fork when the crate is a value-level or bounded feature service and does not require Asha runtime ownership. Rename the crate/API only when the Asha name itself would create an unwanted dependency or product claim.

### Adapt/extract

Copy behavior selectively into a successor-owned crate. Replace Asha IDs, state stores, event unions, replay records, and protocol types at the seam. Preserve invariants and focused tests, not the old composition root.

### Feature later

Do not make the crate part of the initial runtime spine. Revisit it when the successor has a named consumer such as dynamic physics, animation, project persistence, mesh import, or voxel authoring. The feature should enter through a small interface owned by that consumer.

### Evidence only

Use the Asha tests, fixtures, invariants, or observed behavior to guide a fresh implementation. Do not make the old crate a dependency merely because it contains a useful test.

### Exclude

Do not port the crate as a unit. It is an Asha-specific structural owner, a broad transport facade, a proof harness, or a tool whose value is inseparable from the old runtime topology.

## Crate-by-crate matrix

The matrix below covers every Rust crate under `engine-rs/crates`. “Port/reference” is intentionally narrow: it means a low-risk donor for the successor’s current direction, not a mandate to preserve all Asha APIs.

### Foundation (7)

| Crate | Recommendation | Reason and successor action |
| --- | --- | --- |
| [`core-assets`](../engine-rs/crates/foundation/core-assets) | Reference unchanged | Typed asset identity/reference values are leaf-level and do not resolve catalogs. Keep the shape; rename only if the successor wants a generic asset namespace. |
| [`core-collections`](../engine-rs/crates/foundation/core-collections) | Reference unchanged | Deterministic sorted/unique helpers are generic utilities with no runtime assumptions. |
| [`core-error`](../engine-rs/crates/foundation/core-error) | Adapt/extract | The error categories and fail-closed result style are useful, but `AshaError` naming and categories should become successor-owned. |
| [`core-ids`](../engine-rs/crates/foundation/core-ids) | Reference unchanged | Typed numeric IDs are a clean identity substrate. Add successor IDs only when a real domain needs them. |
| [`core-math`](../engine-rs/crates/foundation/core-math) | Reference unchanged | Small value types and deterministic math have no Asha control-plane dependency. |
| [`core-space`](../engine-rs/crates/foundation/core-space) | Reference unchanged | Voxel/chunk/world coordinate types are a strong boundary for spatial services. Keep coordinate semantics and avoid adding world/session policy. |
| [`core-time`](../engine-rs/crates/foundation/core-time) | Reference unchanged | `Tick`, delta, and interval values are neutral. Do not import Asha’s time-control protocol or simulation runner with them. |

### State (9)

| Crate | Recommendation | Reason and successor action |
| --- | --- | --- |
| [`core-catalog`](../engine-rs/crates/state/core-catalog) | Feature later | Asset dependency DAGs, locks, material authority/style separation, and revalidation are useful only after successor asset schemas settle. Port the catalog rules behind a new loader. |
| [`core-commands`](../engine-rs/crates/state/core-commands) | Exclude | Its closed `Command`/`CommandKind` union is the old universal authority input boundary and yields Asha `DomainEvent`s. Define narrow successor service inputs instead. |
| [`core-entity`](../engine-rs/crates/state/core-entity) | Adapt/extract | Typed capability tables, minimal identity/lifecycle, and explicit lifecycle commands are useful. Rewrite movement ownership, tombstone policy, persistence, and replay hashing around `rusty-engine`’s `entity-state`. |
| [`core-events`](../engine-rs/crates/state/core-events) | Exclude | The closed universal `DomainEvent` and ordered `EventBatch` encode the Asha validator/applier pipeline. Use named typed post-commit events owned by successor services. |
| [`core-game-rules`](../engine-rs/crates/state/core-game-rules) | Feature later | Modifiers, durations, stack policies, reactions, and cadence are feature primitives, but they are not a neutral runtime spine. Reimplement or extract them when a rules consumer exists. |
| [`core-scene`](../engine-rs/crates/state/core-scene) | Adapt/extract | Authored scene documents, canonical flattening, and reference validation are useful. Replace Asha bootstrap records, `SpatialSessionState`, and replay-unit assumptions with a successor content loader. |
| [`core-snapshot`](../engine-rs/crates/state/core-snapshot) | Feature later | Canonical snapshots and hashes can support save/reopen or an optional assurance profile. Do not impose `StateStore` or mandatory replay hashes on the baseline runtime. |
| [`core-state`](../engine-rs/crates/state/core-state) | Exclude | The abstract Entity/Subject/Process/Mode/Signal/Tag `StateStore` is the older generic simulation model and would compete with the successor’s object-centric state. |
| [`core-voxel`](../engine-rs/crates/state/core-voxel) | Reference unchanged | Neutral voxel values/material classification intentionally avoid product materials, renderer, storage, and generation. This is one of the strongest donors. |

### Protocol (23)

| Crate | Recommendation | Reason and successor action |
| --- | --- | --- |
| [`protocol-assets`](../engine-rs/crates/protocol/protocol-assets) | Feature later | Asset/catalog DTOs encode Asha authority/style and catalog conventions. Select fields only when a successor asset border exists. |
| [`protocol-codegen`](../engine-rs/crates/protocol/protocol-codegen) | Exclude | It scans the whole Asha protocol family and writes to the fixed TypeScript contracts package. Build a smaller successor generator only if generated contracts are actually needed. |
| [`protocol-diagnostics`](../engine-rs/crates/protocol/protocol-diagnostics) | Feature later | Severity/report shapes are reusable, but scopes and codes assume Asha scene/project/render composition. Trim the vocabulary around successor diagnostics. |
| [`protocol-entity-authoring`](../engine-rs/crates/protocol/protocol-entity-authoring) | Feature later | EntityDefinition and capability authoring DTOs are useful after the successor’s stored definitions are settled; do not port the Asha admission envelope wholesale. |
| [`protocol-game-extension`](../engine-rs/crates/protocol/protocol-game-extension) | Exclude | Module manifests, providers, owners, proposals, and receipts are the gameplay-extension topology the successor is intentionally avoiding. |
| [`protocol-game-rules`](../engine-rs/crates/protocol/protocol-game-rules) | Exclude | The catalog and reaction wire model assumes Asha game-rule modules and universal rule routing. |
| [`protocol-ids`](../engine-rs/crates/protocol/protocol-ids) | Adapt/extract | Border ID DTOs can be selected when a real successor transport exists. Keep domain IDs in successor Rust rather than importing the complete Asha list. |
| [`protocol-input`](../engine-rs/crates/protocol/protocol-input) | Evidence only | Input catalogs, context stacks, and envelopes are coupled to Asha `RuntimeSession` and browser catalog lifecycle. Rebuild a smaller controller/input model if needed. |
| [`protocol-policy-view`](../engine-rs/crates/protocol/protocol-policy-view) | Exclude | It exposes policy-world projections, command validation, and replay records for the Asha policy boundary. |
| [`protocol-presentation`](../engine-rs/crates/protocol/protocol-presentation) | Feature later | Audio, billboard, particle, animation, and telemetry operations can be selected per output family after the successor projection border is defined. |
| [`protocol-project-bundle`](../engine-rs/crates/protocol/protocol-project-bundle) | Exclude | The manifest/load-plan/prefab/session bundle is a broad Asha project contract rather than a portable runtime primitive. |
| [`protocol-project-content`](../engine-rs/crates/protocol/protocol-project-content) | Exclude | Content admission and gameplay-provider contracts carry the project-content control plane into the wire border. |
| [`protocol-render`](../engine-rs/crates/protocol/protocol-render) | Adapt/extract | The retained-mode render-diff idea is useful, especially with the retained Three renderer, but the successor should own a smaller render-frame contract. |
| [`protocol-replay`](../engine-rs/crates/protocol/protocol-replay) | Evidence only | Replay records, steps, hashes, and snapshots are optional assurance artifacts. Do not make them a required gameplay protocol. |
| [`protocol-scene`](../engine-rs/crates/protocol/protocol-scene) | Feature later | Stored scene DTOs can be useful for authoring, but they belong behind a successor loader and should not imply Asha project/session bootstrap. |
| [`protocol-script`](../engine-rs/crates/protocol/protocol-script) | Exclude | The script-policy runtime border introduces a second execution authority and its associated command/replay contracts. |
| [`protocol-telemetry`](../engine-rs/crates/protocol/protocol-telemetry) | Feature later | Readout and telemetry can be added at a host/tool border; they should not enter the core runtime model. |
| [`protocol-time-control`](../engine-rs/crates/protocol/protocol-time-control) | Exclude | External time-control envelopes belong to Asha’s simulation/bridge workflow, not the successor’s local runtime core. |
| [`protocol-view`](../engine-rs/crates/protocol/protocol-view) | Evidence only | Camera handles/pose/projection are useful concepts, but the Asha FPS view contract should not become a runtime authority border. Derive camera state from accepted successor pose. |
| [`protocol-voxel-annotation`](../engine-rs/crates/protocol/protocol-voxel-annotation) | Feature later | Annotation layers are a useful authoring format if the successor adopts voxel annotation; they are not baseline runtime state. |
| [`protocol-voxel-asset`](../engine-rs/crates/protocol/protocol-voxel-asset) | Feature later | Canonical voxel asset DTOs can be adapted once the successor chooses an asset format and ownership boundary. |
| [`protocol-voxel-conversion`](../engine-rs/crates/protocol/protocol-voxel-conversion) | Feature later | Conversion plans/previews are useful tooling only when mesh/voxel authoring has a consumer; keep them outside runtime authority. |
| [`protocol-voxel-edit-history`](../engine-rs/crates/protocol/protocol-voxel-edit-history) | Feature later | Edit history is useful for authoring/undo, but its command/event/persistence form should be successor-owned and optional. |

### Simulation (5)

| Crate | Recommendation | Reason and successor action |
| --- | --- | --- |
| [`sim-applier`](../engine-rs/crates/sim/sim-applier) | Exclude | Applies the universal Asha `DomainEvent` union to `core-state`; it is inseparable from the old authority model. |
| [`sim-kernel`](../engine-rs/crates/sim/sim-kernel) | Exclude | Its CollectInput -> Validate -> AccumulateEvents -> ApplyEvents -> Snapshot phases define the structural spine the successor is avoiding. Preserve the idea of explicit phases only where a named successor system needs it. |
| [`sim-replay`](../engine-rs/crates/sim/sim-replay) | Evidence only | Deterministic encoding/diff is useful for a later assurance tool, but it should consume successor snapshots/events rather than become a runtime dependency. |
| [`sim-runner`](../engine-rs/crates/sim/sim-runner) | Exclude | SimulationAuthority, recorder/playback, and time control make replayable simulation the center of execution. |
| [`sim-validator`](../engine-rs/crates/sim/sim-validator) | Exclude | It validates the closed old command set into old domain events. Replace with local validation at each successor service boundary. |

### Services (20)

| Crate | Recommendation | Reason and successor action |
| --- | --- | --- |
| [`svc-collision`](../engine-rs/crates/services/svc-collision) | Reference unchanged | A bounded Parry-backed projection over authoritative voxel data, with typed coordinates and source-chunk provenance, is a strong donor. Keep raw Parry behind this service. |
| [`svc-combat`](../engine-rs/crates/services/svc-combat) | Adapt/extract | The slab-ray/nearest-target algorithm is useful. Do not import its `CombatState`, fire-control command, health/replay hashes, or Asha combat events; let the successor own combat state. |
| [`svc-entity-authoring`](../engine-rs/crates/services/svc-entity-authoring) | Feature later | Fail-closed authoring validation is useful, but the crate is tied to ECRP/protocol/entity-definition and project-bundle admission. Extract validation patterns after successor definitions settle. |
| [`svc-environment-authoring`](../engine-rs/crates/services/svc-environment-authoring) | Feature later | Procedural materialization into scene/voxel artifacts is valuable tooling, but its Asha provenance, levelgen, serialization, and protocol dependencies make it a later content-loader feature. |
| [`svc-game-rules`](../engine-rs/crates/services/svc-game-rules) | Exclude | Reaction validation/resolution and rule catalogs reinforce the gameplay-fabric control plane. Rebuild specific mechanics later if required. |
| [`svc-gameplay-fabric`](../engine-rs/crates/services/svc-gameplay-fabric) | Exclude | Immutable fabric contracts, codecs, session topology, providers, and owner graphs are central Asha structure even though this crate does not itself mutate state. |
| [`svc-levelgen`](../engine-rs/crates/services/svc-levelgen) | Adapt/extract | Reuse deterministic generation algorithms and perhaps config ideas. Exclude its event/replay/hash control plane, runtime frames, render chunks, and spawn-marker integration; return successor-owned artifacts. |
| [`svc-mesh`](../engine-rs/crates/services/svc-mesh) | Reference unchanged | Deterministic visible-face meshing is chunk-local and renderer-neutral. Consume it through a successor projection adapter. |
| [`svc-mesh-import`](../engine-rs/crates/services/svc-mesh-import) | Feature later | Bounded GLB/static mesh parsing and provenance hashing are good offline tooling, but the successor must choose its asset/content schema and lifecycle. |
| [`svc-pathfinding`](../engine-rs/crates/services/svc-pathfinding) | Reference unchanged | Read-only deterministic navigation over voxel authority has no AI, policy, movement, renderer, or demo ownership. Put navigation policy and durable intent in the successor. |
| [`svc-physics`](../engine-rs/crates/services/svc-physics) | Feature later | The deterministic kinematic integrator is small and clean, but the current migration ledger correctly keeps it as evidence until a concrete dynamic-physics consumer exists. |
| [`svc-policy-view`](../engine-rs/crates/services/svc-policy-view) | Exclude | Policy-world projection, command validation, and replay records are a second authority/protocol boundary. |
| [`svc-project-content`](../engine-rs/crates/services/svc-project-content) | Exclude | This is a large content decoder/compiler/admission surface with a static gameplay-provider requirement. It would recreate the Asha project/runtime boundary. |
| [`svc-rng`](../engine-rs/crates/services/svc-rng) | Reference unchanged | Explicitly scoped deterministic SplitMix64 avoids wall-clock, global, and platform RNG. It is a clean service donor. |
| [`svc-serialization`](../engine-rs/crates/services/svc-serialization) | Feature later | Canonical project-bundle save/load/compaction is useful only after the successor persistence model exists. Adapt encoding ideas narrowly, not the bundle control plane. |
| [`svc-spatial`](../engine-rs/crates/services/svc-spatial) | Reference unchanged | Explicit chunk residency and dirty tracking over a bounded authority are a strong fit for the successor’s spatial adapter. |
| [`svc-volume`](../engine-rs/crates/services/svc-volume) | Reference unchanged | Hidden bounded chunk storage is a neutral substrate and should remain below world partition/generation/meshing policy. |
| [`svc-voxel-annotation`](../engine-rs/crates/services/svc-voxel-annotation) | Feature later | Validation, canonicalization, hashing, and query are useful for a future authoring tool, not baseline runtime state. |
| [`svc-voxel-asset`](../engine-rs/crates/services/svc-voxel-asset) | Feature later | Canonical Asha voxel asset encoding/decoding is a possible donor once the successor chooses whether to adopt that format. |
| [`svc-voxel-conversion`](../engine-rs/crates/services/svc-voxel-conversion) | Feature later | Mesh/voxel conversion plans and previews should remain a bounded authoring/import feature with successor-owned diagnostics and DTOs. |

### Rules (15)

| Crate | Recommendation | Reason and successor action |
| --- | --- | --- |
| [`game-rule-extension`](../engine-rs/crates/rules/game-rule-extension) | Exclude | Public game-owned module traits, manifests, proposals, and receipts are precisely the extensibility/provider topology the successor is avoiding. |
| [`gameplay-module-sdk`](../engine-rs/crates/rules/gameplay-module-sdk) | Exclude | Static module authoring/composition/bindings and generated rule contracts create a second game-code authority. |
| [`gameplay-runtime-host`](../engine-rs/crates/rules/gameplay-runtime-host) | Exclude | `RuntimeSession` is a public-height host over project content, fabric, scheduler, triggers, authoring, serialization, and decision receipts. It should not be the successor center. |
| [`rule-animation-controller`](../engine-rs/crates/rules/rule-animation-controller) | Feature later | The deterministic graph/parameter/transition/timing model is a potential presentation feature. Adapt it above successor events and remove replay/persistence assumptions until those are needed. |
| [`rule-game-modifier`](../engine-rs/crates/rules/rule-game-modifier) | Feature later | Modifier catalogs, state, rejection, and traces can support a real rules consumer later; do not import its game-rule/replay hash model into the core. |
| [`rule-gameplay-fabric`](../engine-rs/crates/rules/rule-gameplay-fabric) | Exclude | Owner routing, reads, reactions, proposals, facts, and session topology define Asha’s gameplay execution model. |
| [`rule-input`](../engine-rs/crates/rules/rule-input) | Evidence only | Catalog/context-stack resolution is useful behavior evidence, but the implementation assumes Asha input catalogs and `RuntimeSession` lifecycle. |
| [`rule-lifecycle`](../engine-rs/crates/rules/rule-lifecycle) | Exclude | FPS roles, weapon mounts, project-bundle loading, authoring, fabric, combat, and pathfinding make this a product-specific lifecycle composition. |
| [`rule-process`](../engine-rs/crates/rules/rule-process) | Evidence only | Process lifecycle is implemented over the old command/event/state model. Rebuild only if the successor actually adopts processes, with local types. |
| [`rule-project-bundle`](../engine-rs/crates/rules/rule-project-bundle) | Exclude | Project load, prefabs, content admission, bootstrap, serialization, and fabric form a broad Asha composition root. |
| [`rule-relationship`](../engine-rs/crates/rules/rule-relationship) | Adapt/extract | Explicit relationship request/preview/readout/apply behavior is a useful feature. Replace Asha entity/readout types with successor-owned relationship state. |
| [`rule-scheduler`](../engine-rs/crates/rules/rule-scheduler) | Exclude | Gameplay action scheduling, event-conditioned actions, and registry/fabric ownership assume the Asha execution topology. |
| [`rule-state-machine`](../engine-rs/crates/rules/rule-state-machine) | Adapt/extract | The explicit state machine spec/instance/transition model is a clean feature primitive. Use successor errors/events and keep it local to its owner. |
| [`rule-trigger-volume`](../engine-rs/crates/rules/rule-trigger-volume) | Feature later | Kinematic overlap/reconcile logic is useful, but trigger snapshots, owner IDs, and facts should be successor-owned when trigger gameplay exists. |
| [`rule-voxel-edit`](../engine-rs/crates/rules/rule-voxel-edit) | Feature later | Picking, transactions, edit history, and chunk generation have value for authoring. Extract algorithms only; do not import its old command/event/scene/replay composition. |

### Render (7)

| Crate | Recommendation | Reason and successor action |
| --- | --- | --- |
| [`render-animation`](../engine-rs/crates/render/render-animation) | Feature later | One-way animation projection is a useful presentation donor when the successor has animation authority. Keep presentation handles outside gameplay authority. |
| [`render-audio`](../engine-rs/crates/render/render-audio) | Feature later | Audio projection can be ported per output family after a successor presentation border exists. |
| [`render-billboard`](../engine-rs/crates/render/render-billboard) | Feature later | Billboard projection is feature-level and should not pull in the whole render bridge. |
| [`render-bridge`](../engine-rs/crates/render/render-bridge) | Adapt/extract | Retained render handles, render diffs, voxel projection, and mesh payloads are valuable. Extract a smaller successor projection contract instead of importing its scene/catalog/levelgen/protocol closure. |
| [`render-debug`](../engine-rs/crates/render/render-debug) | Evidence only | It reads Asha `core-state`/render-bridge structures and is tool-specific. Rebuild diagnostics around successor projections. |
| [`render-particle`](../engine-rs/crates/render/render-particle) | Feature later | Particle projection is a presentation donor only after a real consumer exists. |
| [`render-telemetry-overlay`](../engine-rs/crates/render/render-telemetry-overlay) | Feature later | Useful as a tool/overlay, not as runtime state or an authority dependency. |

### WASM and bridge (3, counting the excluded native package)

| Crate | Recommendation | Reason and successor action |
| --- | --- | --- |
| [`wasm-api`](../engine-rs/crates/wasm/wasm-api) | Exclude | It is a narrow replay-divergence/label helper and has no baseline successor role. Rebuild a WASM seam only if a concrete browser use case requires one. |
| [`runtime-bridge-api`](../engine-rs/crates/bridge/runtime-bridge-api) | Exclude | The 59-edge facade combines runtime sessions, authoring, voxel, input, render, telemetry, and time-control. It is the opposite of a small successor border. |
| [`native-bridge`](../engine-rs/crates/bridge/native-bridge) | Exclude | N-API operation tables, wire JSON, handle registries, and native toolchain assumptions implement the Asha bridge surface. The successor should add a smaller host bridge only after its Rust API is stable. |

### Tools (8)

| Crate | Recommendation | Reason and successor action |
| --- | --- | --- |
| [`asset-import`](../engine-rs/crates/tools/asset-import) | Feature later | Offline deterministic GLB/static-mesh import and artifact generation are good tooling donors, but not runtime dependencies. |
| [`fixture-maker`](../engine-rs/crates/tools/fixture-maker) | Exclude | It generates Asha service/rule fixtures and goldens tied to the old dependency graph. Recreate focused successor fixtures. |
| [`protocol-dump`](../engine-rs/crates/tools/protocol-dump) | Exclude | It is an inspection companion to Asha’s whole-protocol code generator. Rebuild only if the successor adopts generated contracts. |
| [`replay-tool`](../engine-rs/crates/tools/replay-tool) | Evidence only | Keep the CLI concept for an optional assurance profile, but make it consume successor snapshots/events rather than Asha replay records. |
| [`scene-diagnostics`](../engine-rs/crates/tools/scene-diagnostics) | Feature later | Diagnostics are useful, but rewrite them around successor content/state types rather than project-bundle/session state. |
| [`snapshot-diff`](../engine-rs/crates/tools/snapshot-diff) | Evidence only | Snapshot diffing can support save/reopen and optional determinism checks after successor snapshot semantics exist. |
| [`state-inspector`](../engine-rs/crates/tools/state-inspector) | Feature later | The inspector is useful as a tool, but its old `core-entity` view should be rewritten for `entity-state`. |
| [`voxel-diagnostics`](../engine-rs/crates/tools/voxel-diagnostics) | Feature later | Voxel diagnostics can be retained as a focused tool after successor ownership of voxel edits, scheduling, and rejection reporting is clear. |

## Outside `engine-rs/crates`: do not port the public facades

The repository also contains public Rust surfaces and fixtures that are not counted in the 97 crate matrix. They are important to call out because copying them would silently reintroduce the same topology through a different path.

| Surface | Recommendation | Reason |
| --- | --- | --- |
| [`public-rust/game-rule-extension`](../public-rust/game-rule-extension) | Exclude | Quarantined legacy extension surface. |
| [`public-rust/gameplay-module-conformance`](../public-rust/gameplay-module-conformance) | Exclude | Proof/conformance harness tied to gameplay fabric and project bundle. |
| [`public-rust/gameplay-module-sdk`](../public-rust/gameplay-module-sdk) | Exclude | Facade for the Asha module/provider model. |
| [`public-rust/gameplay-runtime-host`](../public-rust/gameplay-runtime-host) | Exclude | Quarantined `RuntimeSession` facade. |
| [`public-rust/runtime-session-composition`](../public-rust/runtime-session-composition) | Exclude | Broad runtime-bridge composition facade. |
| [`public-rust/native-runtime-provider`](../public-rust/native-runtime-provider) | Exclude | Native bridge/provider surface. |
| [`harness/fixtures`](../harness/fixtures) | Evidence only | Downstream/proof fixtures can preserve expected behavior, but they are not runtime donors. |

## Suggested extraction order

The existing Rusty Engine migration ledger is directionally correct. A safe order is:

1. **Keep the value substrate small.** Use or fork IDs, math, space, time, voxel values, collections, errors, and deterministic RNG. Avoid adding a new umbrella crate just to preserve Asha’s module layout.

2. **Finish the successor state ownership.** Let `entity-state` define entity identity, lifecycle, component/data tables, and snapshot decisions. If `core-entity` contributes code, extract typed-table and lifecycle mechanics after removing Asha movement/replay/persist assumptions.

3. **Keep the spatial closure.** Use `svc-volume`, `svc-spatial`, `svc-collision`, `svc-pathfinding`, and `svc-mesh` behind `engine-spatial`-style successor adapters. The successor should own navigation policy, durable intent, movement integration, and projection scheduling.

4. **Add generation as a feature service.** Extract deterministic portions of `svc-levelgen`, adapting output into successor-owned voxel/artifact types. Do not import generation records, runtime frames, event batches, or render summaries merely because the donor produces them.

5. **Add gameplay features one at a time.** For combat, start with the ray/collision algorithm and define successor health/weapon state. For relationships, state machines, triggers, animation, or modifiers, import one feature behind one named owner. Avoid a general gameplay fabric or universal action scheduler.

6. **Define persistence from the successor model.** Add snapshots/save-reopen when the state shape is real. Add canonical hashes, replay records, or diff tools only if an operational need earns them; keep them as optional assurance consumers.

7. **Add authoring/import and render borders last.** Adapt scene/content, mesh import, voxel conversion, and render-diff code only when the successor’s stored content and projection contracts are stable. Generated TS should describe those actual borders, not dictate the runtime shape.

## Porting guardrails

Every proposed donor should pass these checks before entering `rusty-engine`:

- Does its normal dependency closure terminate in values, state, and bounded services, or does it reach gameplay fabric, runtime host, project bundle, or replay infrastructure?
- Does it own a concrete successor feature, or is it being imported because Asha already has a similarly named crate?
- Can the feature accept and return successor-owned types without importing `core-commands`, `core-events`, `RuntimeSession`, `GameplayFabric`, or Asha protocol envelopes?
- Is validation local to the service that owns the invariant?
- Are events typed and owned by the feature, rather than inserted into a universal event union or ambient bus?
- Does the implementation require replay receipts, hashes, or decision frames in its normal path? If yes, make it optional or extract the underlying algorithm.
- Does a protocol exist because a real external border exists, or merely because Asha generated one?
- Can the behavior be tested as a direct Rust service call with accepted/rejected/readback assertions before adding a host or browser bridge?
- Is the proposed copied code smaller than the structural dependency it would bring with it?

A practical dependency budget follows from this review: the initial successor should remain in the low single-digit crate count for its runtime core, growing by feature clusters. It should not recreate Asha’s roughly hundred-crate workspace as a prerequisite for a playable or testable slice.

## Final disposition

Port the boring parts first: values, coordinates, voxels, bounded storage, spatial residency, collision, navigation, meshing, RNG, and carefully selected algorithms. Adapt entity/lifecycle, scene loading, generation, combat, and rendering only behind successor-owned seams. Treat authoring, persistence, animation, and asset tooling as later feature clusters. Use Asha’s tests and invariants as evidence where the implementation is coupled.

Do not port the Asha structural spine. In particular, do not make `core-state`, the universal command/event/sim pipeline, gameplay fabric, `RuntimeSession`, project-bundle admission, broad protocol generation, or the native bridge the successor’s organizing principle. The successor should own a smaller Rust runtime whose services can be called directly, whose state is concrete and inspectable, and whose optional network/browser/protocol workflow is added at the edge after the core behavior is sound.
