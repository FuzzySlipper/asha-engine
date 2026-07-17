---
status: current
audience: agent
tags: [ci, guardrails, testing, governance]
supersedes: []
see-also: [agent-code-atlas.md, design.md]
---

# Guardrail Policy

ASHA CI distinguishes consequential blockers from structural pressure. The machine-readable registry is `harness/ci/guardrail-policy.json`.

## Operating Posture

- Authority/import boundaries, generated borders, strict public wire decoding, replay/atomicity, accepted-state/data-loss behavior, and ambiguous selector fallbacks fail closed.
- Source-shape limits, vocabulary drift, code maps, and generated navigation counts warn with an owner and next action.
- Broad validator self-tests run when harness policy changes and at scheduled or campaign closure.
- Native/provider integration runs when native paths change and at scheduled or campaign closure.
- Visible Demo and Studio acceptance belongs to the consumer repository.

## Gate Selection

`./harness/ci/check-fast.sh` is the normal iteration command. Unknown and CI entrypoint changes expand safely to the full retained inventory.

## New Blocking Gates

Require a named consequential failure class, owner, bounded trigger and fallback, representative regression, rough cost, and a condition for narrowing or removal.

See `topics/ci/guardrail-policy.md` for the full policy and two-visible-slice calibration.
