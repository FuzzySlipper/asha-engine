---
status: current
audience: agent
tags: [governance, source-shape, ci]
supersedes: []
see-also: []
---

# Source-Shape And Structural-Pressure Governance

ASHA source-shape limits keep ownership cells inspectable and prevent bridge, protocol, authority, or shell files from becoming default stuffing points. These limits are warning rails for assignment and review, not an architecture score and not a reason to split cohesive authority blindly.

## Shrink-only defaults

- A file without an exemption must stay within the global source-line limit.
- `warningSourceLines` reports pressure before the global limit is exhausted.
- Every exemption has its own lower `warningLines` threshold and shrink-only `maxLines` baseline.
- Shrinking a file or baseline needs no audit ceremony.
- Raising a global cap is forbidden. Adding an exemption or raising `maxLines` requires a reviewed change record.
- A line-count warning prompts review; it does not authorize an automatic refactor.

## Required exemption metadata

Rust `fileLineExemptions` and TypeScript `fileLineExemptions` / `rootBarrelExemptions` use the same metadata:

```json
{
  "maxLines": 1622,
  "warningLines": 1580,
  "owner": "rust-rule",
  "rationale": "The transactional coordinator keeps related authority adjacent while focused helpers are extracted.",
  "introducedBy": "#5761",
  "reviewBy": "2026-10-15",
  "reviewTrigger": "Review on a new reaction phase, ownership change, or attempted cap increase.",
  "removalCondition": "Remove after helpers are extracted and the authority file fits the global cap."
}
```

`reviewBy` is an expiry: the current-source gate fails after that date until the exception is renewed or removed. `reviewTrigger` captures events that require earlier review. `removalCondition` describes the structural outcome rather than pretending every cohesive large file should be split immediately.

## Audited baseline changes

Every new exemption and every increase to an existing baseline adds or refreshes `baselineChange`:

```json
{
  "baselineChange": {
    "changedAt": "2026-07-13",
    "changeTask": "#5761",
    "reason": "The exact pre-gate size is recorded with no growth headroom while a focused split proceeds.",
    "previousMaxLines": null,
    "newMaxLines": 1622
  }
}
```

For a new exemption, `previousMaxLines` is `null`. A later raise records the exact prior baseline. Git history retains older change records; vague or stale records fail the policy-diff gate.

## Assignment pressure and committed paths

The generated Agent Code Atlas reports actual edges beside allowed edges, fan-in/fan-out, actual or mutually allowed cycle risk, internal and public-role consumers, and near-cap source hotspots. It is generated evidence for assignment and review; the dependency and source-shape gates remain authoritative.

Committed paths are classified as source, generated source, other committed material, or build/cache/output. Tracked `dist`, `target`, `node_modules`, `__pycache__`, smoke output, and equivalent build paths fail the depgraph gate so generated artifacts cannot inflate inventories or masquerade as assignment surfaces.

## Revision selection

Rust and TypeScript policy-diff gates compare against:

1. `ASHA_SOURCE_SHAPE_BASE_REF` when CI supplies a push or pull-request base SHA;
2. `HEAD` when the policy has uncommitted changes; or
3. `HEAD^` for a clean committed change.

Run the normal structural gates with:

```bash
./harness/ci/check-depgraph.sh
./harness/ci/check-ts.sh
```
