# Session-state snapshot fixtures (Den task #2484)

Committed fixtures for the runtime **session-state snapshot** — the durable artifact
that persists runtime-diverged entity authority (runtime-created entities, diverged
transforms, capability tables, relations, source traces, asset references) across a
world-bundle save → reload.

These are generated from `core_entity::fixtures::mixed_world_save_fixture()` and
checked by `scene-diagnostics`'s `session_state_goldens` test. Regenerate after an
intended change:

```text
BLESS=1 cargo test -p scene-diagnostics --test session_state_goldens
```

| File | What it pins |
|------|--------------|
| `mixed-world.snapshot.json` | The canonical encoded `sessionStateSnapshot` artifact for the mixed-world save fixture. Contains every fixture vocabulary class in one save: a scene-sourced spatial rendered entity with a diverged transform, a runtime-created spatial non-rendered collider, a non-spatial logical entity, a containment relation, a transform attachment, an asset-bound import, a source-ancestry trace, and a tombstone. |
| `mixed-world-equivalence.txt` | The deterministic round-trip equivalence report: encode → decode → `from_snapshot` reproduces the exact `EntityStore` fingerprint, with no classified mismatch diagnostics. |

## Vocabulary classes covered

- **spatial rendered** (id 1) — transform + render projection, scene-sourced, transform diverged from authored origin.
- **spatial non-rendered collider** (id 2) — transform + bounds + collision, no render projection.
- **non-spatial logical** (id 3) — no transform; controller association only.
- **containment relation** (id 4 → 2) — membership, not transform attachment.
- **transform attachment + asset + source ancestry** (id 5 → parent 1, derived-from 4).
- **tombstone** (id 6) — destroyed entity retained for replay/dangling-reference diagnostics.

A combined "voxel edit plus entity change in the same save" round-trip is exercised
by `rule-world-bundle`'s `session_state_roundtrip` integration test, which loads a
bundle carrying both a voxel section and this session-state snapshot.
