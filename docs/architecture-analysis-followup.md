# Architecture Analysis Follow-up — ASHA

*Date: 2026-07-04. Read-only re-review of the repository at commit `01be106e`, comparing against `docs/architecture-analysis.md` (2026-07-01, commit `af052878`) and the Den ADR `asha/ecrp-vocabulary-taxonomy`. Scope: which of the 12 findings were implemented, how the ECRP vocabulary landed, and what new pressure points emerged since.*

## Summary

The high-leverage enforcement findings — which the original analysis called the most valuable work — were implemented, several following the suggested mechanics almost verbatim. The ECRP vocabulary landed cleanly in docs and on the public TS surface, and the new `asha-demo` was started in exactly the shape the taxonomy doc prescribed. Two findings did not move: the codegen IR (finding 3) and border-crate tests (finding 7). One significant new pressure point emerged: a TypeScript "reference authority slice" has grown inside `runtime-bridge` that strains the repo's core invariant (details in the final section).

## Finding-by-finding scorecard

### Implemented

- **1. `may_depend_on` enforcement — done.** `harness/depgraph/verify-rust-deps.sh` now treats `may_depend_on` as an allowlist, including the suggested explicit `"unrestricted"` escape value, with the route-through-governance error message.
- **2. TS ownership completeness — done.** `verify-ts-deps.sh` gained the completeness loop plus `ownership_exempt`, and `game-workspace` is registered (`lane = "ts-game-workspace"`, `governance/ownership.toml:502`).
- **4. runtime-bridge split — done, with a caveat.** `runtime-bridge/src/index.ts` is now a 91-line barrel over `errors` / `bridge` / `mock` / `native` / `launcher` / `render-decode` — the exact seams suggested. Caveat: the mock still ships in the production barrel (`export * from './mock.js'` in the root export); the suggested separate reference entry point was not taken. See the pressure-point section.
- **6. Placeholder crates — done (marked, not deleted).** `implementation_status = "reserved"` marks 7 entries in `ownership.toml`, so tooling and orchestrators can filter them.
- **8. Root hygiene — done.** Committed logs/pids/capture JSON removed; `.gitignore` covers the patterns.
- **9. Doc consolidation — done.** `docs/architecture-overview.md` is now a 29-line pointer declaring `docs/design.md` canonical and Den the source of truth for current work.
- **10. Engine-declared public surface — done engine-side; validation loop closed for one of three consumers.** `harness/public-surface/ts-packages.json` exists with per-package tier/status/ownership-key/compatibility-marker *and* per-consumer policies, plus `check-public-boundary.py`. `asha-demo` validates against it (`scripts/check-dependency-boundary.mjs` reads `../asha-engine/harness/public-surface/ts-packages.json`). Gap: **asha-studio and asha-testing still do not** — studio's `check-boundaries.mjs` predates the file and does not reference it, so those consumers still maintain independent boundary truths.
- **11. Consumer manifest cleanup — mostly done.** Studio's `knownLimitations` task journal (~20 essays) and the contradictory `allowedAshaPackages` block are gone, and the suggested "docs must cite real commands" check exists (`check:docs-scripts` / `check-doc-script-references.mjs`). Partial: 34 of studio's 46 npm scripts are still `proof:*` — the proof-harness/product script separation was not done.
- **12. Rusty-view formalization moves — the big ones landed.**
  - *Move 1 (generated lint boundaries):* ESLint boundaries are now generated from `ownership.toml` (`harness/depgraph/generate-ts-eslint-boundaries.py` → `ts/eslint-boundaries.generated.mjs`, do-not-edit header).
  - *Move 2 (two-axis tags):* the `layer =` axis is in `ownership.toml` (42 `layer`/`type` entries).
  - *Move 3 (barrels):* public-surface metadata carries `rootExportOnly`; root-export-only is stated and checked at the border.
  - *Move 4 (scaffold script):* `ts/scripts/new-package.mjs` + `harness/depgraph/check-package-generator.sh` exist — closing the exact hole that created the `game-workspace` governance gap.
  - *Move 5 (type-aware lint tier):* partial — `noImplicitReturns` added, `no-explicit-any` on; `noPropertyAccessFromIndexSignature` and the promise-safety rules (`no-floating-promises`, `no-unsafe-*`) still absent.

### Not moved

- **3. Codegen IR — untouched and growing.** `protocol-codegen/src/model.rs` went from ~2,400 to 2,558 lines (90 → 96 KB). No round-trip tests, no serde-reflection, no mechanical tie to the Rust protocol types. This was ranked #3 in priority and remains the only second source of truth on the border — and it is accumulating risk precisely while the ECRP campaign adds protocol surface.
- **7. Border-crate tests — essentially unchanged.** `protocol-policy-view` has 0 tests, `protocol-telemetry` has 1, `bridge/native-bridge` has 0.
- **5. Single-file convention — not holding.** `renderer-three/src/index.ts` is 2,002 lines, `game-workspace/src/index.ts` unchanged at 1,034, and a new 2,718-line file appeared in `runtime-bridge` (see below).

## ECRP vocabulary adoption

The taxonomy landed the way the ADR intended — as steering vocabulary, not rename churn:

- **New docs speak ECRP fluently.** `ecrp-fps-object-model.md`, `ecrp-capability-rule-ownership.md`, `ecrp-runtime-session-readout.md`, `entity-definition-schema.md`, `runtime-session-facade.md`; and `vocabulary-compatibility.md` explicitly documents the legacy-name mapping — exactly the migration posture the ADR asked for.
- **The public TS surface uses the new terms.** `RuntimeSessionFacade`, `loadEcrpProject` (ProjectBundle + `EntityDefinition[]` + SceneDocument placements), `readEcrpRuntimeReadout` (Entity/CapabilityState/event readouts).
- **Rust internals were originally not churned wholesale.** That has since been narrowed by breaking vocabulary refactors: `core-scene` now uses `SpatialSessionState`, and the protocol contract crate has moved to `protocol-project-bundle`. Remaining lower service/rule bundle names are tracked as explicit compatibility work, not ambient product vocabulary.
- **Rust gained real ECRP substrate.** `svc-entity-authoring` now has `validate_and_apply_rule_owned` with a closed Rule-owner/mutation matrix (EntityBootstrap, LifecycleRule, TransformRule, MovementRule, CollisionRule, RenderProjectionRule, RelationRule) — a concrete answer taking shape for the ADR's open question #2 (capability mutation rights across rule lanes).
- **`asha-demo` matches its prescribed classification.** It exists as a real ASHA Game Project with `catalogs/`, `levels/`, `policies/`, `replays/`, `assets/`, `asha.game.toml`, and app composition over public packages only. Its script surface is product-shaped (`dev` / `build` / `test` plus two checks, zero `proof:*`) — the finding-11 lesson applied from day one.

## New pressure point: the TS reference authority slice

The FPS campaign's actual authority currently lives in TypeScript. `runtime-bridge/src/runtime-session.ts` is 2,718 lines, flanked by `encounter-director.ts` (613), `combat-feedback.ts` (500), and `enemy-policy.ts` (334) — health CapabilityState, primary-fire combat resolution, lifecycle/win-loss state, and enemy policy all implemented in the reference RuntimeSession inside the *transport* package. The docs are honest about it (`ecrp-capability-rule-ownership.md` says this slice "should be promoted into native protocol/state lanes through narrow follow-up work," and the facade reports typed non-claims), but three things compound:

1. **It re-creates finding 4 one level up.** The `index.ts` monolith was split, and then a bigger one grew in the same package within days. Game-domain modules in the transport layer is precisely the "launcher is a layer above the bridge" concern, now with combat rules attached.
2. **It inverts "Rust owns authority."** Today the demo's accepted truth — including the committed lifecycle fixture hashes quoted in `runtime-session-facade.md` — is produced by a TS reference implementation, with Rust expected to catch up. The longer the TS slice is the behavioral golden, the more the eventual native implementation becomes "match the mock" rather than "the mock mirrors Rust."
3. **The mock still ships in the production barrel.** The reference backend was not isolated behind a separate entry point, and now that mock is 1,031 lines plus the session slice, the fail-closed story leans harder on it than before.

## Priority order for the next round

| # | Item | Effort | Payoff |
|---|------|--------|--------|
| 1 | Promote the RuntimeSession authority slice (health/combat/lifecycle/enemy policy) into Rust protocol/state lanes | Medium–large, but grows with delay | Restores the core invariant; stops the TS slice becoming the behavioral golden |
| 2 | Finding 3: mechanically tie the codegen IR to Rust protocol types (round-trip tests first) | Medium | Same border, same class of risk as item 1 — both are "second implementation as de-facto truth" |
| 3 | Point asha-studio and asha-testing boundary checks at `harness/public-surface/ts-packages.json` | Small | Closes the consumer-validation loop finding 10 designed; asha-demo already proves the pattern |
| 4 | Split `renderer-three/index.ts` and `runtime-session.ts`; adopt the exports-only-barrel convention with a size lint | Small–medium | Finding 5 keeps recurring because it is convention-only; make it mechanical |
| 5 | Finding 7: serialization round-trip tests for `protocol-policy-view` / `protocol-telemetry`; native-bridge smoke test | Small | Unchanged from original analysis |
| 6 | Finish lint tier (move 5): `noPropertyAccessFromIndexSignature`, `no-floating-promises`, `no-unsafe-*` | Small | Cheap; `parserOptions.project` already set |
| 7 | Studio proof-harness/product script separation (finding 11 remainder) | Small | Product legible as product; 34 of 46 scripts are still `proof:*` |
