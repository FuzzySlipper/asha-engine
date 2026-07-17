---
status: current
audience: agent
tags: [ci, guardrails, governance]
supersedes: []
see-also: []
---

# Lightweight guardrail policy

ASHA CI distinguishes consequential blockers from structural pressure. The
machine-readable registry is
[`harness/ci/guardrail-policy.json`](../harness/ci/guardrail-policy.json). It is
an operating index, not architecture truth or product evidence.

## Operating posture

- Authority/import boundaries, generated borders, strict public wire decoding,
  replay/atomicity, accepted-state/data-loss behavior, and ambiguous selector
  fallbacks fail closed.
- Source-shape limits, vocabulary drift, code maps, and generated navigation
  counts warn with an owner and next action. Their underlying compile,
  dependency, contract, and public-root checks remain blocking.
- Broad validator self-tests run when harness policy changes and at scheduled or
  campaign closure; unrelated fast paths skip them.
- Native/provider integration runs when native paths change and at scheduled or
  campaign closure.
- Visible Demo and Studio acceptance belongs to the consumer repository. Engine
  structural or provider evidence cannot close a user-facing task.
- Declarative manifests may remain source truth. Computed timing, receipts,
  failure tails, and per-run results are ignored CI/task artifacts by default.

`./harness/ci/check-fast.sh` is the normal iteration command. Unknown and CI
entrypoint changes expand safely to the full retained inventory. The compact
registry validator is selected only when the registry, workflow, or a
`harness/ci` entrypoint changes.

New blocking gates require a named consequential failure class, owner, bounded
trigger and fallback, representative regression, rough cost, and a condition
for narrowing or removal. A gate with no distinct claim should be merged or
deleted.

## Explicit dispositions

The registry records these decisions directly:

- source-shape pressure: advisory;
- vocabulary: advisory;
- generated code maps/workspace counts: advisory;
- validator negative fixtures: change-triggered and scheduled;
- broad Rust/TypeScript workspaces: change-triggered blockers;
- native integration: native-change-triggered and scheduled/campaign-close;
- replay and reviewed goldens: affected-surface blockers;
- committed computed validation reports: retired;
- downstream live acceptance: consumer-owned;
- duplicate execution wrappers: merged behind collision-checked shared
  execution identity.

## Timing context

The last pre-retirement warm full run selected 16 gates in 99.618 seconds. The
first post-retirement run selected 13 retained gates in 118.955 seconds,
including a 20.36-second native release rebuild. This is not presented as a
speedup: the useful change is that ordinary narrow edits use affected-surface
selection, while the complete run remains below the three-minute warm budget.
See [`ci-feedback.md`](ci-feedback.md) for gate-level and cold-run detail.

The policy-change validation run safely expanded the fast selector to the
policy validator plus all 13 retained gates. It completed in 93.064 seconds
(1.551 runner minutes), with 14 unique gate commands, zero repeats, zero
advisory failures, zero blocking failures, and zero duplicate actual execution
fingerprints. The policy validator itself took 0.040 seconds.

## Two visible-slice calibration

These two recent Studio slices show why strong authority rails and real consumer
acceptance must coexist. Times use task status timestamps. Runner minutes are
the elapsed GitHub workflow time for the listed task heads; local verification
and human review time are not included.

| Slice | Lead time | Validation-only follow-up commits | GitHub runner minutes | Late consequential findings | Escaped runtime defects after close | False-green delivery claims |
| --- | ---: | ---: | ---: | --- | ---: | ---: |
| #5825 visible authored voxel house | 22 h 09 m | 1 (`2a12a65`, roundtrip regression only) | 5.00 across four heads | 1 blocking finding: visible geometry ignored the stored SceneDocument instance transform | 0 known | 1 initial handoff treated visible raw projector geometry as the unified movable scene instance |
| #5845 responsive transform gizmos | 10 h 09 m | 0; both follow-ups changed behavior and tests | 4.02 across three heads | 1 blocking acceptance class across two review rounds: TypeScript/Rust settlement split, then missing renderer-local light preview | 0 known | 1 initial handoff claimed the correct preview/settlement split while the test only checked source shape |

The calibration result is deliberately asymmetric:

- authority-call counting, stale rejection, deterministic readback, and public
  provider regressions deserve blocking local coverage because they caught real
  consequential defects;
- source strings, manifests, hashes, and detached reports did not establish the
  visible result;
- live consumer use and behavior-focused review caught both false-green claims
  before final closure;
- neither slice needed another cross-repository proof framework.

For the next two user-facing slices, record the same six fields in the task
thread. Revisit the registry only if a warning repeatedly predicts a
consequential defect, a blocker repeatedly produces noise, or validation-only
follow-up commits and runner time begin dominating delivery.
