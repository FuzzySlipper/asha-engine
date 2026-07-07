# Runtime bridge boundary

> Companion ADR: `governance/adr/0006-runtime-bridge-boundary.md`.
> Source decision note: Den doc `runtime-boundary-napi-wasm-replay-strategy`.

This document fixes the **package/crate shape**, the **allowed dependency graph**, and the
**migration plan** for ASHA's runtime boundary. It is the cold-start reference for the
runtime-boundary implementation tasks (#2249 manifest/glue, #2250 native prototype,
#2251 WASM replay).

## 1. Decision in one paragraph

The Electron runtime path executes the authoritative Rust core natively through `napi-rs`.
WASM stays the canonical **replay/golden verification** target, not the runtime transport.
Neither the native addon nor the WASM module is a public interface: app / UI / renderer /
devtools depend only on a narrow, transport-agnostic facade. Generated contracts remain the
semantic/governance border; native and WASM are transport glue underneath it.

## 2. Layers

Consumer-facing compatibility metadata for the two Tier 1 surfaces lives in
`docs/consumer-compatibility.md` plus package-local `compatibility.json` files:
`ts/packages/contracts/compatibility.json` and
`ts/packages/runtime-bridge/compatibility.json`.

Four distinct layers, lowest coupling to highest:

| Layer | Role | TS package | Rust crate |
|---|---|---|---|
| Semantic contracts | command/event/view/render/replay schemas | `@asha/contracts` (generated) | `protocol-*` crates |
| Runtime bridge facade | stable transport-agnostic API consumers couple to | `@asha/runtime-bridge` | `runtime-bridge-api` (manifest + types + conformance helpers) |
| Native transport | `napi-rs` addon wrapping the engine for the Electron runtime | `@asha/native-bridge` | `native-bridge` |
| Replay verification | WASM build of the authoritative core for golden/divergence checks | `@asha/wasm-replay-bridge` | `wasm-api` (existing) |

The facade selects an implementation at composition time: **native** (production runtime),
**mock** (most TS tests, no addon load), or **wasm-replay** (devtools/replay harness).

## 3. Package / crate names (final)

TypeScript (`ts/packages/`):

- `contracts` — unchanged. Generated semantic TS contracts.
- `runtime-bridge` — **new.** Public facade: interfaces, mock implementation, buffer-handle
  API, error taxonomy, generated conformance tests. Render-diff decoding moves here (see §6).
- `native-bridge` — **new.** Thin wrapper over the `napi-rs` addon. Imported **only** by
  `runtime-bridge`. Holds no semantic logic.
- `wasm-replay-bridge` — **rename/repurpose of `wasm-bridge`.** Replay/golden/devtools WASM
  path only. Imported by tests/devtools, never by `app`/`renderer-three`/`ui-dom`.

Rust (`engine-rs/crates/bridge/`, a new layer between `render`/`wasm` and `tools`):

- `runtime-bridge-api` — **new.** Owns the bridge manifest, the N-API-visible boundary types,
  protocol↔boundary conversion stubs, and conformance helpers. No `napi` dependency itself.
- `native-bridge` — **new.** The `napi-rs` `cdylib` addon. Depends on `runtime-bridge-api`
  plus the engine crates (sim/services/render). The only crate that depends on `napi`.
- `wasm-api` — **existing.** Stays the WASM export surface; its scope narrows toward replay
  (init, tick, command submit, replay/snapshot hooks, diff retrieval) per design §8.8.

Exact names are now fixed for the implementation tasks; the **dependency shape** below is the
invariant that must not change even if a name is later revised.

## 4. Allowed dependency graph

### 4.1 TypeScript

```
contracts ◄── runtime-bridge ◄── native-bridge        (native: runtime-bridge imports it)
                   ▲                                    wasm-replay-bridge ──► contracts
   app / renderer-three / ui-dom / cosmetic ──► runtime-bridge, contracts
   devtools ──► runtime-bridge, contracts, wasm-replay-bridge
   policy-* / catalog-* ──► contracts only
```

Rules (enforced by `harness/depgraph/verify-ts-deps.sh` via `ownership.toml`):

1. `app`, `renderer-three`, `ui-dom`, `cosmetic` import `@asha/runtime-bridge` (and
   `@asha/contracts`) for runtime — **not** `@asha/native-bridge`, **not**
   `@asha/wasm-replay-bridge`.
2. **Only** `@asha/runtime-bridge` may import `@asha/native-bridge`.
3. `@asha/wasm-replay-bridge` may be imported **only** by `devtools` and test code.
4. `policy-*` and `catalog-*` import **neither** any runtime/transport implementation **nor**
   the raw native addon — `@asha/contracts` only (unchanged from today).
5. No transport-specific type (addon handle, WASM memory view) appears in any package's public
   API except `native-bridge` / `wasm-replay-bridge` internals.

### 4.2 Rust

New `bridge` layer inserted into `dependency-policy.toml` layer order:

```
foundation, state, protocol, sim, services, rules, render, wasm, bridge, tools
```

- `runtime-bridge-api` may depend on `protocol-*` (+ foundation). No `napi`, no `wasm-bindgen`.
- `native-bridge` may depend on `runtime-bridge-api`, sim/services/render/protocol crates, and
  `napi`/`napi-derive`. It is the **only** crate allowed to depend on `napi`.
- `wasm-api` keeps its existing `may_depend_on` set; `napi` is forbidden to it, `wasm-bindgen`
  forbidden to `native-bridge`. No feature-gated behavior divergence in authority crates.
- No crate below `bridge` may depend on `native-bridge` or `runtime-bridge-api`.

## 5. Bridge shape (allowed / disallowed)

Allowed bounded verbs (full manifest defined in #2249):

```
initializeEngine(config)
stepSimulation(inputEnvelope) -> StepResult
submitCommands(commandBatch) -> CommandResult
validateGameRuleCatalog(gameRuleCatalog) -> GameRuleCatalogValidationReceipt
submitGameRuleEffectIntent(catalog, request) -> GameRuleResolutionReceipt
readGameRuleRuntimeReadout() -> GameRuleRuntimeReadout
readRenderDiffs(frameCursor)  -> RenderFrameDiffDescriptor
getBuffer(bufferHandle)       -> typed buffer view
releaseBuffer(bufferHandle)
loadReplayFixture(...) / runReplayStep(...)
```

Disallowed (mechanically guarded in #2249): `callRust(methodName, json)`-style dispatch; raw
JSON escape hatches; exposing `StateStore` handles to TS; UI/renderer importing the native
addon; duplicate hand-written schemas in the bridge; transport types leaking into
policy/catalog; bypassing generated contract surfaces.

The game-rules operations are bounded RuntimeSession bridge verbs over generated
`protocol-game-rules` DTOs. They validate catalogs, resolve one typed effect
intent, and expose recent modifier/trace/replay readouts through Rust-owned
`svc-game-rules` state. They are not permission to add arbitrary rule method
dispatch, JS callbacks, local TS rule authority, or raw JSON tunnels.

## 6. `wasm-bridge` runtime assumptions — migration (DONE)

> **Status: completed.** `ts/packages/wasm-bridge` has been removed. Its render-diff decode +
> `RenderDiffStream` + `FrameMemory` moved into `@asha/runtime-bridge` (`render-decode.ts`);
> `renderer-three`/`ui-dom`/`app`/`devtools` now import the `@asha/runtime-bridge` facade; the
> replay/WASM role lives in `@asha/wasm-replay-bridge`. The table below records the original
> assumptions and where each piece landed.

Previously `ts/packages/wasm-bridge` (lane `ts-shell`) was the single thing shell packages imported,
and it mixes two concerns:

| Piece | Concern | Disposition |
|---|---|---|
| `decodeRenderDiff` / `decodeRenderFrameDiff` | transport-neutral: payload → `@asha/contracts` types | **Move to `runtime-bridge`** as `readRenderDiffs` output decoding. Both native and WASM paths reuse it. |
| `RenderDiffStream` (FIFO of decoded frames) | facade-level frame buffering | **Move to `runtime-bridge`.** |
| `FrameMemory` (borrowed view over "WASM-owned bytes") | **runtime assumption**: large payloads come from WASM memory | **Quarantine + reshape.** Runtime large payloads come from native bridge-owned buffers via `getBuffer`/`releaseBuffer` handles, not WASM memory. The lifetime/invalidation contract is sound and carries over to the facade's `BufferView`; the "WASM-owned" framing is the part to drop. |
| package name / `ts-shell` lane position as runtime transport | **runtime assumption**: WASM is the runtime path | **Repurpose** package to `wasm-replay-bridge`, demoted to replay/devtools-only import scope. |

Current importers to update: `renderer-three` (imports `decodeRenderFrameDiff` +
`RenderDiffStream`), and any future `app`/`ui-dom`/`devtools` wiring. After migration they
import `@asha/runtime-bridge`; `renderer-three` no longer depends on `@asha/wasm-bridge`.

Rust side: `wasm-api/src/lib.rs` is currently empty and `render-bridge` already emits JSON
fixtures consumed cross-language (`render-protocol.md` §"Phase 5 dataflow"). That JSON-fixture
path is the **replay/golden** artifact and stays; it is not the runtime transport. No authority
crate currently assumes a transport, so migration is additive (new crates/packages) rather than
a rewrite.

## 7. Affected files / packages (cold-start checklist)

Docs/config (this task, #2248):
- `docs/runtime-bridge-boundary.md` (this file), `governance/adr/0006-runtime-bridge-boundary.md`.
- `governance/dependency-policy.toml` — add `bridge` layer + forbidden cross-layer pairs.
- `governance/ownership.toml` — register `runtime-bridge`, `native-bridge`,
  `wasm-replay-bridge`, `runtime-bridge-api`, `native-bridge` (Rust) import rules.
- `governance/boundary-rules.md`, `AGENTS.md`/`agents-project.md` repo-structure block — note
  the runtime/replay split.

Later tasks (do not implement here):
- #2249: bridge manifest file + owning crate, generated-glue plan, conformance test shape,
  guardrail check script.
- #2250: `native-bridge` crate (`napi-rs`), `@asha/runtime-bridge` + mock, `@asha/native-bridge`.
- #2251: repurpose `wasm-bridge` → `wasm-replay-bridge`; native-vs-WASM divergence classifier;
  ensure replay goldens run through the WASM path.

## 8. Migration sequencing

1. **#2248 (this):** docs + dependency-policy/ownership scaffolding. No runtime code.
2. **#2249:** manifest + glue/conformance design; guardrails against `serde_json::Value` / TS
   `any`/`unknown` / dynamic dispatch in stable bridge surfaces.
3. **#2250:** facade + mock + minimal native op behind it; only `runtime-bridge` imports native.
4. **#2251:** preserve WASM replay path; move decode/stream out of `wasm-bridge`; classify
   native-vs-WASM divergence.

Each step keeps existing Phase 5 checks (`check-render-goldens`, `check-contracts`,
`check-depgraph`) green; the render JSON-fixture golden path is preserved throughout.
