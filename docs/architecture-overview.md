# Architecture overview

## One-breath summary

Rust owns canonical state, validation, deterministic services, replay, and simulation.
Constrained TypeScript policy proposes commands through generated read-only views.
A separate TypeScript shell renders and displays projected state.
Protocols are generated from Rust and stewarded as border infrastructure.
Every crate and package is an agent assignment cell with machine-checkable dependency rules.

## Layer model

| Layer | Language | Owns |
|---|---|---|
| Authority core | Rust | canonical state, validation, event application, replay, simulation |
| Protocols | Rust → generated TS | command/view/event/diff schemas, type generation |
| Policy/catalog | TypeScript | authored policy, data catalogs, proposed commands |
| Script host | TypeScript | policy loading, sandboxing, deterministic invocation |
| WASM bridge | TypeScript | WASM loading, memory views, protocol encode/decode |
| Renderer | TypeScript + Three.js | scene projection from render diffs |
| UI shell | TypeScript + DOM | panels, input collection, inspectors |
| Cosmetic | TypeScript + renderer | non-authoritative visual effects |
| Wrapper | Electron | process/window/platform integration |
| Devtools | TypeScript | inspection, replay viewing, debug workflows |

## Core invariant

> TypeScript can propose commands. Rust validates commands and applies accepted events.

TypeScript never mutates authoritative state. It receives generated read-only projections
and returns proposed commands. Rust is the sole validator and state mutator.

## Tick model

```
Inputs / tools / policy packs
  ↓ proposed commands
Rust validation
  ↓ accepted domain events
Sequential event application
  ↓ updated StateStore
Render diffs + telemetry + replay records
```

## Dependency direction

Rust: `foundation → state → protocol → sim/services/rules → render/wasm/tools`

TypeScript:
- `contracts → script-sdk → policy/catalog → script-host`
- `contracts → runtime-bridge (facade) → renderer/ui/devtools → app → electron-main`
  (native `napi-rs` / WASM-replay transports sit behind the facade — ADR 0006)

See `governance/ownership.toml` for machine-readable per-crate/package rules.
See `governance/dependency-policy.toml` for layer-level enforcement rules.
