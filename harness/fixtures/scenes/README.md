# Scene fixtures

Committed authored-scene documents (`FlatSceneDocument` JSON) used by `core-scene`
golden/readback tests and bootstrap examples. Nouns are abstract — no
product-domain content.

| Fixture | Source / producer | Consumer test |
|---|---|---|
| `sample-flat.json` | `core-scene` example `dump_canonical_scene` | `core-scene` `tests/golden.rs` (encode/decode/readback) |
| `bootstrap-summary.json` | `core-scene` example `dump_bootstrap_summary` (bootstraps `sample-flat` into world 7) | `core-scene` `tests/bootstrap.rs` summary golden |
| `invalid-cycle.json` | hand-authored invalid scene (parent cycle) | scene validation negative tests |

## Regenerate

```bash
# canonical scene document
cargo run -p core-scene --example dump_canonical_scene > \
  harness/fixtures/scenes/sample-flat.json

# bootstrap summary (deterministic world hash + source trace)
cargo run -p core-scene --example dump_bootstrap_summary > \
  harness/fixtures/scenes/bootstrap-summary.json
```

Regenerate only when a serialization/bootstrap change is intended, and review the
diff: a world-hash change is a deliberate determinism event (see the
rust-determinism / replay reviewers).
