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

## Canonical replay target

WASM semantics are the replay authority.
Native builds are used for fast iteration and tooling only.
If native and WASM produce different outputs, the divergence must be classified and tested explicitly.

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

## Adding a new golden replay

1. Record the scenario with `sim-runner::Recorder` (see the
   `golden_replay_*` tests in `sim-runner` for the pattern).
2. Save the encoded output to `harness/goldens/replays/<descriptive-name>.replay`.
3. `check-replays.sh` picks it up automatically (it globs the directory).
