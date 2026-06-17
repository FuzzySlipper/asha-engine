# Consumer Compatibility Surface

Status: task #2536 compatibility surface for `asha-demo` and future downstream consumers. This is not a public-registry semver promise.

## Purpose

ASHA is still local-path / in-house engine substrate work, but downstream consumers need a durable place to answer:

- which generated contract surface am I using?
- which runtime bridge facade surface am I using?
- where is the changelog/migration note for a breaking border change?
- what should a consumer do when the surface is incompatible?

The answer is split between machine-readable package metadata and this human-readable changelog/process document.

## Machine-readable metadata

Tier 1 consumer packages carry `asha.compatibility` in `package.json` and a package-local `compatibility.json` file.

| Surface | Metadata file | Compatibility version | Role |
|---|---|---|---|
| `@asha/contracts` | `ts/packages/contracts/compatibility.json` | `contracts.v0` | Generated semantic DTO/type border from Rust protocol crates. |
| `@asha/runtime-bridge` | `ts/packages/runtime-bridge/compatibility.json` | `runtime-bridge.v0` | Transport-neutral runtime facade, manifest-backed operation vocabulary, typed errors. |

The metadata schema is intentionally tiny for now:

- `schemaVersion`: metadata schema version. Current value: `1`.
- `surface`: package/surface name.
- `compatibilityVersion`: opaque ASHA compatibility marker for consumers and conformance artifacts.
- `packageVersion`: current package version; not a registry promise yet.
- `sourceOfTruth`: where agents should make source changes.
- `changelog`: section in this document for surface-specific compatibility entries.
- `migrationNoteTemplate`: section in this document that breaking changes must fill in.
- `failClosedPolicy`: what consumers should do when the version or operation is incompatible.
- `pinningGuidance`: how `asha-demo` should record the surface it tested.
- `breakingChangeRequires`: minimum evidence checklist for border-breaking changes.

`harness/public-surface/check-public-boundary.py` validates that Tier 1 packages publish this metadata and that package metadata mirrors the compatibility version.

## Generated contract compatibility log

### `contracts.v0` — initial local-path boundary

Status: current initial compatibility marker for the committed generated TypeScript contracts in `@asha/contracts`.

Source of truth:

- Rust protocol crates under `engine-rs/crates/protocol/*`.
- Generated TypeScript under `ts/packages/contracts/src/generated/*`.
- Generator command: `cargo run -p protocol-codegen`.
- Check command: `bash harness/ci/check-contracts.sh`.

Consumer behavior:

- Consumers import only from `@asha/contracts` root export.
- Consumers do **not** import generated file paths directly.
- Consumers do **not** copy generated DTOs into their own repo as a forked truth source.
- `asha-demo` records the ASHA git commit plus `contracts.v0` in conformance artifacts until #2536-style metadata is copied into downstream artifacts.

Breaking generated-contract changes require a migration note using the template below.

Additive notes under `contracts.v0`:

- #2563 adds a public `view` generated module for deterministic camera/view evidence: `CameraHandle`, camera pose/basis/projection/viewport DTOs, first-person camera input envelopes, and projection snapshots with column-major matrices. The compatibility marker remains `contracts.v0` because the change is additive and consumers that do not import the new types are unaffected.

## Runtime bridge compatibility log

### `runtime-bridge.v0` — initial local-path facade boundary

Status: current initial compatibility marker for the committed `@asha/runtime-bridge` facade and runtime bridge manifest family.

Source of truth:

- Bridge manifest: `engine-rs/crates/bridge/runtime-bridge-api/bridge-manifest.toml`.
- Generated conformance descriptor: `ts/packages/runtime-bridge/src/generated/conformance.json`.
- Runtime facade root export: `ts/packages/runtime-bridge/src/index.ts`.
- Check command: `bash harness/ci/check-bridge.sh`.

Consumer behavior:

- Consumers import only from `@asha/runtime-bridge` root export.
- Consumers never import `@asha/native-bridge` or `@asha/wasm-replay-bridge` as a runtime transport.
- Required operations must either be present on the facade or fail closed with classified `RuntimeBridgeError` kinds.
- `native_unavailable` and `operation_unimplemented` are acceptable diagnostics for missing native implementation during prototype/conformance work only when the task records an explicit gap; consumers must not silently fall back to raw transports.

Breaking facade/operation changes require a migration note using the template below.

Additive notes under `runtime-bridge.v0`:

- #2564 adds three stable camera/view operations to the manifest-backed facade: `create_camera` / `createCamera`, `apply_first_person_camera_input` / `applyFirstPersonCameraInput`, and `read_camera_projection` / `readCameraProjection`. Native remains fail-closed with `operation_unimplemented` until a real native implementation lands; the mock/reference paths provide deterministic boundary evidence only. The compatibility marker remains `runtime-bridge.v0` because the change is additive.

## Consumer pinning guidance

Until ASHA has registry/package publication, `asha-demo` pins by local path plus ASHA git commit:

```json
{
  "dependencies": {
    "@asha/contracts": "file:../asha/ts/packages/contracts",
    "@asha/runtime-bridge": "file:../asha/ts/packages/runtime-bridge"
  }
}
```

Conformance artifacts should record:

- ASHA git commit or source path being tested;
- `@asha/contracts` compatibility version from `compatibility.json`;
- `@asha/runtime-bridge` compatibility version from `compatibility.json`;
- any explicit compatibility gaps or migration tasks.

If a consumer sees an unknown `compatibilityVersion`, missing metadata file, missing required operation, or breaking-change log entry without a migration note, it should fail closed and file an ASHA engine compatibility/request task instead of papering over the gap.

## Migration note template

Use this template for every breaking generated-contract or runtime-bridge facade change.

```markdown
### <compatibility-version> — <short title>

Surface: `@asha/contracts` or `@asha/runtime-bridge`
Change type: breaking | additive | deprecation | fail-closed behavior change
Introduced by: <task id / commit / PR>
Source files: <Rust protocol crate, bridge manifest, facade file, generator>

What changed:
- ...

Why this is engine-level:
- ...

Downstream impact:
- `asha-demo`: ...
- future consumers: ...

Required migration:
1. ...
2. ...

Required evidence:
- generated contract sync / bridge check command:
- fixture/golden update:
- downstream typecheck/conformance command:

Fail-closed expectation:
- what consumers should reject rather than silently adapting:
```

## Generated-file hand-edit policy

Generated contract files are committed for worker convenience but never hand-edited.

Current generated files include `ts/packages/contracts/src/generated/*.ts` and carry a generated header. The source of truth is Rust protocol code plus `protocol-codegen`.

If a generated contract output is wrong:

1. change the Rust protocol source or generator;
2. run `cargo run -p protocol-codegen`;
3. commit source and generated output together;
4. update this compatibility document and migration notes if the consumer-visible border changed;
5. run `bash harness/ci/check-contracts.sh`.

Manual edits to generated output are drift, not a shortcut.
