# Bridge manifest format, generated glue, and conformance

Companion to `bridge-manifest.toml`. Governs the curated-manifest + generated-boring-glue +
handwritten-bodies middle path (ADR 0006).

## 1. Why a manifest (and what it is not)

The manifest is a small, hand-reviewed list of allowed bridge operations with typed input/output,
an error channel, capability ownership, and lifetime notes. It makes the public runtime border
diffable and review-gated.

It is not:

- a generic RPC registry;
- a `callRust(methodName, json)` dispatcher;
- a place for `serde_json::Value`, `any`, or `unknown` payloads;
- a second handwritten schema layer (semantic types live in protocol crates);
- permission for generated code to decide policy, validation, authority, or mutation.

Adding an operation is a deliberate boundary change requiring contract-steward review.

## 2. Schema

```text
[manifest]
version            : int (currently 1)
owning_crate       : "runtime-bridge-api"
facade_package     : "@asha/runtime-bridge"
native_package     : "@asha/native-bridge"
wasm_replay_package: "@asha/wasm-replay-bridge"
error_type         : single typed error enum for all operations
error_families     : complete snake_case RuntimeBridgeError kind list
handle_types       : opaque bridge-owned handle type names
default_max_input_bytes : positive default request bound
default_max_output_bytes: positive default response bound

[[capability]]
id                : snake_case, unique
interface         : generated TypeScript interface name, unique
property          : generated RuntimeBridgePorts property, unique
initialization    : "requiresEngine" | "createsEngine"
project_bundle    : "retainedAcrossLoadUnload" | "ownsLoadUnload"
snapshot_hash     : named readout/hash family governed by the cell
resource_lifetime : "session" | "frame" | "mixedExplicitAndSession"
operations        : exact operation names; every operation appears once globally

[[operation]]
name              : snake_case, unique
surface           : "stable" | "quarantined"
quarantine_reason : required iff surface == "quarantined"
input             : exactly one protocol ref, declared handle, or Unit
output            : exactly one protocol ref, declared handle, RuntimeBufferView, or Unit
errors            : must equal manifest.error_type
buffers           : required when the operation lends, borrows, or releases a buffer
summary           : one-line human description
facade_method     : optional explicit camelCase public method override
facade_input      : optional bridge::/contracts::/session:: semantic type override
facade_output     : optional bridge::/contracts::/session:: semantic type override
max_input_bytes   : optional positive operation-specific request bound
max_output_bytes  : optional positive operation-specific response bound
```

`harness/bridge/validate-manifest.py` enforces:

1. Every operation has exactly one input and output; there are no variadic or method-name payloads.
2. Type references are bounded and typed. Opaque escape-hatch tokens are rejected.
3. Every operation uses the common error type and a valid surface classification.
4. Buffer operations declare their lifetime contract.
5. Operation names and error families are unique snake_case values.
6. Capability ids, interfaces, and properties are unique. Their operation lists are an exact,
   non-overlapping cover of the manifest.
7. Facade overrides point only to existing semantic owners: `bridge::`, `contracts::`, or
   `session::`.
8. Stable operations have exactly one handwritten TypeScript binding signature and one concrete
   Rust `#[napi]` export. Missing, duplicate, extra, or mismatched wiring fails before runtime.
9. Generated artifacts match the committed manifest byte-for-byte.
10. Every operation resolves positive request/response limits and generated wire type ownership.

## 3. Generated boring glue

`harness/codegen/bridge-emit.py` emits only transparent, mechanical parity surfaces:

| Artifact | Generated path | Shape |
|---|---|---|
| Operation descriptors | `ts/packages/runtime-bridge/src/generated/operations.ts` | tagged descriptor union, wire type ownership, byte limits, error families, stable/quarantined and native-wiring inventory |
| Capability facade | `ts/packages/runtime-bridge/src/generated/surfaces.ts` | grouped typed port interfaces, root bridge, ports, and lifecycle contracts |
| Native TS declaration | `ts/packages/native-bridge/src/generated/addon-surface.ts` | exact stable export-name union checked against handwritten semantic signatures |
| Runtime Rust metadata | `engine-rs/crates/bridge/runtime-bridge-api/src/generated/mod.rs` | typed operation/capability binding inventory |
| Native Rust metadata | `engine-rs/crates/bridge/native-bridge/src/generated/mod.rs` | exact stable native-export inventory plus request/response limits |
| Conformance snapshot | `ts/packages/runtime-bridge/src/generated/conformance.json` | machine-readable capability, signature, and wiring snapshot |
| Native reference | `engine-rs/crates/bridge/native-bridge/src/generated/EXPORTS.md` | inspectable operation/capability/type table |

Run `python3 harness/codegen/bridge-emit.py --write` to regenerate all seven artifacts. The
`--check` mode runs in `check-bridge.sh` and fails on source/artifact drift. The manifest validator
separately compares generated declarations with handwritten TypeScript signatures and concrete
Rust exports, so a generated name cannot masquerade as real wiring.

There is no broad reflection or dynamic dispatcher. One manifest operation produces one bounded
method declaration and one row in each relevant inventory.

## 4. Handwritten semantic boundary

Semantic operation bodies stay handwritten and reviewed: engine/session lifecycle, validation,
state-mutation phases, render-diff collection, buffer allocation and disposal, error
classification, and native/WASM divergence reporting. The generated facade interface makes the
handwritten mock and native adapters structurally accountable; it does not implement them.

Facade overrides map a manifest protocol reference to an existing semantic type owner. The
protocol code generator emits recursive runtime wire validators from the same Rust-derived schema
IR that emits TypeScript DTOs. Manifest codegen binds each operation to a generated, custom,
handle, or unit validator. Custom transition DTOs keep explicit exact-shape validators until they
move into protocol ownership. Any Rust/N-API conversion with semantic choices remains next to its
handwritten native body.

The public native adapter validates the canonical input before invocation and the canonical output
before returning it to a consumer. Rust JSON entrypoints independently enforce generated operation
limits, deserialize into the authoritative DTO, and compare the decoded canonical shape to reject
unknown nested fields. A JSON parse plus a TypeScript assertion is never the final decode step.

Native failures cross N-API as a bounded version-1 envelope with `code`, `operation`, `path`,
`retryable`, `message`, bounded `details`, and `provenance: native_rust`. The TypeScript facade
validates this envelope and exposes those fields on the backward-compatible `RuntimeBridgeError`;
it does not recover classifications from prose prefixes.

## 5. Conformance shape

The bridge and conformance gates combine four kinds of evidence:

1. Generated TypeScript exactness: the handwritten facade and native binding declaration satisfy
   the generated operation and capability surfaces.
2. Native export exactness: each stable manifest operation has one concrete `#[napi]` export; no
   non-manifest export is silently added.
3. Real operation probes: every stable operation is called by a named assertion in an executed
   compiled Rust or native-transport suite.
4. WASM replay conformance: deterministic replay fixtures use the WASM verification path and
   classify divergence explicitly.

Mock behavior and generated declarations are useful structural checks, but they do not count as
real operation evidence.

## 6. Extending the bridge

To add one operation without creating parallel inventories:

1. Add its generated protocol DTOs, or an explicit existing semantic facade override, and one
   `[[operation]]` entry in `bridge-manifest.toml`.
2. Put it in exactly one capability operation list. A genuinely new capability must declare its
   initialization, ProjectBundle, hash/readout, and resource-lifetime contract.
3. Run `python3 harness/codegen/bridge-emit.py --write`; inspect and commit every generated diff.
4. Implement the handwritten Rust semantic body and `#[napi]` export, its exact
   `NativeAddonBindings` signature, and the runtime adapter. Do not edit generated files.
5. Add a real named conformance probe that calls the export and asserts authority-visible
   behavior. Stable operations cannot stop at generated-only or mock-only evidence.
6. Run `bash harness/ci/check-bridge.sh` and
   `python3 harness/conformance/validate.py --write-report`, then the focused Rust and TypeScript
   tests for the owning capability.

The checks reject a forgotten capability assignment, duplicate operation, stale generated file,
signature mismatch, absent native export, or stable operation without a real probe before it can
be advertised downstream.

## 7. Mechanical guardrails

`harness/bridge/check-bridge-guardrails.sh` rejects opaque escape hatches in stable bridge
surfaces. Rust rejects `serde_json::Value`, `Box<dyn`, free trait-object dispatch, and method-name
JSON dispatch. TypeScript rejects `any`, `unknown`, and call-by-name tunnels in non-test source.

Test, devtools, and explicitly replay-quarantined paths retain their bounded exemptions.
