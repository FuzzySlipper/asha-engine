# Protocol border: downstream consumer map

The generated TypeScript contracts in `ts/packages/contracts/src/generated/`
are the only sanctioned way for TypeScript packages to talk about the Rust
authority core's border. This note records which packages are expected to
consume each generated family in later phases, so a contract-shape change can be
impact-assessed before it lands.

`@asha/contracts` itself depends on no other workspace package; consumption flows
strictly outward.

| Generated family | Surface | Expected downstream consumers (later phases) |
| --- | --- | --- |
| `ids.ts` | Branded IDs (`EntityId`, `SubjectId`, `ProcessId`, `ModeId`, `SignalId`, `TagId`) + constructors | Every consumer below — IDs are the shared vocabulary. |
| `script.ts` | `ScriptView`, `Command`/`CommandEnvelope`/`CommandKind`, `ScriptRejection`, `ScriptOutcome` | `script-sdk`, `script-host`, `policy-core`, `policy-examples` (author/validate commands against a read-only view); `runtime-bridge` (command ingress, view egress); `devtools` (inspect commands & rejections). |
| `render.ts` | `RenderHandle`, `Transform`, `RenderMetadata`, `RenderDiff`, `RenderFrameDiff` | `renderer-three` (apply retained-mode diffs); `runtime-bridge` (diff decode/transport); `devtools` (inspect frames); `ui-dom` (debug overlays). |
| `replay.ts` | `StepIndex`, `ReplayHash`, `REPLAY_FORMAT_VERSION`, `DomainEvent`, `ReplayStep`, `SnapshotMeta`, `ReplayRecord` | replay tools (`replay-tool`, `snapshot-diff`); `devtools` (divergence views); `wasm-replay-bridge` (WASM replay authority). |

## Importability proof

`ts/packages/contracts/src/smoke.ts` is the import/typecheck smoke for the Phase
2 exit criterion "a TypeScript package can import generated branded IDs and
command unions." It imports the generated branded IDs and the `Command` union
through the package's public entry point (`./index.js` → `@asha/contracts`) and
constructs real values, so `tsc` fails if the surface stops being importable or
usable. It pulls in no policy, renderer, UI, bridge, Electron, or browser
globals.

A dedicated consumer fixture in `script-sdk` is intentionally **not** added: the
contracts package proves command-union importability on its own, and the
workspace has no project-reference wiring that would let a sibling package
typecheck against `@asha/contracts` source without first building it.
