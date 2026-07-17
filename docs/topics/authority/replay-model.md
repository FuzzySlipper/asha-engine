---
status: current
audience: agent
tags: [replay, determinism, authority]
supersedes: []
see-also: []
---

# Replay model

## Purpose

Replay is the core audit mechanism for agent-written changes.
It allows a change to be tested against prior behavior without a human running the game.

## What is recorded

| Record | When | Authority |
|---|---|---|
| Proposed commands | every tick input phase | non-authoritative |
| Accepted domain events | after validation | authoritative |
| State hash | at configurable intervals | verification |
| Snapshots | on demand or at checkpoints | verification |

For long-term golden regressions, accepted events plus snapshots/hashes are the stronger authority.

Gameplay-fabric reactions add a second, joined evidence unit rather than
flattening module execution into the base `DomainEvent` stream. A reaction frame
records the closed registry/module digests, delivered semantic events, frozen
reads, invocation outputs, proposals and owner receipts, accepted module facts,
diagnostics, and before/after hashes. Pre-commit decision receipts additionally
record Workspace generations, suspension/continuation evidence, routing, and
the final receipt hash.

## Canonical replay target

WASM semantics are the replay authority.
Native builds are used for fast iteration and tooling only.
If native and WASM produce different outputs, the divergence must be classified and tested explicitly.
The current WASM authority surface is intentionally replay-only: `wasm-api` exports
`classify_divergence` and `divergence_class_labels` over `sim-replay` artifacts. It is not a
runtime transport and does not expose init, tick, command submission, render diffs, telemetry,
or raw memory views.

## Determinism requirements

All authoritative randomness comes from `svc-rng` with an explicit seed.
Wall-clock time, ambient randomness, network, filesystem, and DOM access are forbidden inside
the simulation path. Policy code receives deterministic inputs only.

## Replay file format

The in-memory shape is `protocol-replay::ReplayRecord`; the on-disk `.replay`
text encoding is `sim-replay::{encode, decode}` — small, deterministic, and
diff-reviewable:

```text
replay <format_version>
init <hash>
step <index>
cmd <origin> <domain>.<kind> <args...>
event <noun>.<verb> <args...>      # zero or more, accepted steps only
post <hash>
...
snapshot <step> <hash> <snapshot_version>
```

Each step records a proposed command and its `StepOutcome`: accepted (`event`
lines) **or** rejected (a `reject <summary>` line). Hashes are 16-digit hex. A
rejected step's `post` hash equals the prior hash (state unchanged).

Committed real recordings live under `harness/goldens/replays/*.replay`.
Synthetic *format* fixtures (illustrative hashes, not played back) live under
`harness/fixtures/replays/`.

## Recording

`sim-runner::Recorder` drives a `StateStore` forward, recording each command's
outcome and post-step hash. `CheckpointInterval` (`FinalOnly`, `EveryStep`,
`EverySteps(n)`) controls checkpoint frequency; a final checkpoint is always
added. Recording is explicit and opt-in — the normal `run_tick` path keeps no
hidden recording state.

## Running replays

```sh
# Play a golden replay back against current authority logic (0 = ok, 1 = diverged)
cargo run -p replay-tool -- check harness/goldens/replays/<name>.replay

# Re-encode an artifact to stdout for inspection
cargo run -p replay-tool -- show harness/goldens/replays/<name>.replay
```

`harness/ci/check-replays.sh` builds `replay-tool` and checks every golden under
`harness/goldens/replays/` with it.

Gameplay-module playback and verification are intentionally distinct:

- playback restores authority and applies recorded accepted module facts
  without invoking module behavior;
- verification rebuilds the exact static composition, reruns behavior, and
  compares events, reads, invocations, proposals, facts, decisions, routing,
  diagnostics, snapshots, and hashes.

`./harness/ci/check-gameplay-module-conformance.sh` runs this proof for the
external downstream fixture. `GameplayRuntimeHost` snapshots also preserve
trigger active pairs, pending continuation generations, and the bounded
decision/frame evidence ledgers used by downstream replay archives. The same
host snapshot retains the complete gameplay-scheduler queue, fact log, and
outstanding dispatches, so interruption does not discard the canonical
proposal needed for owner routing. The public host hash binds current
EntityStore/prefab authority and scheduler state in addition to module and
evidence state.

`harness/ci/check-wasm-replay.sh` is the authoritative opt-in WASM replay gate. It builds
`wasm-api` for `wasm32-unknown-unknown`, runs `wasm-bindgen --target nodejs`, and reruns the
`@asha/wasm-replay-bridge` tests against the real module. When that module has not been built,
the package's WASM-authority tests skip with an explicit instruction to run
`harness/ci/check-wasm-replay.sh`; those skips are not replay coverage.

## Divergence reports

`sim-runner::playback` re-runs a golden and `sim-replay::diff`s it against the
record, returning the first `sim-replay::Divergence`. The report names the
replay, the diverging step, expected vs. actual, and a likely owner:

| class | likely owner |
|---|---|
| `command-mismatch` | core-commands / sim-replay encoding |
| `accepted-event-mismatch` | sim-validator + sim-applier + core-events |
| `rejection-mismatch` | sim-validator |
| `hash-checkpoint-mismatch` | core-snapshot, or an upstream state change |
| `structural-mismatch` | sim-runner recording / sim-replay assembly |
| `malformed-artifact` | sim-replay encoder/decoder, or a corrupt file |

The report is deterministic and CI-friendly so an orchestrator can route the
failure to the responsible lane — never a bare "replay failed".

## Voxel durability evidence (parallel path; unification deferred)

The first launchable voxel loop has its own **durability** evidence that is
deliberately *separate* from the generic `ReplayRecord` above. A voxel world is
saved as a base **edit log** (`rule-voxel-edit::persist`) optionally compacted into
chunk **snapshots** plus a retained edit tail (`rule-project-bundle::compose`).
`rule-project-bundle::durability` records three voxel state fingerprints for the canonical
fixture sequence and proves the edited world survives a save→compaction→reload cycle:

| Checkpoint | Meaning |
|---|---|
| `postLoad` | fingerprint after the base fixture loads (generation only) |
| `postEdit` | fingerprint after the canonical edit sequence |
| `postReload` | fingerprint after compaction + reconstruction |

Durability holds iff `postEdit == postReload`; a mismatch (tampered snapshot or edit
log) fails **closed** with a classified `DurabilityError`/`SnapshotError` rather than
loading a divergent world. The committed golden lives at
`harness/fixtures/project-bundle/voxel-durability.txt` and is checked by the
`voxel_durability_matches_committed_golden` test in `rule-project-bundle` (run under
`cargo test` / `check-rust.sh`); regenerate it with
`cargo run -p rule-project-bundle --example dump_durability`. The TS devtools read model
`buildVoxelDurabilityModel` / `summarizeVoxelDurability` summarizes the projected
status for a panel.

**Deferred debt (Den task #2440):** this voxel save/reload fingerprint path is *not*
yet unified with the tick-stepped `ReplayRecord`. Unifying them — so a voxel edit
sequence is just another replay stream verified by `replay-tool` — is intentionally
deferred so it does not block the first launchable loop. The voxel state fingerprint used
here is the same FNV-1a `BundleHash` the regenerate-and-replay diagnostic uses, so the
two paths stay directly comparable when they are eventually merged.

## Adding a new golden replay

1. Record the scenario with `sim-runner::Recorder` (see the
   `golden_replay_*` tests in `sim-runner` for the pattern).
2. Save the encoded output to `harness/goldens/replays/<descriptive-name>.replay`.
3. `check-replays.sh` picks it up automatically (it globs the directory).
