---
status: current
audience: agent
tags: [ci, github, gates, checks]
supersedes: []
see-also: []
---

# GitHub Check Gates

ASHA exposes a fast changed-surface gate for ordinary iteration:

```bash
./harness/ci/check-fast.sh
```

The comprehensive engine gate remains:

```bash
./harness/ci/check-all.sh
```

The workflow is `.github/workflows/offline-ci.yml`. Pushes and pull requests run
`Fast changed-surface safeguards`. The scheduled job and a manual dispatch with
`tier=full` run `Full engine and native safeguards`.

`check-all.sh` owns the semantic gate inventory. In particular,
`check-gameplay-runtime-host.sh` is the single execution owner for direct host
tests and its targeted one-cell provider lifecycle proof. The earlier Rust
workspace pass excludes that crate only under `check-all.sh`; standalone
`check-rust.sh` remains complete. The runtime-host gate records bounded local
evidence under `harness/smoke-out/gameplay-runtime-host/`.

For Den Review GitHub check gates, use:

```json
{
  "project_id": "asha",
  "task_id": "<den-task-id>",
  "repository": "FuzzySlipper/asha-engine",
  "commit_sha": "<full-40-character-sha>",
  "ref": "main",
  "required_checks": ["Fast changed-surface safeguards"],
  "requested_by": "<agent-name>"
}
```

Agents should register the exact pushed commit SHA after a task commit is
pushed. The Den service records pass, fail, timeout, or superseded evidence on
the task thread; GitHub Actions remains the runner.

Use `Fast changed-surface safeguards` for ordinary exact-SHA task gates. Use
`Full engine and native safeguards` only when the task explicitly requires a
manually dispatched comprehensive/native run. A fast result is not evidence
that the full or product-consumer tier ran.
