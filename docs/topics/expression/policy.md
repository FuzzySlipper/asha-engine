---
status: current
audience: agent
tags: [policy, typescript, expression, catalog]
supersedes: []
see-also: [design.md, contract-governance.md]
---

# Policy and Catalog Expression

TypeScript policy and catalog packages are high-churn expression code. They receive read-only views and return proposed commands. They do not mutate authoritative state.

## Policy Packages

Decision rules, scenario orchestration, state-machine policies, procedural selection logic, testable heuristics, and abstract process control. May not contain authoritative mutation, renderer imports, DOM imports, direct WASM memory access, filesystem/network calls, wall-clock time, ambient random, hidden global registries, or shadow copies of state.

## Catalog Packages

Typed catalogs and data-like declarations. Rust validates catalog data before accepting it into the authoritative runtime.

## Script Host

Loads policy packages, invokes policy functions, provides deterministic inputs, collects proposed commands, sandboxes forbidden APIs, records script diagnostics, and runs fixture-based script tests. Does not validate commands.

See `docs/policy-authoring.md` for the full authoring guide.
