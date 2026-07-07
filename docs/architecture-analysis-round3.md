# Architecture Analysis Round 3 — ASHA

*Date: 2026-07-06. Read-only re-review of the repository at commit `c0c698c7`, comparing against `docs/architecture-analysis-followup.md` (2026-07-04, commit `01be106e`). Scope: which of the follow-up's 7 priority items were implemented after the cleanup/restructuring, and what new pressure points emerged. 63 commits and ~38k inserted lines since the follow-up.*

## Summary

This round is the strongest of the three. Every item on the follow-up's priority list moved, and the two structural inversions it flagged as most dangerous — the TypeScript reference authority slice and the mock in the production barrel — were both resolved in the prescribed direction: FPS authority now lives in Rust (`rule-lifecycle` over `svc-entity-authoring`/`svc-combat`), and the reference/mock surface is behind a separate `@asha/runtime-bridge/reference` entry point with `productAuthority: false` labelling and split evidence lanes. The governance rails built in earlier rounds are visibly doing their job: all 7 reserved crates were activated *with* tightened per-crate allowlists, and every new crate/package created during the voxel-conversion campaign was registered with a lane at birth.

The remaining findings are smaller and mostly of one shape — the discipline that was mechanized for TypeScript hasn't been applied to the Rust side, where a new 4,000-line border monolith is forming and the crate that now owns combat/lifecycle authority has one unit test.

## Scorecard against the follow-up's priority list

### 1. Promote the RuntimeSession authority slice into Rust — **done**

The headline item, and it landed the way the follow-up prescribed:

- `rules/rule-lifecycle` (964 lines, `implementation_status = "active"`) owns the FPS slice: `load_fps_project_bundle()` bootstrap through `svc-entity-authoring`, health CapabilityState, primary-fire application through `svc-combat::apply_fire_intent()`, death → lifecycle/visibility transitions through the owning Rules.
- The facade routes Rust-backed sessions through the bridge authority surface with `rust_bridge` / `native_rust` provenance and named surfaces (`runtime_session.fps.primary_fire.v0`); reference receipts are labelled fixture/reference evidence.
- Evidence lanes are split (`test:evidence:reference` vs `test:evidence:rust`), so "the TS slice becomes the behavioral golden" is now structurally prevented rather than merely warned about. `docs/runtime-session-facade.md` states plainly that the non-claims "no longer mean the FPS demo owns local combat/health/target authority."
- On the TS side, `runtime-session.ts` shrank from 2,718 to 1,632 lines, splitting into `runtime-session-ecrp.ts`, `runtime-session-lifecycle.ts`, `runtime-session-enemy-authority.ts`, and `runtime-session-rust-facade.ts` (1,200 lines — the Rust routing layer).

The direction is also now written down durably: `docs/game-rust-authority-extension-model.md` defines the generic-vs-authored authority split and an explicit list of upstream extension points that do *not* yet exist, and `docs/asha-demo-upstream-gap-audit.md` tracks demo-visible behaviors still owned by TS glue, item by item, with the task numbers that retired each one. Both are honest about non-claims.

### 2. Mechanically tie the codegen IR to Rust protocol types — **first increments done, destination still open**

The suggested first steps landed: `protocol-codegen/src/lib.rs` now serde-serializes real Rust protocol values (policy-view world/command/event/outcome, entity authoring, telemetry, voxel conversion…) and validates the JSON against the IR's interface and variant descriptions (`compare_object_to_interface` / `compare_object_to_variant`). IR-vs-Rust drift on covered types is now a CI failure, not a discipline hope.

Remaining risk: the IR keeps growing by hand (2,558 → 2,786 lines; `protocol-voxel-conversion` added a whole new module) and round-trip coverage is sample-based with **no completeness check** — nothing forces a new IR module or a new field's sample to get a round-trip test. See new finding R3-4. The long-term derive-the-IR-from-Rust destination is unchanged and still right.

### 3. Point consumer boundary checks at `harness/public-surface/ts-packages.json` — **done, all three consumers**

`asha-studio/scripts/check-boundaries.mjs`, `asha-testing/scripts/check-boundary.mjs` (and `check-public-artifacts.mjs`), and `asha-demo/scripts/check-dependency-boundary.mjs` all read the engine's file. The four-way allow-list divergence from finding 10 is structurally closed; engine-side, `check-public-boundary.py` runs inside `check-bridge.sh` under `check-all.sh`.

### 4. Split `renderer-three` / `runtime-session`; make the size convention mechanical — **done**

- `renderer-three/src/index.ts` is a 3-line barrel; implementation lives in `three-renderer.ts` (1,091), `browser-surface.ts` (506), `static-room.ts`.
- `harness/depgraph/check-ts-source-shape.mjs` + `ts-source-shape-policy.json` enforce a 1,600-line source cap and root-barrel-exports-only, wired into `check-ts.sh`. Finding 5 recurred twice because it was convention-only; it is now a check.
- Caveats: 14 packages are grandfathered via `rootBarrelExemptions` (each with a rationale and "retire when split" language), and `runtime-session.ts` itself is exempted from the line cap at 1,632. See new finding R3-5 on ratcheting.

Beyond the ask, the renderer-ownership question from finding 10 was resolved in exactly the pattern-consistent direction the original analysis suggested: new `@asha/render-projection` (framework-free render-frame construction) and `@asha/renderer-host` (browser host facade, backend-neutral API) packages, with `renderer-three` demoted to the concrete WebGL backend and the demo no longer importing it or bare Three.js.

### 5. Border-crate tests — **done**

`protocol-policy-view` 0 → 3 tests, `protocol-telemetry` 1 → 3, `native-bridge` 0 → 3, plus the codegen round-trip tests exercising their serialized shapes from the other side.

### 6. Finish the lint tier — **done**

`noPropertyAccessFromIndexSignature` and `noImplicitReturns` are in `ts/tsconfig.base.json`; `no-floating-promises` and the four `no-unsafe-*` rules are error-level in `ts/eslint.config.mjs`.

### 7. Studio proof-harness/product script separation — **done**

`asha-studio/package.json` is down to 21 scripts with **zero** `proof:*` entries (was 34 of 46).

### Follow-up pressure point: mock in the production barrel — **done at the export level** (residual coupling: R3-2)

`package.json` exposes `./reference`; `reference.ts` is the only export path for `createMockRuntimeBridge`, `createMockRuntimeSession`, and the reference launcher factories. The root barrel exports only the native/selected-backend launchers. Reference identity is self-describing: `REFERENCE_RUNTIME_BACKEND_PROFILE.productAuthority === false`, `not_product_authority` in session identities, fail-closed native selection unchanged.

## Growth since the follow-up was absorbed cleanly

Worth stating because it is the system working as designed:

- **All 7 reserved cells went active with real implementations and tests** (`rule-process` 8 tests, `rule-relationship` 8, `rule-state-machine` 8, `svc-physics` 6, `svc-pathfinding` 6), and their `may_depend_on` allowlists were *tightened* on activation — e.g. the rule crates dropped the speculative generic `core-state`/`core-events`/`core-commands` grants for the specific crates they actually use. `implementation_status = "reserved"` count is now 0.
- **The voxel-conversion campaign crossed the border correctly end to end**: `protocol-voxel-conversion` (contract-steward lane, no voxelization logic) → `svc-voxel-conversion` (1,279 lines, 13 tests) → bridge wiring → generated `voxelConversion.ts` → facade methods that fail closed on reference. Every new crate/package appeared in `ownership.toml` at creation — the scaffold-script/governance hole that produced the `game-workspace` gap two rounds ago has not recurred.
- **CI gained an offline GitHub workflow** (`.github/workflows/offline-ci.yml`, `docs/github-check-gates.md`) formalizing the check gates.

## New findings

### R3-1. `rule-lifecycle` has 1 unit test for 964 lines of promoted authority (small effort, high value)

The crate that now owns the FPS health/death/primary-fire/visibility slice has a single `#[test]`. Its behavior is exercised from TypeScript through the bridge conformance and evidence lanes — but that is through two layers of marshaling, and it is exactly the "catch regressions closest to the source" argument of finding 7, now applied to the most authority-dense crate in the workspace. Neighboring crates show the expected density (`svc-entity-authoring` 16, `svc-voxel-conversion` 13). **Suggestion:** unit-test the bootstrap owner-matrix paths, health/death transitions, render-visibility lifecycle, and rejection paths directly in the crate. Adopt as a working rule: when authority is promoted into a crate, its tests move in with it.

### R3-2. The mock is out of the public API but still in the production module graph (small)

`launcher.ts` — exported from the root barrel — imports `createMockRuntimeBridge` at module scope because `ReferenceGameRuntimeLauncher`/`ReferenceGameRuntimeSession` are *defined* in `launcher.ts` and merely re-exported by `reference.ts`. Consequences: importing the root barrel loads `mock.js` (1,413 lines) at runtime; non-tree-shaking bundlers ship it; and nothing but review prevents a future root export of the reference factory, since the module is already reachable. **Suggestion:** move the `ReferenceGameRuntime*` types and factory into a `reference-launcher.ts` imported only by `reference.ts`, and add a depgraph assertion that the root entry point's module graph cannot reach `mock.js` / `mock-session.ts`. That makes the entry-point split load-bearing instead of cosmetic.

### R3-3. `runtime-bridge-api/src/lib.rs` is the new monolith — the TS shape discipline has no Rust counterpart (small–medium)

At 4,024 lines / 150 KB it is now the largest source file in the repository (it grew +2,125 lines this round), accumulating every payload family: error taxonomy, handles, step/world/save, voxel commands, picking, mesh evidence, FPS/ECRP session payloads, nav, encounter, voxel conversion. This is findings 4/5 recurring on the Rust side of the border, in the crate every backend and the codegen conformance depend on. It has 44 tests and clear `── section ──` seams, so the split is mechanical. **Suggestion:** split along the existing section markers into submodules (`errors.rs`, `handles.rs`, `voxel.rs`, `fps_session.rs`, `conversion.rs`, …) with `lib.rs` as re-exports; and extend the source-shape policy to Rust (the TS checker's cap + exemption model ports directly). Without a mechanical cap, the next campaign adds its payload family to the same file.

### R3-4. IR round-trip coverage has no completeness ratchet (small)

The round-trip tests only cover the types someone remembered to sample. Since the IR is still hand-maintained and actively growing, the gap re-opens one forgotten module at a time. **Suggestion:** add a test that walks `model::all_modules()` and asserts every module (or every interface marked as a Rust-mirrored type) has at least one registered round-trip sample, failing with the module name. This is the same "completeness loop" trick that fixed ownership coverage in round 1. Keep serde-reflection/proc-macro derivation as the eventual destination.

### R3-5. Exemption lists need a ratchet so grandfathering doesn't become permanence (small)

Current debt: 14 `rootBarrelExemptions`, one line-cap exemption (`runtime-session.ts` at 1,632 vs the 1,600 cap), and `game-workspace/src/index.ts` still a 1,034-line single file three rounds after finding 5 first named it. Every exemption carries good rationale prose, but nothing stops an exempted file from growing indefinitely under its exemption. **Suggestion:** record the current line count in each exemption entry and have `check-ts-source-shape.mjs` fail if an exempted file *grows* past its recorded size — shrink-only ratchet, no new work required until someone touches the file anyway. The facade decomposition note inside the policy file ("split when the facade decomposition task is scheduled") should become an actual scheduled task; see R3-6.

### R3-6. `@asha/runtime-bridge` is becoming the god package of the public surface (medium, schedule before the next campaign)

The package is now ~17,000 lines across `src/`, spanning four distinct altitudes: transport (bridge/native/render-decode), launcher, the RuntimeSession semantic facade (session + ecrp + lifecycle + rust-facade + evidence), and game-domain projection helpers (encounter-director, enemy-policy, combat-feedback, nav-readout, browser-fps-input, playable-loop/tick readouts). Authority correctly moved to Rust this round, so unlike the follow-up's version of this concern there is no invariant violation — but every ECRP/FPS campaign adds its public methods to this one package, its root barrel is 160+ export lines, and its own exemption text acknowledges a pending "facade decomposition task." **Suggestion:** the natural cut is the one the original finding 4 named: `@asha/runtime-session` (semantic facade + domain readouts) depending on `@asha/runtime-bridge` (transport). Consumers already import through the root barrel and the public-surface file already models tiers, so the move is a metadata + re-export exercise. Doing it before the next campaign is cheap; after, it won't be.

## Priority order for the next round

| # | Item | Effort | Payoff |
|---|------|--------|--------|
| 1 | R3-1: unit tests in `rule-lifecycle` (and the promote-authority-with-tests rule) | Small | The repo's most authority-dense crate is its least-tested |
| 2 | R3-3: split `runtime-bridge-api/lib.rs`; extend source-shape policy to Rust | Small–medium | Stops the Rust border monolith while the split is still mechanical |
| 3 | R3-2: move reference launcher out of the root module graph; depgraph guard | Small | Makes mock isolation load-bearing |
| 4 | R3-6: schedule the `@asha/runtime-session` facade decomposition | Medium | Cost grows with every campaign; currently a metadata + re-export exercise |
| 5 | R3-4: IR round-trip completeness loop | Small | Same trick that fixed ownership completeness; keeps finding 3 closed as the IR grows |
| 6 | R3-5: shrink-only ratchet on shape-policy exemptions; split `game-workspace` | Small | Prevents grandfathered debt from compounding |
