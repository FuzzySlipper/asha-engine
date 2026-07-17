---
status: current
audience: agent
tags: [ci, feedback, timing, governance]
supersedes: []
see-also: [guardrails.md, ci-feedback.md]
---

# CI Feedback

CI feedback and timing context for ASHA's guardrail policy.

## Timing Context

The last pre-retirement warm full run selected 16 gates in 99.618 seconds. The first post-retirement run selected 13 retained gates in 118.955 seconds, including a 20.36-second native release rebuild. Ordinary narrow edits use affected-surface selection; the complete run remains below the three-minute warm budget.

## Policy-Change Validation

The policy-change validation run safely expanded the fast selector to the policy validator plus all 13 retained gates. It completed in 93.064 seconds with zero repeats, zero advisory failures, zero blocking failures, and zero duplicate execution fingerprints.

See `topics/ci/ci-feedback.md` for gate-level and cold-run detail.
