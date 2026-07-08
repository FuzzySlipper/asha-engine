# Agent-Legible Engine System Design

**Name:** ASHA — Agent Safety & Harness Architecture
**Status:** Revised infrastructure-first design  
**Scope:** Project-private engine infrastructure, not a generic public framework  
**Primary goal:** Make a complex simulation/rendering codebase maintainable under high fan-out agent development  
**Non-goal:** Solve specific product-domain, authored-content, or state-modeling problems during infrastructure-first work

> **Current-status note (2026-07):** this is the canonical repository architecture/design baseline, not the live implementation queue. Some package names and the prototype phase plan below are historical. Use `README.md` for repository orientation and Den guidance/tasks/docs/messages for current implementation status.

---

## 1. Executive summary

This engine is designed for a development model where the human stays mostly at the PRD, architecture, review, and orchestration layer while many ephemeral coding agents perform implementation work inside constrained lanes.

The design therefore optimizes for **hard boundaries, compiler-checkable contracts, deterministic tests, generated protocol surfaces, and machine-reviewable dependency rules**. Traditional human-team velocity assumptions do not apply directly. A structure that would feel bureaucratic for a small human team can be excellent for agent-driven development because review, linting, test repair, contract checking, and local implementation can all be parallelized.

The core architectural split is:

> **Rust owns authority. TypeScript owns expression and projection. Generated contracts define the border.**

Rust is the authoritative compiled core: canonical state, validation, event application, deterministic services, replay, serialization, and heavy simulation. TypeScript is split into two sharply separated domains: constrained policy/catalog packages that propose commands, and a shell/render/UI layer that displays projected state and collects user, tool, or harness input.

TypeScript does not mutate authoritative state. It may read generated, read-only projections and return proposed commands. Rust validates those commands, converts accepted commands into domain events, applies those events to the canonical state store, and emits render diffs or telemetry as projections.

The entire repository is shaped so that crates and packages become **agent assignment cells**. A worker receives a lane, an allowed dependency set, and a local test surface. Watchers verify import rules, public API drift, deterministic replay, generated contract consistency, lint tripwires, and golden fixtures.

---

## 2. Design principles

### 2.1 Authority is centralized, expression is constrained

There is one authoritative state owner: the Rust core. Other layers may propose, display, inspect, or author catalogs, but they do not become parallel truth systems.

The engine distinguishes between:

- **Authoritative state:** canonical state stored and mutated in Rust.
- **Read-only views:** generated projections exposed to TypeScript policy and tooling.
- **Proposed commands:** requests from TypeScript policy, UI, tools, input systems, or external harnesses.
- **Accepted domain events:** validated state changes applied by Rust.
- **Render diffs:** display projections emitted by Rust and consumed by the renderer.
- **Telemetry events:** observations for debugging, tracing, and tooling.

These categories must never collapse into one generic event bus.

### 2.2 Contracts are borders

The Rust/TypeScript boundary is not an informal interface. It is a generated contract surface.

Rust protocol crates define the canonical schema. TypeScript contract packages are generated from those Rust definitions. Generated TypeScript files may be committed for worker convenience, but they are not hand-edited.

A contract change is treated as a border change. It requires protocol review, generated fixture updates, compatibility notes when applicable, and downstream typecheck/test runs.

### 2.3 Crates and packages are assignment cells

A crate or package is not just an organizational folder. It is the unit of agent assignment.

An agent can be told:

> You own this crate/package. You may depend only on these crates/packages. You may not alter generated contracts. You may not cross this lane. Your tests and fixtures live here. Your public API changes must be explained.

The compiler, package manager, dependency graph checker, lint rules, and review watchers enforce the assignment.

### 2.4 Boring architecture beats clever framework magic

The engine should prefer explicit Rust libraries and simple TypeScript packages over framework inversion-of-control, hidden scheduling, macro-heavy behavioral magic, or generic plugin systems.

The goal is not to become a public general-purpose engine. The goal is to build project-specific infrastructure that agents can understand, test, and modify without smuggling product-domain assumptions into the foundation.

### 2.5 Infrastructure first, catalogs later

Early work should build maintainable, testable capabilities:

- state storage
- typed IDs
- event application
- command validation
- replay
- deterministic services
- generated contracts
- policy sandboxing
- render diffs
- diagnostics
- dependency enforcement
- fixture generation

It should not introduce specific product concepts, domain rules, or authored content beyond minimal abstract fixtures needed to prove the machinery.

---

## 3. Layer model

| Layer | Primary language | Owns | Must not own |
|---|---:|---|---|
| Authority core | Rust | canonical state, validation, event application, replay, serialization, deterministic services | UI, renderer behavior, policy authorship, visual-only effects |
| Protocols | Rust source → generated TypeScript | command/view/event/diff schemas, type generation, compatibility fixtures | product-domain logic, renderer logic, ad hoc convenience APIs |
| Policy/catalog layer | TypeScript | authored policy, data catalogs, high-level command proposals | authoritative mutation, DOM, renderer, WASM memory, wall-clock randomness |
| Script host | TypeScript | loading policy packs, sandboxing, deterministic invocation, command collection | command validation, canonical event application |
| WASM bridge | TypeScript | WASM loading, memory views, protocol encode/decode | product-domain decisions, renderer-specific behavior |
| Renderer | TypeScript + Three.js | scene projection from render diffs | authoritative state, policy, validation |
| UI shell | TypeScript + DOM | panels, input collection, inspectors, user-facing controls | direct state mutation, hidden state model |
| Cosmetic layer | TypeScript + renderer runtime | visual-only particles, screen effects, non-authoritative animation | replay truth, simulation authority |
| Wrapper | Electron | process/window/platform integration | product-domain logic, policy, renderer decisions |
| Devtools | TypeScript and/or Rust | inspection, replay viewing, debug workflows, fixture generation | runtime authority |

A compact version:

> Rust decides what is true. TypeScript policy proposes what might happen. TypeScript shell shows what Rust says happened.

---

## 4. Language strategy

### 4.1 Rust as authority substrate

Rust is chosen for the authoritative side because it makes many classes of agent mistakes fail before runtime:

- ownership and borrowing errors
- missing match variants
- null-like absence handled through `Option`
- explicit error paths through `Result`
- crate privacy and package-level dependency boundaries
- structured compiler diagnostics usable by orchestration tools

This is not primarily a “Rust is fast” argument. The stronger claim is that Rust is a good governance substrate for agent-written compiled code.

The desired failure mode is:

> The agent cannot compile the wrong thing.

Not:

> The agent ships plausible runtime behavior that later needs a human archaeologist.

### 4.2 TypeScript as expression substrate

TypeScript remains valuable even in an agent-driven workflow. Its ergonomics are not merely human-centric. They make high-churn policy, content, test fixtures, tools, UI, and render glue cheaper to specify, regenerate, inspect, and review.

TypeScript should be used for:

- policy packs that return proposed commands
- catalog definitions
- UI and tooling
- renderer projection
- bridge code
- fixture authoring
- debug dashboards
- replay viewers

TypeScript should not become a second engine. It needs strict rails: strict compiler options, project references, package exports, dependency graph checks, custom lint rules, generated contract types, sandbox restrictions, and deterministic test fixtures.

### 4.3 The authority/expression boundary

The core boundary rule:

> TypeScript can propose commands. Rust validates commands and applies accepted events.

TypeScript policy receives generated read-only views:

```ts
export function runPolicy(view: PolicyView): PolicyCommand[] {
  const commands: PolicyCommand[] = [];

  if (view.signals.someSignal > view.thresholds.someThreshold) {
    commands.push({
      kind: "RequestStateChange",
      target: view.subject.id,
      requestedMode: "ExampleMode",
    });
  }

  return commands;
}
```

Rust treats the result as a request:

```rust
pub fn validate_policy_command(
    state: &StateStore,
    command: PolicyCommand,
) -> Result<Vec<DomainEvent>, RejectionReason> {
    match command {
        PolicyCommand::RequestStateChange(cmd) => validate_state_change(state, cmd),
        PolicyCommand::RequestProcessStart(cmd) => validate_process_start(state, cmd),
    }
}
```

The policy code is expressive. The Rust validator is sovereign.

---

## 5. Architectural model

### 5.1 Storage: entity IDs and arenas

The engine uses central storage with typed, copyable IDs.

- `StateStore` owns authoritative state.
- Entities are referenced by typed IDs, not pointers.
- Cross-entity references use generated or hand-defined handle types.
- Deletion and stale references are handled as lookup failures, not memory safety questions.
- Serialization and replay operate over explicit IDs and events.

This adopts the useful storage side of ECS without adopting dogmatic ECS execution.

### 5.2 Execution: services, commands, events

The default execution model is explicit and boring:

1. Read state.
2. Compute proposed changes.
3. Validate commands.
4. Emit domain events.
5. Apply events in a controlled mutation phase.
6. Project render diffs and telemetry.

Services are plain Rust modules/functions over explicit state access. They are not hidden framework systems.

A service may perform hot iteration over storage when needed, but iteration-heavy internals do not turn the whole engine into a framework-driven ECS.

### 5.3 Read-propose-validate-apply-project

The complete tick model:

```txt
Inputs / tools / policy packs
  ↓
Proposed commands
  ↓
Rust validation
  ↓
Accepted domain events
  ↓
Sequential event application
  ↓
Updated authoritative StateStore
  ↓
Render diffs + telemetry + replay records
```

This model keeps mutation centralized and makes each stage testable.

### 5.4 Event taxonomy

Use separate types and queues for separate meanings.

| Type | Meaning | Authority level |
|---|---|---:|
| `InputCommand` | request from user input, tools, replay, or external host | proposed |
| `PolicyCommand` | request from constrained TypeScript policy | proposed |
| `SystemCommand` | request from Rust service orchestration | proposed/internal |
| `DomainEvent` | accepted state change | authoritative |
| `RenderDiff` | projection for renderer | non-authoritative |
| `TelemetryEvent` | observation/log/trace | non-authoritative |
| `ReplayRecord` | audit artifact for deterministic regression | verification |

No generic `Event` type should absorb all of these.

### 5.5 Replay as audit bureaucracy

Replay is not a feature bolted on later. It is the core audit mechanism for agent-written changes.

The engine should support:

- recording proposed commands
- recording accepted events
- recording state hashes at deterministic intervals
- snapshotting state
- diffing replay divergence
- running replay tests headlessly
- producing compact diagnostics suitable for repair agents

During development, it is useful to record both proposed commands and accepted events. For long-term golden regressions, accepted events plus snapshots/hashes are usually the stronger authority.

### 5.6 Determinism stance

The canonical replay target should be explicit.

Recommended default:

> Shipping WASM semantics are the replay authority. Native builds are useful for tools and fast iteration, but canonical golden replays should include a WASM path.

If native and WASM are both supported for replay, divergence must be classified and tested intentionally rather than discovered accidentally.

All authoritative randomness must come from deterministic engine services. Policy code may receive deterministic random streams only through explicit inputs. Wall-clock time, ambient randomness, global mutable state, network calls, filesystem calls, and DOM access are forbidden inside policy execution.

---

## 6. Boundary protocols

### 6.1 Protocol families

Split protocols by meaning instead of making one giant boundary package.

#### `protocol-script`

Defines what policy/content code can see and say.

```txt
StateStore → generated read-only views → TypeScript policy → proposed commands → Rust validator
```

Contains:

- policy views
- command variants
- rejection reasons
- script execution metadata
- deterministic input envelopes

#### `protocol-render`

Defines retained-mode scene diffs.

```txt
Rust render bridge → render diffs → TypeScript bridge → renderer
```

Contains:

- render handles
- create/update/destroy messages
- transforms
- material references
- mesh payload descriptors
- debug overlay descriptors

Renderer code consumes this protocol. It does not inspect `StateStore`.

#### `protocol-replay`

Defines replay files, replay steps, hashes, snapshots, divergence reports, and compatibility metadata.

#### `protocol-telemetry`

Defines structured logs, traces, counters, spans, and diagnostic messages.

Telemetry is observational. It must not become authority event machinery.

### 6.2 Retained-mode rendering

The render boundary should be retained-mode, not immediate-mode.

Rust emits diffs:

```txt
create handle
update transform
replace mesh payload
set visibility
destroy handle
emit debug overlay
```

It does not send “everything to draw this frame.”

This keeps traffic small, makes renderer tests fixture-friendly, and gives agents a simple vocabulary to reason about.

### 6.3 Large payloads

Large geometry or buffer payloads should travel through explicit memory handles or pointer/length-style bridge APIs rather than bloating structured messages.

Rules:

- structured protocol carries small metadata
- large payloads use stable bridge-owned memory views
- lifetime and invalidation rules are documented
- renderer upload behavior is isolated inside the bridge/renderer packages
- no policy package may access raw WASM memory

### 6.4 Generated contracts

Rust protocol crates are the source of truth.

Generated TypeScript contracts should live in:

```txt
/ts/packages/contracts/src/generated
```

Rules:

- generated files are committed for worker convenience
- generated files are not hand-edited
- codegen runs in CI
- generated diffs require protocol-steward review
- fixtures show before/after contract shape when protocols change

---

## 7. TypeScript policy and catalog layer

### 7.1 Policy packages

Policy packages are high-churn expression code. They receive read-only views and return proposed commands.

They may contain:

- decision rules
- scenario-like orchestration
- state-machine policies
- procedural selection logic
- testable heuristics
- abstract process control

They may not contain:

- authoritative mutation
- renderer imports
- DOM imports
- direct WASM memory access
- filesystem/network calls
- wall-clock time
- ambient random
- hidden global registries
- shadow copies of state

### 7.2 Catalog packages

Catalog packages define typed catalogs and data-like declarations. They are useful because TypeScript gives agents typechecking, imports, tests, and reviewable diffs.

Catalog packages should export catalogs, not mutate runtime state.

Rust validates catalog data before accepting it into the authoritative runtime.

```txt
TypeScript catalog
  ↓
serialized/generated catalog bundle
  ↓
Rust catalog validator
  ↓
accepted runtime catalog
```

### 7.3 Script host

The script host is responsible for deterministic execution of policy packs.

It owns:

- loading policy packages
- invoking policy functions
- providing deterministic inputs
- collecting proposed commands
- sandboxing forbidden APIs
- recording script diagnostics
- fixture-based script tests

It does not validate commands. Validation belongs to Rust.

### 7.4 TypeScript strictness rules

Recommended defaults:

```json
{
  "compilerOptions": {
    "strict": true,
    "noImplicitAny": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true,
    "noFallthroughCasesInSwitch": true,
    "noImplicitReturns": true,
    "noImplicitOverride": true,
    "useUnknownInCatchVariables": true
  }
}
```

The TypeScript ESLint tier also enforces type-only imports, explicit public module
boundary types, no explicit `any`, and no async promise callbacks where a void
callback is expected. `check-ts.sh` includes a negative smoke so these rules fail
closed if the config drifts.

Deferred strictness ratchets are intentional and should be handled as focused cleanup
tasks rather than broad churn: `noPropertyAccessFromIndexSignature` currently hits
decoder/validator and environment-variable access patterns, while `no-floating-promises`
mostly flags Node test registration calls. `no-non-null-assertion` and the full
`no-unsafe-*` family also require source/test remediation before they can be enabled
workspace-wide.

Policy packages should additionally be linted against:

- `Date`
- `Math.random`
- `document`
- `window`
- `localStorage`
- `fetch`
- renderer imports
- bridge imports
- Electron imports
- generated file edits
- cross-lane imports

---

## 8. Rust authority layer

### 8.1 Rust crate principles

Every Rust crate should have:

- its own `Cargo.toml`
- a small public API
- local unit tests
- integration tests when appropriate
- crate-level docs explaining ownership and lane rules
- `#![forbid(unsafe_code)]` by default
- no unexplained `clone` patterns in hot or authoritative paths
- no `Rc<RefCell<_>>` for core state mutation
- no framework-shaped abstractions over `StateStore`

### 8.2 Foundation crates

Foundation crates are widely imported and changed deliberately.

Examples:

- `core-ids`
- `core-math`
- `core-time`
- `core-error`
- `core-collections`

Foundation changes require stronger review because they ripple through many workers.

### 8.3 State crates

State crates define state shape, command/event definitions, access rules, snapshots, and migrations.

Examples:

- `core-state`
- `core-events`
- `core-commands`
- `core-snapshot`

State changes require replay fixtures or snapshot migration notes.

### 8.4 Simulation crates

Simulation crates orchestrate ticks, validation, event application, replay, and headless runs.

Examples:

- `sim-kernel`
- `sim-validator`
- `sim-applier`
- `sim-replay`
- `sim-runner`

### 8.5 Service crates

Service crates are good parallel-worker territory. They should be small, explicit, and capability-focused.

Examples:

- `svc-rng`
- `svc-spatial`
- `svc-collision`
- `svc-physics`
- `svc-pathfinding`
- `svc-serialization`
- `svc-volume`
- `svc-mesh`

Services may be hot-loop optimized internally without turning the whole engine into an ECS framework.

### 8.6 Rule crates

Rule crates are authoritative but still generic at the infrastructure stage. They should prove that domain-specific rules can be added later without introducing domain assumptions now.

Early neutral examples:

- `rule-lifecycle`
- `rule-process`
- `rule-scheduler`
- `rule-relationship`
- `rule-state-machine`

These crates should operate on abstract fixtures and generic entity/process concepts only.

### 8.7 Render bridge crates

Render bridge crates convert authoritative state or events into retained render diffs. They do not render.

Examples:

- `render-bridge`
- `render-debug`

They may know render concepts such as handles, transforms, materials, geometry payloads, and debug overlays. They should not introduce product-domain concepts.

### 8.8 WASM API crates

The WASM API is narrow and boring. In the current architecture it is a replay/golden
verification surface, not the product runtime transport.

It exposes replay authority helpers only:

- replay artifact decode/diff classification
- stable divergence class labels

It should not contain product-domain logic, renderer logic, policy logic, runtime init/tick
exports, command submission, render-diff retrieval, telemetry retrieval, or raw memory view
helpers. Runtime transport belongs behind the native bridge and the transport-agnostic
`@asha/runtime-bridge` facade.

---

## 9. Repository structure

```txt
/asha-engine
  README.md

  /governance
    agents.md
    architecture.md
    boundary-rules.md
    ownership.toml
    dependency-policy.toml
    contract-change-process.md

    /adr
      0001-rust-authority-ts-expression.md
      0002-replay-boundary.md
      0003-policy-command-boundary.md
      0004-render-diff-protocol.md
      0005-generated-contracts.md

    /lanes
      rust-foundation.md
      rust-state.md
      rust-service.md
      rust-rule.md
      rust-wasm-bridge.md
      ts-policy.md
      ts-catalog.md
      ts-shell.md
      ts-tools.md
      contract-steward.md

    /reviewer-prompts
      rust-api-reviewer.md
      rust-determinism-reviewer.md
      rust-state-mutation-reviewer.md
      ts-policy-sandbox-reviewer.md
      ts-import-boundary-reviewer.md
      protocol-reviewer.md
      replay-regression-reviewer.md
      render-diff-reviewer.md

  /harness
    /ci
      check-all.sh
      check-rust.sh
      check-ts.sh
      check-contracts.sh
      check-replays.sh
      check-render-goldens.sh
      check-depgraph.sh

    /lint
      /rust-dylint
        no-refcell-state/
        no-unexplained-clone/
        no-unsafe/
        no-framework-ecs/

      /ts-eslint
        no-policy-dom-access/
        no-policy-date-now/
        no-policy-math-random/
        no-cross-lane-imports/
        no-generated-edits/

    /depgraph
      rust-allowed-deps.toml
      ts-allowed-deps.json
      verify-rust-deps.rs
      verify-ts-deps.ts

    /goldens
      /replays
      /snapshots
      /protocol
      /render-diffs
      /screenshots

    /fixtures
      /states
      /policy-inputs
      /policy-outputs
      /commands
      /events
      /render-diffs
      /catalogs

  /engine-rs
    Cargo.toml
    Cargo.lock

    /crates

      /foundation
        /core-ids
          Cargo.toml
          src/lib.rs
        /core-math
          Cargo.toml
          src/lib.rs
        /core-time
          Cargo.toml
          src/lib.rs
        /core-error
          Cargo.toml
          src/lib.rs
        /core-collections
          Cargo.toml
          src/lib.rs

      /state
        /core-state
          Cargo.toml
          src/lib.rs
          src/state.rs
          src/entities.rs
          src/access.rs
        /core-events
          Cargo.toml
          src/lib.rs
          src/domain_event.rs
          src/event_queue.rs
          src/apply.rs
        /core-commands
          Cargo.toml
          src/lib.rs
          src/input_command.rs
          src/policy_command.rs
          src/system_command.rs
          src/validation.rs
        /core-snapshot
          Cargo.toml
          src/lib.rs
          src/snapshot.rs
          src/version.rs
          src/migrate.rs

      /protocol
        /protocol-ids
          Cargo.toml
          src/lib.rs
        /protocol-script
          Cargo.toml
          src/lib.rs
          src/views.rs
          src/commands.rs
          src/rejections.rs
        /protocol-render
          Cargo.toml
          src/lib.rs
          src/render_diff.rs
          src/geometry_payload.rs
          src/materials.rs
          src/handles.rs
        /protocol-replay
          Cargo.toml
          src/lib.rs
          src/replay_file.rs
          src/replay_step.rs
          src/hash.rs
        /protocol-telemetry
          Cargo.toml
          src/lib.rs
          src/log_event.rs
          src/trace_event.rs
        /protocol-codegen
          Cargo.toml
          src/main.rs
          src/emit_ts.rs
          src/emit_schema.rs

      /sim
        /sim-kernel
          Cargo.toml
          src/lib.rs
          src/tick.rs
          src/phases.rs
          src/schedule.rs
        /sim-validator
          Cargo.toml
          src/lib.rs
          src/validate_input_command.rs
          src/validate_policy_command.rs
          src/validate_system_command.rs
        /sim-applier
          Cargo.toml
          src/lib.rs
          src/apply_domain_event.rs
          src/apply_batch.rs
        /sim-replay
          Cargo.toml
          src/lib.rs
          src/record.rs
          src/playback.rs
          src/divergence.rs
        /sim-runner
          Cargo.toml
          src/lib.rs
          src/headless.rs
          src/run_until.rs

      /services
        /svc-rng
          Cargo.toml
          src/lib.rs
          src/seed.rs
          src/streams.rs
        /svc-spatial
          Cargo.toml
          src/lib.rs
          src/transform.rs
          src/spatial_index.rs
        /svc-collision
          Cargo.toml
          src/lib.rs
          src/queries.rs
          src/shapes.rs
        /svc-physics
          Cargo.toml
          src/lib.rs
          src/step.rs
          src/bodies.rs
          src/determinism.rs
        /svc-pathfinding
          Cargo.toml
          src/lib.rs
          src/grid_search.rs
          src/path_cache.rs
        /svc-serialization
          Cargo.toml
          src/lib.rs
          src/save.rs
          src/load.rs
          src/encode.rs
          src/decode.rs
        /svc-volume
          Cargo.toml
          src/lib.rs
          src/volume.rs
          src/chunk.rs
          src/cells.rs
        /svc-mesh
          Cargo.toml
          src/lib.rs
          src/build.rs
          src/buffers.rs

      /rules
        /rule-lifecycle
          Cargo.toml
          src/lib.rs
        /rule-process
          Cargo.toml
          src/lib.rs
        /rule-scheduler
          Cargo.toml
          src/lib.rs
        /rule-relationship
          Cargo.toml
          src/lib.rs
        /rule-state-machine
          Cargo.toml
          src/lib.rs

      /render
        /render-bridge
          Cargo.toml
          src/lib.rs
          src/diff_builder.rs
          src/geometry_exports.rs
          src/scene_handles.rs
        /render-debug
          Cargo.toml
          src/lib.rs
          src/debug_overlays.rs
          src/inspection_layers.rs

      /wasm
        /wasm-api
          Cargo.toml
          src/lib.rs

      /tools
        /replay-tool
          Cargo.toml
          src/main.rs
        /snapshot-diff
          Cargo.toml
          src/main.rs
        /protocol-dump
          Cargo.toml
          src/main.rs
        /state-inspector
          Cargo.toml
          src/main.rs
        /fixture-maker
          Cargo.toml
          src/main.rs

  /ts
    package.json
    pnpm-workspace.yaml
    tsconfig.base.json
    eslint.config.mjs

    /packages

      /contracts
        package.json
        src/index.ts
        src/generated/
          ids.ts
          script.ts
          render.ts
          replay.ts
          telemetry.ts
        src/brands.ts

      /script-sdk
        package.json
        tsconfig.json
        src/index.ts
        src/views.ts
        src/commands.ts
        src/rejections.ts
        src/test-harness.ts

      /script-host
        package.json
        tsconfig.json
        src/index.ts
        src/loadPolicyPack.ts
        src/runPolicyTick.ts
        src/sandbox.ts
        src/commandBuffer.ts

      /policy-core
        package.json
        tsconfig.json
        src/index.ts
        src/defaultPolicy.ts
        src/noopPolicy.ts
        tests/

      /policy-examples
        package.json
        tsconfig.json
        src/index.ts
        src/exampleThresholdPolicy.ts
        src/exampleStateMachinePolicy.ts
        tests/

      /catalog-core
        package.json
        tsconfig.json
        src/index.ts
        src/catalog.ts
        src/definitions/
        tests/

      /catalog-examples
        package.json
        tsconfig.json
        src/index.ts
        src/exampleCatalog.ts
        tests/

      /wasm-bridge
        package.json
        tsconfig.json
        src/index.ts
        src/loadWasm.ts
        src/memoryView.ts
        src/protocolDecode.ts
        src/commandEncode.ts
        src/renderDiffStream.ts

      /renderer-three
        package.json
        tsconfig.json
        src/index.ts
        src/scene.ts
        src/handleRegistry.ts
        src/geometryRegistry.ts
        src/materialRegistry.ts
        src/applyRenderDiff.ts
        src/camera.ts
        src/debugRenderLayers.ts

      /ui-dom
        package.json
        tsconfig.json
        src/index.ts
        src/app.ts
        src/panels/
        src/inspectors/
        src/commandPalette/
        src/stateViewModels/

      /cosmetic
        package.json
        tsconfig.json
        src/index.ts
        src/particles.ts
        src/transientAnimation.ts
        src/screenEffects.ts

      /devtools
        package.json
        tsconfig.json
        src/index.ts
        src/replayViewer/
        src/debugDashboard/
        src/catalogValidator/
        src/stateInspector/
        src/scriptLab/

      /electron-main
        package.json
        tsconfig.json
        src/main.ts
        src/window.ts
        src/ipc.ts
        src/platform.ts

      /app
        package.json
        tsconfig.json
        src/main.ts
        src/bootstrap.ts
        src/runtimeLoop.ts
        src/wireUiCommands.ts
        src/wireRenderDiffs.ts
        src/wirePolicy.ts

  /assets
    /art
    /audio
    /fonts
    /shaders
    /localization

  /data
    /golden-states
    /seed-catalog
    /reference-fixtures

  /docs
    architecture-overview.md
    replay-model.md
    policy-authoring.md
    render-protocol.md
    determinism.md
    contract-governance.md
```

---

## 10. Dependency policy

### 10.1 Rust dependency direction

The intended Rust dependency direction:

```txt
foundation
  ↓
state
  ↓
protocol
  ↓
sim / services / rules
  ↓
render-bridge / wasm-api / tools
```

No lower-level crate may depend on higher-level crates.

Rules:

- foundation crates know nothing about state, protocols, services, rules, rendering, WASM, or tools
- state crates know nothing about rendering, UI, Electron, or TypeScript
- protocol crates define border types but do not perform product-domain logic
- service crates provide capabilities, not policy
- rule crates are authoritative but generic and isolated
- render bridge emits render diffs only
- wasm API exports a narrow host boundary

### 10.2 TypeScript dependency direction

The intended TypeScript dependency direction:

```txt
contracts
  ↓
script-sdk
  ↓
policy/catalog packages
  ↓
script-host
```

and separately:

```txt
contracts
  ↓
wasm-bridge
  ↓
renderer-three / ui-dom / devtools
  ↓
app
  ↓
electron-main wrapper boundary
```

Policy packages may import contracts, script SDK, and approved catalogs. They may not import renderer, UI, WASM bridge, Electron, or browser globals.

Renderer packages may import contracts and bridge packages. They may not import policy packages.

UI packages may issue commands and display state views. They may not mutate authoritative state or import policy internals.

Tools can be more omniscient for inspection, but tool-only imports must not leak into runtime packages.

### 10.3 Machine-readable ownership

`/governance/ownership.toml` should encode lanes and boundaries.

Example:

```toml
[package."engine-rs/crates/services/svc-pathfinding"]
lane = "rust-service"
owners = ["engine-orchestrator"]
may_depend_on = [
  "core-ids",
  "core-math",
  "core-state",
  "core-error"
]
may_not_depend_on = [
  "protocol-render",
  "wasm-api",
  "render-bridge"
]

[package."engine-rs/crates/rules/rule-state-machine"]
lane = "rust-rule"
owners = ["authority-orchestrator"]
may_depend_on = [
  "core-ids",
  "core-state",
  "core-events",
  "core-commands",
  "core-error"
]
may_not_depend_on = [
  "render-bridge",
  "wasm-api"
]

[package."ts/packages/policy-core"]
lane = "ts-policy"
owners = ["policy-orchestrator"]
may_import = [
  "@agent-engine/contracts",
  "@agent-engine/script-sdk",
  "@agent-engine/catalog-core"
]
may_not_import = [
  "@agent-engine/renderer-three",
  "@agent-engine/ui-dom",
  "@agent-engine/wasm-bridge",
  "@agent-engine/electron-main"
]
forbid_globals = [
  "Date",
  "Math.random",
  "document",
  "window",
  "localStorage",
  "fetch"
]
```

---

## 11. Harness and CI

### 11.1 Core checks

`/harness/ci/check-all.sh` should orchestrate:

```txt
Rust formatting
Rust check/build
Rust clippy
Rust tests
Rust WASM build
Rust dependency graph verification
Rust public API drift check
Rust semver/API compatibility check where useful
Rust custom lints

TypeScript install consistency
TypeScript typecheck
TypeScript tests
TypeScript lint
TypeScript dependency graph verification
Generated-file edit check
Policy sandbox lint

Protocol codegen
Generated TS contract comparison
Schema fixture diff
Replay golden tests
Snapshot migration tests
Render diff fixture tests
Headless screenshot tests when renderer exists
```

### 11.2 Local agent checks

Every lane should have a fast local command.

Examples:

```txt
cargo test -p svc-pathfinding
cargo clippy -p svc-pathfinding
pnpm test --filter @agent-engine/policy-core
pnpm typecheck --filter @agent-engine/renderer-three
pnpm lint --filter @agent-engine/script-host
```

The orchestrator can then route compiler/test failures back to the responsible lane without involving the entire repo.

### 11.3 Golden fixtures

Golden fixtures should exist for:

- command validation
- accepted event batches
- replay divergence
- snapshots
- generated protocols
- policy input/output
- render diffs
- catalog validation
- screenshot diffs once rendering is active

Golden tests should be small, named, and inspectable. The point is not only regression protection; it is agent legibility.

### 11.4 Drift tripwires

Rust tripwires:

- `unsafe` in authoritative crates
- `Rc<RefCell<_>>` in state or simulation crates
- unexplained clones in authoritative paths
- new public API without review note
- framework-shaped abstractions over `StateStore`
- generic event buses
- renderer concepts leaking into state/sim crates
- protocol types gaining convenience behavior

TypeScript tripwires:

- policy package importing renderer/UI/bridge/Electron
- policy package using browser or wall-clock globals
- manual edits to generated files
- shadow state models in policy or UI
- script host validating authority decisions
- renderer importing policy packages
- app package accumulating feature logic
- tool-only imports leaking into runtime packages

---

## 12. Agent lanes

### 12.1 `rust-foundation`

Owns foundational crates. Changes are deliberate and heavily reviewed.

Allowed work:

- typed IDs
- math helpers
- time/tick primitives
- shared error types
- low-level collections

Review requirements:

- public API review
- downstream compile check
- migration notes when identifiers or serialization change

### 12.2 `rust-state`

Owns authoritative state shape, command/event definitions, snapshots, and access rules.

Allowed work:

- state storage
- entity lifecycle
- command definitions
- event definitions
- snapshot/migration shape

Review requirements:

- replay fixture updates
- snapshot compatibility check
- protocol review if command/view/event surfaces change

### 12.3 `rust-service`

Owns isolated engine capabilities.

Allowed work:

- deterministic RNG
- spatial indexing
- collision queries
- physics integration
- path search
- serialization
- volume/mesh processing

Review requirements:

- local service tests
- deterministic fixture when authoritative
- no policy concepts
- no renderer leakage except through render bridge contracts

### 12.4 `rust-rule`

Owns generic authoritative rule crates during the infrastructure phase.

Allowed work:

- lifecycle rules
- process rules
- scheduling rules
- relationship rules
- state-machine rules

Review requirements:

- command validation tests
- event application tests
- replay fixture updates
- no domain-specific nouns in public APIs

### 12.5 `contract-steward`

Owns Rust protocol crates, code generation, generated TypeScript contracts, and schema fixtures.

Allowed work:

- protocol definition changes
- codegen changes
- generated contract updates
- compatibility notes

Review requirements:

- generated diff matches codegen
- TS typecheck passes
- fixture diff is intentional
- downstream package impact is listed

### 12.6 `ts-policy`

Owns constrained TypeScript policy packages.

Allowed work:

- policy functions over generated views
- proposed command generation
- policy fixtures
- deterministic script tests

Forbidden:

- direct state mutation
- renderer imports
- UI imports
- bridge imports
- Electron imports
- browser globals
- ambient random
- wall-clock time

### 12.7 `ts-catalog`

Owns typed catalogs and schema-like authoring code.

Allowed work:

- catalog definitions
- validation fixtures
- generated documentation
- catalog test cases

Forbidden:

- authoritative mutation
- renderer imports unless explicitly display-only and isolated
- runtime global registries

### 12.8 `ts-shell`

Owns renderer, UI, bridge, cosmetic effects, and app composition.

Allowed work:

- render diff consumption
- DOM UI
- command submission
- visual-only effects
- app wiring

Forbidden:

- policy decisions
- authoritative state mutation
- hidden state shadow models
- imports from policy packages unless routed through script host/app composition

### 12.9 `ts-tools`

Owns devtools, replay viewers, inspectors, validators, dashboards, and script labs.

Allowed work:

- broad inspection
- fixture authoring
- debug views
- replay diagnostics
- catalog validation tools

Restriction:

- tool omniscience must remain tool-only and must not leak into runtime packages.

---

## 13. Prototype plan

> **Historical section:** these phases describe the original infrastructure prototype plan. They are not the current work queue. Current implementation status is tracked in Den tasks/docs; the launchable voxel loop and post-launchable expansion work are indexed from `README.md` and `docs/launchable-voxel.md`.

The prototype should prove infrastructure capability, not product-domain behavior.

### Phase 0 — Governance skeleton

Build:

- repo skeleton
- Cargo workspace
- pnpm workspace
- governance docs
- ownership/dependency config
- CI entrypoints
- generated-file guard
- minimal lint configuration

Exit criteria:

- empty or near-empty crates/packages compile/typecheck
- dependency graph checker can pass/fail intentional examples
- agents can be assigned to lanes mechanically

### Phase 1 — Minimal Rust authority core

Build:

- typed IDs
- `StateStore`
- abstract entity storage
- command types
- domain event types
- event queue
- event applier
- snapshot/hash scaffold

Use only abstract fixtures such as `Entity`, `Subject`, `Process`, `Mode`, `Signal`, and `Tag`.

Exit criteria:

- create/update/delete entity fixture
- command validation fixture
- event application fixture
- state hash fixture
- headless tick test

### Phase 2 — Protocol generation

Build:

- `protocol-script`
- `protocol-render`
- `protocol-replay`
- TypeScript contract generation
- generated contract package
- schema/golden diffs

Exit criteria:

- Rust protocol source generates TS contracts
- manual generated-file edit fails CI
- TS package can import generated branded IDs and command unions

### Phase 3 — Constrained TypeScript policy

Build:

- script SDK
- script host
- no-op policy
- threshold policy fixture
- command collection
- sandbox lint rules

Exit criteria:

- policy receives read-only view
- policy returns proposed command
- Rust validates or rejects command
- policy cannot import renderer/UI/bridge packages
- forbidden globals fail lint

### Phase 4 — Replay audit path

Build:

- replay record format
- command/event recording
- state hash intervals
- divergence report
- replay tool

Exit criteria:

- golden replay passes
- intentional event change produces useful divergence report
- repair agent can identify crate/package responsible from failure output

### Phase 5 — Render projection path

Build:

- retained render diff protocol
- render bridge
- WASM bridge decode path
- placeholder Three.js scene
- handle registry
- basic screenshot fixture

Use abstract renderables only: handles, transforms, primitive geometry, labels, debug overlays.

Exit criteria:

- Rust emits create/update/destroy render diffs
- TS renderer consumes diffs without state access
- screenshot/golden diff can run headlessly

### Phase 6 — Parallel agent fan-out trial

Run multiple agents in separate lanes:

- one Rust service change
- one Rust rule change
- one TS policy change
- one renderer change
- one devtool change
- one contract-steward change, if needed

Exit criteria:

- independent tasks merge without hidden coupling
- contract change impacts are explicit
- dependency violations are caught automatically
- replay/render fixtures identify regressions locally

---

## 14. Dependency and library posture

The project should use libraries that are called by the engine, not frameworks that call the engine.

Candidate categories:

- math
- typed arenas/generational handles
- deterministic RNG
- serialization
- replay encoding
- path search
- graph algorithms
- collision/physics
- geometry/mesh processing
- Rust-to-TypeScript type generation
- WASM bridge helpers
- structured tracing

Avoid adopting a full external engine framework early. Frameworks often bring scheduling, plugin models, reflection-like patterns, macro-heavy APIs, or execution models that make source code less agent-legible.

The project can selectively adopt focused crates/packages where they preserve the architecture:

- library called by explicit service code: good
- framework that owns execution and inverts control: suspect
- macro magic that hides behavior from reviewers: suspect
- dependency that crosses authority/render/policy boundaries: reject by default

### Renderer choice

The initial renderer is **Three.js** (`@asha/renderer-three`), chosen to match
the "libraries the engine calls, not frameworks that call the engine" posture:
the Rust authority core owns the loop, state, and timing and emits retained-mode
render diffs, so the renderer is a thin projector that submits to our loop and
applies our `RenderFrameDiff`s. Three.js's library shape, agent-authorability,
and minimal surface fit the abstract renderables (handles, transforms, primitive
geometry, labels, overlays) better than a full engine.

The renderer must stay **loosely coupled**: it is reached only through the
generated `render.ts` contract and the `ts-shell` lane boundary, never owns
authoritative state, and is swappable. Keep engine-specific code behind a small
`applyFrameDiff(scene, diff)` seam so the choice can be revisited (or A/B'd)
without touching the contract. Apply authoritative diffs imperatively — do not
introduce a second reconciler (e.g. react-three-fiber) that re-diffs the scene.

---

## 15. Wrapper and shell posture

The wrapper should be thin.

Electron is acceptable because it ships a pinned Chromium runtime, which is useful for renderer reproducibility. The cost in bundle size and idle memory is acceptable for a renderer-heavy application where rendering consistency and testability matter.

Wrapper rules:

- main process owns window/platform integration only
- context isolation on
- Node integration off for renderer content
- IPC surface narrow and validated
- no authoritative product-domain logic in Electron main/preload
- no policy execution in Electron main/preload
- no renderer decisions in Electron main/preload

The wrapper should be replaceable because it owns little.

---

## 16. Documentation set

Repository docs describe durable architecture, boundaries, and operating
procedures. Current planning state belongs in Den tasks/docs/messages rather
than in repo prose.

Canonical and supporting docs:

```txt
/README.md
/docs/design.md
/governance/agents.md
/governance/architecture.md
/governance/boundary-rules.md
/governance/contract-change-process.md
/docs/architecture-overview.md
/docs/replay-model.md
/docs/policy-authoring.md
/docs/render-protocol.md
/docs/determinism.md
/docs/contract-governance.md
```

Each lane doc should answer:

- what this lane owns
- what this lane may import/depend on
- what this lane must never touch
- what tests are required
- what fixtures are required
- what drift smells reviewers should flag
- what public API changes require escalation

---

## 17. Minimal agent assignment templates

### Rust service worker

```txt
Assignment:
  Crate: engine-rs/crates/services/<crate-name>

Allowed:
  Work inside assigned crate.
  Add local tests and fixtures.
  Depend only on approved foundation/state crates.

Forbidden:
  Do not edit protocol crates.
  Do not edit generated TypeScript.
  Do not introduce renderer/UI concepts.
  Do not add generic framework abstractions.

Required checks:
  cargo fmt
  cargo test -p <crate-name>
  cargo clippy -p <crate-name>
  harness depgraph check
```

### TypeScript policy worker

```txt
Assignment:
  Package: ts/packages/<policy-package>

Allowed imports:
  @agent-engine/contracts
  @agent-engine/script-sdk
  approved catalog packages

Forbidden imports:
  @agent-engine/renderer-three
  @agent-engine/ui-dom
  @agent-engine/wasm-bridge
  @agent-engine/electron-main

Forbidden APIs:
  Date
  Math.random
  document
  window
  localStorage
  fetch

Required checks:
  pnpm typecheck --filter <package>
  pnpm test --filter <package>
  pnpm lint --filter <package>
  policy fixture update when behavior changes
```

### Contract steward

```txt
Assignment:
  Protocol crates and generated contracts.

Allowed:
  Edit engine-rs/crates/protocol/*.
  Run protocol codegen.
  Update ts/packages/contracts/src/generated/*.
  Update protocol fixtures.

Forbidden:
  Do not hand-edit generated files without matching Rust protocol source.
  Do not add product-domain behavior convenience logic to protocol crates.

Required checks:
  cargo test -p protocol-codegen
  protocol fixture diff
  pnpm typecheck
  downstream compile/typecheck impact note
```

### Renderer worker

```txt
Assignment:
  Package: ts/packages/renderer-three

Allowed:
  Consume render diffs.
  Manage scene handles.
  Update geometry/material registries.
  Add render-diff fixtures and screenshot tests.

Forbidden:
  Do not import policy packages.
  Do not inspect authoritative state.
  Do not invent product-domain behavior state.
  Do not submit authority commands except through approved UI/app paths.

Required checks:
  pnpm typecheck --filter @agent-engine/renderer-three
  pnpm test --filter @agent-engine/renderer-three
  render fixture update
```

---

## 18. Historical open questions

These were original design prompts. Check Den planning docs/tasks before treating
any item here as current work.

1. **Replay target:** Should canonical replay be WASM-only, native-plus-WASM, or native until render integration forces WASM parity?
2. **Crate granularity:** What is the smallest useful crate boundary before cross-crate API churn outweighs isolation?
3. **Protocol granularity:** Should render diffs remain one general protocol or split into channels for transforms, geometry, materials, debug overlays, and UI-facing labels?
4. **Policy execution:** Should policy packs be bundled into the app, dynamically loaded during development only, or both?
5. **Catalog validation:** Should catalogs originate in TypeScript and be validated by Rust, or should Rust own more of the catalog schema earlier?
6. **Tool omniscience:** How much private state/protocol access may devtools have before they start normalizing bad runtime dependencies?
7. **Lint investment:** Which governance rules should become custom lints immediately, and which can remain prose until the first violation?
8. **Native/WASM divergence:** What diagnostics are needed when native tests pass but WASM replay diverges?
9. **Public API drift:** Which crates/packages require public API review on every exported symbol change?
10. **Agent fan-out limit:** At what number of simultaneous workers do contract-steward and foundation lanes become bottlenecks?

---

## 19. One-breath summary

> A Rust authority core owns state, validation, deterministic services, replay, and heavy simulation; constrained TypeScript policy/catalog packages author high-level intent through generated read-only views and command types; a separate TypeScript shell renders and displays projected truth; protocols are generated from Rust and stewarded as border infrastructure; every crate and package is an agent assignment cell with machine-checkable dependency rules.

The system is intentionally bureaucratic. That is the point. The bureaucracy is not there to let one senior reviewer become a repo tyrant. It is there to let parallel agent labor remain legible, bounded, testable, and rejectable.
