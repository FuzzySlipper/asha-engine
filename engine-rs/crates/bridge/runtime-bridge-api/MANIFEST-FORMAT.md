# Bridge manifest format + generated-glue plan + conformance shape

Task #2249. Companion to `bridge-manifest.toml`. Governs the curated-manifest +
generated-boring-glue + hand-written-bodies middle path (ADR 0006).

## 1. Why a manifest (and what it is NOT)

The manifest is a **small, hand-reviewed list of allowed bridge operations** with typed
input/output, an error channel, and buffer/lifetime notes. It exists so the bridge surface is
diffable and review-gated.

It is **not**:
- a generic RPC registry,
- a `callRust(methodName, json)` dispatcher,
- a place for `serde_json::Value` / `any` / `unknown` payloads,
- a second hand-written schema layer (semantic types live in `protocol-*` crates).

Adding an operation is a deliberate boundary change → contract-steward review.

## 2. Schema

```
[manifest]
version            : int (currently 1)
owning_crate       : "runtime-bridge-api"
facade_package     : "@asha/runtime-bridge"
native_package     : "@asha/native-bridge"
wasm_replay_package: "@asha/wasm-replay-bridge"
error_type         : single typed error enum for ALL operations
handle_types       : opaque bridge-owned handle type names

[[operation]]            (one table per operation)
name              : snake_case, unique
surface           : "stable" | "quarantined"
quarantine_reason : required iff surface == "quarantined"
input             : exactly one type ref (protocol_*::Type or a declared handle_type or "Unit")
output            : exactly one type ref (protocol_*::Type, a declared handle_type,
                    "RuntimeBufferView", or "Unit")
errors            : must equal manifest.error_type
buffers           : optional; required when the op lends/borrows/releases a buffer; states lifetime
summary           : one-line human description
```

### Validation rules (enforced by `harness/bridge/validate-manifest.py`)
1. Every operation has exactly one `input` and one `output` — no variadic, no `methodName`+payload.
2. `input`/`output` reference a `protocol_*::` type, a declared `handle_type`, `RuntimeBufferView`,
   or `Unit`. **Forbidden type tokens:** `serde_json::Value`, `Value`, `Json`, `any`, `unknown`,
   `Box<dyn`, `dyn `. (These would re-open an opaque escape hatch.)
3. `errors` equals `manifest.error_type` for every operation.
4. `surface` ∈ {`stable`, `quarantined`}; `quarantined` requires `quarantine_reason`.
5. Buffer-lending ops (`get_buffer`/`release_buffer`/anything with `buffers`) must declare a
   lifetime note in `buffers`.
6. Operation `name`s are unique and snake_case.

## 3. Generated boring glue (the plan)

From `bridge-manifest.toml` + the `protocol-*` crates, codegen (extending `protocol-codegen`)
emits **only mechanical glue — one operation in, one wrapper out, no hidden behavior**:

| Artifact | Path (generated, committed, not hand-edited) | Shape |
|---|---|---|
| TS facade interface | `ts/packages/runtime-bridge/src/generated/operations.ts` | one method signature per operation over `@asha/contracts` types |
| TS facade skeleton | `ts/packages/runtime-bridge/src/generated/skeleton.ts` | abstract base the hand-written facade + mock implement |
| N-API wrapper sigs | `engine-rs/crates/bridge/native-bridge/src/generated/exports.rs` | one `#[napi]` fn signature per operation; bodies call hand-written semantic fns |
| Rust conversion stubs | `engine-rs/crates/bridge/runtime-bridge-api/src/generated/convert.rs` | protocol-type ↔ N-API-visible struct, where shapes differ |
| Conformance fixtures | `ts/packages/runtime-bridge/src/generated/conformance.json` | manifest signature snapshot the conformance test asserts against |
| Dep-policy metadata | already in `ownership.toml` | which package may import the raw addon |

Rules: generated code is transparent and diffable; **no broad reflection, no dynamic
`methodName + json` dispatcher**. Codegen never decides policy, state access, ownership, or
buffer lifetime — those are hand-written bodies.

> Status (#2249): the **plan, manifest, and validators are committed**; the codegen emitter
> itself is implemented in #2250 alongside the first real operations (it needs the
> `protocol_runtime` types — `EngineConfig`/`StepInputEnvelope`/`StepResult`/
> `RenderFrameDiffDescriptor` — which #2250 introduces). This file + the validator are the
> diffable contract the emitter must satisfy.

## 4. Hand-written (NOT generated)

Semantic operation bodies stay hand-written and reviewed: engine/session lifecycle, validation
calls, state-mutation phases, render-diff collection, buffer allocation/lifetime/disposal, error
classification, native/WASM divergence reporting. The public `@asha/runtime-bridge` facade is
hand-written for readability **but must satisfy the generated conformance test**.

## 5. Conformance test shape

Lives in `ts/packages/runtime-bridge` (facade) and is mirrored by a Rust addon smoke test.

1. **Facade-vs-manifest conformance** (`runtime-bridge`): load
   `generated/conformance.json`; assert the hand-written facade exposes exactly the manifest
   operations, with matching arity and contract-typed params/returns. Fails on drift (extra
   method, missing method, signature mismatch).
2. **Mock-vs-facade** (`runtime-bridge`): the mock implementation passes the same facade-level
   behavioral tests as any real transport (so most TS tests need no addon load).
3. **Native addon smoke** (`native-bridge`, #2250): every exported `#[napi]` op is called once
   with a tiny fixture; asserts it loads and round-trips a typed value.
4. **WASM replay conformance** (`wasm-replay-bridge`, #2251): replay fixtures run through the
   WASM path for deterministic ops; divergence is classified, not discovered.

These map to checklist items 3–9 in the source decision note
(`runtime-boundary-napi-wasm-replay-strategy` §"Binding maintenance checks").

## 6. Mechanical guardrails

`harness/bridge/check-bridge-guardrails.sh` rejects, in **stable** bridge surfaces
(`runtime-bridge`, `native-bridge`, `runtime-bridge-api`):
- Rust: `serde_json::Value`, `Box<dyn`, free `dyn ` trait-object dispatch, `methodName`+json dispatch.
- TS: `: any`, `as any`, `: unknown` in non-test source, `callRust(`-style dispatchers.

`test`/`devtools`/replay-quarantined paths are exempted (explicit quarantine), matching the
manifest's `surface = "quarantined"` operations.
