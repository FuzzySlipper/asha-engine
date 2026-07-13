# Architecture Analysis Round 5 — ASHA

*Date: 2026-07-12. Read-only re-review of the repository at commit `0aebf75f`, comparing against `docs/architecture-analysis-round4.md` (2026-07-09, commit `cdd72f29`). Scope: the round-4 priority list, and how the Batteries-for-Agents Wave 1 expansion (#5627 and its ~30 child tasks) landed against its two governing design documents — the gameplay-fabric ADR (den: asha/gameplay-fabric-and-federated-rust-authority) and the Unity-to-ASHA translation pressure test (den: asha/unity-to-asha-gameplay-translation). 50 commits and ~117k inserted / ~23k deleted lines since round 4.*

> **Resolution note (2026-07-13):** this document remains the historical
> assessment of `0aebf75f`. Campaign #5746 addressed every finding below and
> is awaiting series review. The finding text is retained to explain why the
> follow-up exists; it must not be read as a list of current defects.

## Summary

The headline is that the Wave 1 architecture substantially delivered its own hardest acceptance criteria — the ones previous rounds would have predicted it would quietly drop. Both downstream pressures the ADR demanded exist as real linked Rust: asha-demo's `primary-fire-effect` module (post-commit systemic reactions, module-owned state, authored bindings with per-instance overrides, trigger consumption, a shared-owner proposal) and asha-rulebench's `rulebench-gameplay-module` (pre-commit React with Suspend/Continue/Cancel dispositions over Transform workspaces). Both consume the engine only through `public-rust/` path dependencies — notably ending rulebench's deliberate zero-dependency posture, which was the single strongest indicator that an upstream path was missing. The conformance regime is the strictest yet: 69/69 stable bridge operations have real probes with zero temporary exemptions, the downstream fixture runs negative composition fixtures in CI, and a live Playwright test drives the browser demo through arm → trigger-enter → close-range fire → complete → snapshot/restore with hash assertions. All six round-4 priorities moved; five are done.

The two significant ways the implementation landed *less* well than designed were both visible from the demo's wiring. The fabric ran as a sidecar Session with TypeScript ferrying facts between two Rust authority islands, and its integrity vocabulary relied on self-attested labels. #5749 replaced the preferred sidecar topology with one statically composed RuntimeSession cell; #5751 made canonical codecs and derived composition identities enforce those claims.

Separately, the wave landed with four unlabeled mega-commits (`docs`, `wave`, `testing`, `unity`), one of which checked 3,379 Cargo `target/` build artifacts into git. The historical labels remain, but current main no longer tracks those outputs; #5760/#5761 add generic ignores, path classification, and a recurrence gate. No history rewrite is required.

## Scorecard against round 4's priority list

### 1. R4-3(b): runtime-bridge facade extraction — **done; the transport package is still the mass center**

`runtime-session.ts` (1,810 lines at its cap) no longer exists. `@asha/runtime-session` now owns facade creation (`createRuntimeSessionFacade`), `transport-contracts.ts`, `facade-ecrp.ts`, `facade-gameplay.ts`, and the new `gameplay-runtime-host.ts` transport contract; its exemption was removed from the TS policy rather than raised. Caveat: `runtime-bridge/src` grew to ~23.4k lines, and its two largest files (`runtime-session-rust-facade.ts` 1,589, `mock.ts` 1,585) sit 11–35 lines under their limits. The extraction relieved the facade; the adapter/rust-facade layer is where the next cap collision will happen.

### 2. R4-1: rename `reference/` / `ReferenceBridge` — **done, with a guard**

The bridge authority body now lives in `runtime-bridge-api/src/authority/` (`runtime_bridge_impl.rs` plus lane modules), and `harness/vocab/check-product-bridge-naming.sh` fails CI if `src/reference` reappears or `ReferenceBridge` occurs anywhere in the bridge crates. The exemption for `runtime_bridge_impl.rs` is narrow (1,612 vs. current 1,611) and carries the trait-impl rationale. This is the suggested fix, executed exactly.

### 3. R4-2: numeric baselines in the Rust source-shape policy — **done**

`rust-source-shape-policy.json` now records `maxLines` per exemption, and the exemption list shrank from six files (17k lines) to three. `model.rs`, the round-4 growth example, no longer exists at all (see item 5).

### 4. R4-3(a): changelog-gated cap raises — **implemented by #5761**

Round 4 offered (a) and (b) as alternatives and (b) landed first. #5761 subsequently added auditable baseline-change records, owner/review/removal metadata, pre-cap warnings, automatic shrink, and negative fixtures for unexplained raises and expired exemptions. Near-cap pressure is now visible in the generated code atlas rather than remaining an ambient CI surprise.

### 5. R4-4: codegen IR endgame — **done, the right way**

The four-round deferral loop ended in the derive direction: `protocol-codegen/src/model.rs` (4,546 hand-maintained lines) was deleted and replaced by `source.rs`, which derives contract shapes from the Rust protocol source and fails closed on underivable constructs (#5508). This retires the last open round-1 finding.

### 6. R4-5: README structural counts — **done, generated**

CI generates the counts (#5509); README states 93 workspace + 1 excluded crate and 24 packages, which matches the tree.

## How Wave 1 crossed the border

The expansion is the largest since the series began, and most of it crossed in the prescribed shape. Evidence highlights, keyed to the ADR's acceptance ladder:

- **Real composition (ladder rung 2).** `public-rust/` grew from one crate to four (`gameplay-module-sdk`, `gameplay-runtime-host`, `gameplay-module-conformance`, `game-rule-extension`), each with `[package.metadata.asha.public-surface]` declaring role and allowed consumers. asha-demo and asha-rulebench consume them by path. The rulebench dependency is a repo-family milestone: the sanctioned parallel authority stack now has its first structural dependency on the engine.
- **Both gameplay pressures (the ADR's generality condition).** Demo: `Observe` over trigger/combat/lifecycle events, module-owned session+entity state with facts/apply/migrate, an owner query (`CurrentTriggerOverlaps`) whose frozen-view hash the module verifies against the accepted enter fact, and a `SetCapabilityActivation` proposal routed to the shared owner. Rulebench: pre-commit reaction windows with typed dispositions. The four invocation families all have non-test consumers (the legacy weapon Transform adapter covers `Transform`/`Guard` paths in-engine).
- **Authority evidence and reproducibility (rungs 3–4).** Reaction frames record registry digests, delivered envelopes, frozen view hashes, invocation output hashes, routing receipts, and before/after state hashes; the live demo test asserts snapshot round-trip (`gameplay_runtime_host.snapshot.v1`) and frame-hash accumulation.
- **Negative gates (rung 5).** `svc-gameplay-fabric/registry.rs` fails closed on duplicate modules/providers, overlapping namespaces, unknown contracts, missing codecs, zero budgets, and provider/manifest mismatches; the downstream fixture exercises the negatives in `check-gameplay-module-conformance.sh`, which runs in `check-all.sh`.
- **A gameplay outcome a player can perceive (rung 1).** `tests/live-ui.spec.mjs` ("gameplay fabric drives the close-range tunnel challenge") drives the real browser build through the full loop and asserts both the HUD label and the module state readout.
- **The rest of the Unity translation surface** landed as projection where the design said projection: audio, particles, billboards (world-space UI), material feedback, telemetry overlay through the shared non-scene envelope; animation controller state in authority with renderer pose sampling; named input actions with a session context stack; time control as tick-cadence commands; kinematic trigger volumes as owner facts with exactly-once enter/exit; tick- and event-conditioned scheduling with recoverable proposal state. `docs/` gained ~25 subsystem documents in the same authority-split format.
- **Migration posture.** The legacy weapon-effect hook survives as two explicitly bounded adapters (`legacy_weapon_transform.rs`, `legacy_weapon.rs`), each with a named deletion condition (#5634) — the right pattern for retiring `game-rule-extension`'s bespoke hook without breaking the existing consumer.

## New findings

### R5-1. The fabric landed as a sidecar session; TypeScript is the bus between two Rust authorities (large; decide the endgame before ferry verbs accrete)

**Resolved by #5749.** The preferred provider is now one statically composed
RuntimeSession cell with one EntityStore and in-process fabric delivery. The
separate gameplay host survives only as an inventoried compatibility quarantine
with named consumers and deletion conditions.

`gameplay-runtime-host` does not depend on `runtime-bridge-api` or any engine session authority. It composes its *own* mini-session: `core-entity` EntityStore, scene/bundle bootstrap, trigger collision sampling, prefab expansion, module state, scheduler. A consumer links it into a second napi addon beside the engine's, and the TS facade drives both (`createRuntimeSessionFacade({ bridge, gameplayHost, mode: 'rust' })`).

Concretely, in asha-demo today:

- The player exists twice. The engine session owns FPS movement/collision/combat; the host owns a mirror entity whose pose TS updates via `actorMovement` deltas so triggers reconcile against it. `gameplay-host-native` hardcodes the mirror world: `PLAYER_ENTITY = 10`, `ENEMY_ENTITY = 20`, `CHALLENGE_TRIGGER_ENTITY = 30`, three hand-entered half-extent boxes, and an inline `scene_artifact()` — a shadow copy of scene data the engine session loads from the real ProjectBundle.
- Owner facts cross through JavaScript. `gameplay_host_observe_weapon_effect(request_json, result_json)` takes the engine addon's weapon-effect output *as JSON handed over by TS* and adapts it to combat gameplay events inside the host. The host's `observe()` accepts any envelope whose contract is registered; nothing verifies the envelope originated in engine authority. TS cannot *mutate* either island, but it can fabricate, drop, reorder, or delay registered semantic events between them — precisely the class of authority participation the ECRP border exists to make impossible, and invisible to both sides' hash evidence because each island is only self-consistent.
- Every future coupling adds a ferry verb. Combat views, inventory proposals, animation timing facts — each will need its own TS-mediated crossing, and each crossing re-raises the fabrication/ordering question.

`docs/gameplay-runtime-host.md` already flags the spatial input as "transitional... not a second world format," and its own framing — "a consumer builds one native provider cell" — names the correct endgame: one consumer-built addon that statically composes the *engine's* authority body (the same `authority/` internals `native-bridge` wraps) together with downstream modules, so owner facts reach the fabric in-process and there is one EntityStore. That is a real build-system and public-surface project (the engine authority crates are not currently public roots), which is exactly why it should be decided and scheduled deliberately rather than discovered after five more ferry verbs exist. An interim hardening — the host verifying ferried envelopes against an engine-session evidence hash chain — is worth pricing but should not be mistaken for the destination.

### R5-2. The integrity vocabulary is self-attested: every hash in the module chain is a hand-typed label (medium; cheap to fix where it matters most)

**Resolved by #5751.** Canonical schema descriptors, typed declared-read plans,
linked SDK/contract inputs, and truthful source provenance now derive the
composition identities. Codec admission covers root, module, owner, proposal,
and replay boundaries.

The ADR: "linked code with a different contract/artifact hash fails session creation," and the manifest-only stand-in pattern is "not an acceptable completion proof." What landed: `sdk_hash: "sha256:gameplay-sdk-v1"`, `artifact_hash: "sha256:demo-primary-fire-gameplay-artifact-v1"`, `schema_hash: format!("sha256:demo.primary-fire-effect.{name}.v1")` — string constants written by the module author. `registry.rs` checks prefix format (`is_hash`) and equality in `validate_provider_links`, but the provider's hashes are *copied from the same manifest* at `linked_from_manifest`, so the equality is tautological on the paved road. A module can change behavior arbitrarily without any hash changing. The declared-read plan hash is likewise a hand-maintained byte string (`b"demo.primary-fire-effect|trigger-entered,...|v2"`).

By contrast the composition registry digest is real (computed from registered content), which is why the load-time `composition_hash` check in the demo host genuinely binds TS input to linked Rust. Extend that property down the chain, in order of value:

1. **Schema hashes**: compute from a canonical schema descriptor. The #5508 source-derivation machinery in `protocol-codegen` already knows how to canonicalize Rust type shapes.
2. **Read-plan hashes**: derive from the typed `GameplayRuntimeDeclaredReadPlan`/manifest structures instead of a parallel byte string an agent must remember to bump.
3. **Artifact/source hashes**: either compute at build time (a `build.rs` hashing the crate source is enough to make "did the linked code change" answerable) or rename the fields (`*_label`) so the evidence stops claiming integrity it doesn't have. Fail composition on the literal placeholder pattern if renaming is deferred.

### R5-3. The wave landed as four unlabeled mega-commits, one of which committed 3,379 build artifacts (hygiene; purge is trivial and urgent)

**Resolved for current main by #5760/#5761.** The old commit labels remain as
history, but tracked build/cache output has been removed, fixture builds use a
shared disposable target outside fixture source, and CI rejects classified
tracked output. Rewriting repository history remains explicitly out of scope.

Every campaign since the series began landed as task-referenced `type(scope): subject (#NNNN)` commits — round 4 called the resulting history the strongest evidence the architecture was teaching its contributors. Wave 1, the largest campaign ever, landed as `docs` (Jul 11), `wave` (77.6k insertions), `testing`, and `unity` (13.5k insertions) — no task references, no scopes, ~100k lines. The planning rigor existed (the den docs enumerate #5595–#5677 with owners); the history just doesn't record which commit delivered which task, which is what future rounds and bisects navigate by.

At the reviewed commit, `testing` had checked in `harness/fixtures/gameplay-module-sdk/downstream-module/target/` plus two `__pycache__/*.pyc` files. Those paths are no longer tracked. The real downstream Cargo workspace, its lockfile, committed generated contracts, goldens, and inspectable evidence remain source; compiler/cache output does not.

### R5-4. Module authoring boilerplate is ~4:1 against gameplay meaning, and topology is declared in five places (medium; this is the "parallel engine" pressure gauge)

**Resolved by #5752.** The public SDK now supplies typed serde codec,
configuration, state-adapter, and topology construction helpers. The downstream
fixture demonstrates the reduced declaration path while conformance still checks
the expanded closed topology and malformed composition failures.

The demo module is 767 lines: roughly 140 lines of behavior and state transitions, and ~450 lines of manifest/provider/codec/adapter declaration. Specific taxes, all visible in a file a downstream agent will copy as the template:

- Subscription/read topology is declared in **five places** that must agree by hand: manifest `subscriptions`, manifest `invocations` (`read_requirements`), manifest `read_views`, the host's `GameplayRuntimeDeclaredReadPlan` list, and the read-plan hash string. The registry cross-validates some pairs at composition, but the author writes all five.
- `GameplayTypedModuleStateAdapter` requires eight encode/decode methods that are all serde one-liners.
- Each event codec is ~15 lines of closure plumbing for `serde_json::to_vec`/`from_slice`.
- Trait methods returning `&GameplayContractRef` force the `static_ref` `OnceLock` + `panic!("unknown static contract")` pattern — the SDK's signature choice becoming downstream folklore.
- Configuration field metadata (`name`/`value_type`/`required`) is hand-listed per field, duplicating the serde struct.

The rulebench module repeats the same ~300-line skeleton. The ADR's own test — "if steps 2–8 require bespoke engine source edits, the architecture has failed" — passes; no engine edits were needed. But the translation doc's sharper test — the governed path must be *genuinely easier* than building a parallel system — is where a 4:1 ceremony ratio erodes the margin. A serde-default codec constructor, derived state adapters, manifest builders that derive subscriptions from registered invocations, and by-value contract returns would cut the demo module roughly in half without touching the governance semantics. Do this before more downstream modules clone the current template.

### R5-5. `gameplay-runtime-host` is guarded almost entirely from downstream; its dedicated gate is orphaned (small)

**Resolved by #5757.** Direct host integration coverage now exercises the final
one-cell runtime/fabric lifecycle and emits bounded evidence. The dedicated gate
is part of the main gate with shared execution attribution rather than an
orphaned script.

The host — 2,690 lines owning snapshots, scheduler recovery, trigger reconciliation, prefab expansion, and owner routing — has four inline unit tests and no `tests/` directory; `scheduler.rs` and `owner_router.rs` have none. Real coverage comes from the harness fixture, the demo conformance binary, and the live Playwright spec — the first two in engine CI, the last in a sibling repo. Meanwhile `harness/ci/check-gameplay-runtime-host.sh` (which runs the host's tests plus the fixture plus both TS packages) is called by nothing: not `check-all.sh`, not the workflow. `cargo test --workspace` in `check-rust.sh` incidentally runs the four inline tests, so nothing is silently broken — but the crate at the center of R5-1 deserves host-level integration tests (snapshot/restore under scheduler load, trigger reconcile across restore, owner-router rejection paths), and the script should either be wired into `check-all.sh` or deleted so the gate inventory stays honest.

### R5-6. The Unity gap tracker's summary table contradicts its own sections (trivial)

**Resolved by #5760.** The summary, detailed input/scheduler/trigger sections,
and current priority prose now agree with the implemented Wave 1 baselines.

`docs/unity-gap-analysis.md` row 8 says World-space UI **MISSING** while section 8 says **BASELINE IMPLEMENTED ✅ (Wave 1)** (billboards landed, with `render-billboard` contract validation). A cold-start agent reads the table first. Audit the other rows against their sections while fixing it — or generate the table from section headings, the R4-5 trick.

## Original priority order and campaign disposition

| Original item | Owning task | Current disposition |
|---|---|---|
| R5-3: tracked build/cache output | #5760/#5761 | Implemented; current main is clean and gated |
| R5-1: one native RuntimeSession cell | #5749 | Implemented; downstream migration follows in #5732 |
| R5-2: computed identities and codec ingress | #5751 | Implemented |
| R5-4: module authoring ergonomics | #5752 | Implemented |
| R5-5: direct host coverage and honest gate | #5757 | Implemented |
| R5-6: Unity tracker reconciliation | #5760 | Implemented |

Campaign #5746 also resolved the broader retrospective findings around
renderer authority (#5747), canonical proposal routing (#5748), wave barriers
(#5750), bridge ports/code generation/wire validation (#5753–#5755),
compatibility quarantine (#5756), scheduler authorization (#5758), proof
identity/execution consolidation (#5759), and structural-pressure governance
(#5761). Each campaign task has one pushed task-referenced commit and a Den
task-to-SHA handoff; earlier unlabeled history was not rewritten.
