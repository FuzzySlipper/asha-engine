---
status: current
audience: consumer
tags: [template, agents, consumer]
supersedes: []
see-also: []
---

# ASHA Project AGENTS.md Template

Use this document as the starting point for `AGENTS.md` in a new ASHA-adjacent project repository. It is intentionally free of project-management-system assumptions so it can be used inside repos with plain GitHub issues, local task files, or other project trackers.

Copy the template section below into the downstream repo, then replace bracketed placeholders such as `[repo-name]` and `[project-specific command]` with local facts.

---

# AGENTS.md

## Project Role

`[repo-name]` is an ASHA project repository. It consumes ASHA through public package roots, generated contracts, and documented runtime/session surfaces. It does **not** own ASHA engine authority, generated contracts, runtime bridge internals, native transports, or Rust authority crates.

ASHA upstream engine repository:

```text
FuzzySlipper/asha-engine
```

Preferred sibling checkout shape for local development:

```text
/home/dev/
  asha-engine/      # upstream engine: Rust authority, contracts, public packages, CI/gates
  [repo-name]/      # this project: game/demo/studio/proof content over public ASHA surfaces
```

If your checkout paths differ, keep the same ownership rule: this repository may depend on ASHA public surfaces, but should not silently edit an unrelated local `asha-engine` checkout as part of normal project work.

## ASHA Architecture Contract

> Rust owns authority. TypeScript owns expression and projection. Generated contracts define the border.

- **Rust authority lives upstream in `asha-engine`**: canonical state, validation, event application, deterministic services, replay, serialization, simulation, and render projection generation.
- **TypeScript proposes and projects**: constrained policy/catalog code proposes typed commands; shell/render/UI/devtools display readouts and projected state.
- TypeScript does **not** mutate authoritative state. Rust validates commands before applying them.
- Public ASHA boundaries are generated or explicitly exported. Hand-editing generated ASHA files is forbidden.
- Missing capability in an ASHA public surface is an upstream engine gap, not permission for this repo to import internals or recreate authority locally.

## Source of Truth

Use this priority order when facts conflict:

1. This repo's current code, tests, and generated artifacts.
2. This repo's current README/docs for local setup and project-specific workflow.
3. `asha-engine` public docs and package metadata for ASHA architecture and public surfaces.
4. Historical notes, old planning docs, stale TODOs, and local scratch files.

Do not infer active work from historical roadmap prose. Verify against current code and tests.

## Upstream ASHA Usage Rules

### Allowed ASHA imports

Prefer package roots and documented subpaths only, for example:

```ts
import type { ProjectBundle } from '@asha/contracts';
import { createRuntimeSessionFacade } from '@asha/runtime-bridge';
import type { RuntimeActionIntentEnvelope } from '@asha/runtime-session';
```

Allowed surfaces depend on the project role, but common public/approved roots include:

- `@asha/contracts` — generated DTO/type border from Rust protocol crates.
- `@asha/runtime-bridge` — public runtime bridge/facade surface and typed errors.
- `asha-gameplay-module-sdk` — public local-path Rust facade for compiled,
  statically composed gameplay modules; use the engine public-surface manifest
  for the approved path and consumer roles.
- `@asha/runtime-session` — transport-neutral RuntimeSession semantic readouts/proposal vocabulary.
- `@asha/command-registry` — Studio/tool command metadata, not authority execution.
- `@asha/devtools` — observational diagnostics/readout models.
- `@asha/game-workspace` — typed game/workspace manifest validation.
- `@asha/render-projection` — renderer-neutral retained render-diff projection model.
- `@asha/renderer-host` — backend-neutral browser render surface host.
- `@asha/ui-dom` — render-agnostic UI projection/control descriptors.

### Forbidden ASHA shortcuts

Do not import or depend on:

- ASHA package `src/*` paths.
- ASHA package `dist/generated/*` paths.
- Rust crate internals from `asha-engine/engine-rs`.
- `@asha/native-bridge` or raw native transport packages unless this repo is explicitly an engine/native integration repo.
- `@asha/wasm-replay-bridge` as a product runtime transport.
- Private generated files, copied DTO forks, or arbitrary JSON command tunnels.
- Renderer buffers, DOM state, or local UI state as substitutes for runtime authority.

If a public API is missing, create an upstream request/issue/task for `asha-engine`, block the downstream feature, and keep the downstream workaround out of product paths.

## Do Not Modify Local `asha-engine` From This Repo

A sibling `/home/dev/asha-engine` checkout is convenient for local package linking, inspection, and running upstream checks. Treat it as upstream source, not as hidden workspace scratch.

From this project repository:

- Do not directly edit `../asha-engine` to make this repo pass.
- Do not commit unreviewed local engine changes as part of a downstream repo change.
- Do not patch generated ASHA contracts by hand.
- Do not vendor engine internals into this repo.
- Do not bypass a missing public surface with raw native/WASM calls or deep imports.

When the downstream project needs an ASHA engine change:

1. Document the missing public surface or bug with a minimal reproduction from this repo.
2. Open an upstream `asha-engine` issue/task/PR, or hand off to the engine owner.
3. Mark this repo's dependent work as blocked until the upstream public surface lands.
4. After upstream lands, update this repo through package-root imports and normal dependency/link refresh.

## Local Commands

Replace these with the actual commands for this repo. Keep commands small, repeatable, and CI-aligned.

```bash
# Install dependencies
[project-specific install command]

# Fast local verification
[project-specific fast check]

# Full project gate
[project-specific ci command]

# Build or package
[project-specific build command]
```

If this repo links to local ASHA packages, also know the relevant upstream gates in `asha-engine`:

```bash
# From /home/dev/asha-engine
./harness/ci/check-all.sh
./harness/ci/check-rust.sh
./harness/ci/check-ts.sh
./harness/ci/check-depgraph.sh
./harness/ci/check-contracts.sh
./harness/ci/check-bridge.sh
./harness/ci/check-gameplay-runtime-host.sh
./harness/ci/check-vocabulary.sh
```

Use focused upstream gates when inspecting an upstream change, but do not run them as a substitute for this repo's own checks.

## CI and Evidence Posture

Every meaningful change should leave evidence that another agent or reviewer can inspect.

- Run the narrowest relevant check first, then the repo's full gate before declaring done.
- Keep generated proof/evidence artifacts reproducible and documented.
- Prefer one command that rebuilds and validates proof artifacts over manual screenshots or cached outputs.
- If a workflow intentionally fails closed, test the failure mode and document the diagnostic.
- Record exact commands run and their result in handoff/review notes.
- Do not claim native runtime, GPU, browser, deployment, or performance proof unless the corresponding command actually exercised that surface.

## Coding Style

### General

- Prefer boring, explicit code over clever abstractions.
- Make boundaries visible in imports, function names, and tests.
- Keep mutation local and obvious.
- Avoid hidden global registries, ambient state, untyped/ambient event buses,
  and framework magic. Use the statically composed gameplay fabric for open
  typed game meanings.
- Favor small typed data shapes and deterministic functions.
- Add tests for behavior and for fail-closed boundary cases.

### TypeScript

TypeScript in ASHA projects is written for agent governance and mechanical review, not maximum terseness.

- Use named intermediate values for meaningful decisions.
- Split work into small functions with explicit verbs.
- Keep public API changes explicit and documented.
- Avoid `any`, unsafe casts, arbitrary JSON hatches, browser globals in policy code, and dynamic method-name dispatch.
- Keep policy/catalog packages separate from renderer/UI packages.
- Do not mutate authority from TypeScript; propose typed commands and display typed readouts.

### Rust

Rust authority code should live upstream in `asha-engine` unless this repository is explicitly an engine fork.

When working in Rust authority lanes:

- Prefer explicit state, explicit errors, explicit events, and narrow crate APIs.
- Avoid `unsafe`, `Rc<RefCell>`, unexplained clones, framework-ECS adoption, and
  untyped/ambient event buses unless explicitly approved by the engine owner.
- Keep crate dependencies aligned with the assignment-cell dependency direction.
- Add golden fixtures/replay evidence for state, protocol, and replay changes.

## Generated Contracts and Fixtures

Generated contracts define the Rust/TypeScript border.

- Rust protocol crates define canonical schemas upstream.
- TypeScript contracts are generated artifacts, committed for convenience, and never hand-edited.
- Contract changes require regenerated TypeScript, fixture updates, compatibility notes, and downstream checks.
- Golden fixtures should be named, inspectable, deterministic, and tied to the command/test that regenerates them.

## Dependency Direction

Respect ASHA's directional architecture:

```text
Rust: foundation -> state -> protocol -> services/rules/sim -> render/bridge/wasm/tools
TS: contracts -> script-sdk -> policy/catalog -> runtime/session/devtools -> shell/render/UI
```

This repo should sit downstream of ASHA public surfaces. It should not invert the dependency by making ASHA engine code depend on project content.

## Review Checklist

Before handing off a change:

- [ ] Imports use approved package roots and documented subpaths only.
- [ ] No ASHA internals, generated-path imports, raw transports, or JSON command tunnels were added.
- [ ] Any missing upstream surface was recorded as an upstream gap rather than patched locally.
- [ ] Project checks ran with real output.
- [ ] Generated artifacts, if any, were regenerated by documented commands.
- [ ] Docs/readmes changed if setup, commands, public surfaces, or limitations changed.
- [ ] Handoff includes exact commands run, relevant artifact paths, known non-claims, and remaining blockers.

## Useful Upstream Docs

Consult the current `asha-engine` docs for architecture and public-surface details:

- `README.md`
- `docs/design.md`
- `docs/architecture-overview.md`
- `docs/consumer-compatibility.md`
- `docs/runtime-bridge-boundary.md`
- `docs/runtime-session-facade.md`
- `docs/contract-governance.md`
- `docs/game-agent-code-organization.md`
- `docs/repo-family-deployment.md`
- `docs/github-check-gates.md`

Keep this repo's `AGENTS.md` shorter than the engine docs. It should tell agents how to behave here, not copy the whole upstream architecture manual.
