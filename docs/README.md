---
status: current
audience: all
tags: [navigation, index, documentation]
supersedes: []
see-also: [design.md, agent-code-atlas.md]
---

# ASHA Docs

> New here? Read `design.md` first for architecture principles, then `agent-code-atlas.md` for lane routing.

## By Audience

- **Agents implementing in this repo**: `design.md` → `agent-code-atlas.md` → `topics/authority/`
- **Downstream consumers**: `topics/consumer/` → `topics/bridge/` → `topics/contracts/`
- **Reviewers**: `agent-code-atlas.md` → `topics/ci/` → `governance/`

## By Topic

| Topic | Contents |
|---|---|
| **Authority & gameplay** | `topics/authority/` — ECRP, voxel, combat, camera, input, time, replay, animation, nav |
| **Render & projection** | `topics/projection/` — feedback, HUD, materials, particles, billboards, audio, telemetry, devtools |
| **Contracts & codegen** | `topics/contracts/` — governance, vocabulary |
| **Bridge & composition** | `topics/bridge/` — boundary, static composition |
| **Consumer setup** | `topics/consumer/` — compatibility, deployment, game organization |
| **CI & testing** | `topics/ci/` — guardrails, feedback, perf |
| **Policy & expression** | `topics/expression/` — policy authoring, catalogs |

## By Status

- `status: current` — durable, trust it
- `status: draft` — WIP, verify against code
- `status: deprecated` — superseded, see `supersedes` link
- `status: historical` — ignore unless researching (lives in Den legacy, not this repo)

## "I need to..."

| I need to... | Start here |
|---|---|
| Add a gameplay event | `gameplay-fabric-growth-recipes.md` → `topics/authority/gameplay-fabric.md` |
| Add a RuntimeSession method | `runtime-session-facade.md` → `topics/bridge/` |
| Change a contract | `topics/contracts/governance.md` |
| Add a render projection | `topics/projection/overview.md` |
| Set up a downstream repo | `topics/consumer/deployment.md` |
| Understand voxel authority | `topics/authority/voxel.md` |
| Debug replay divergence | `topics/authority/replay.md` |
| Add CI gate | `topics/ci/guardrails.md` |
| Author a gameplay module | `gameplay-module-sdk.md` → `gameplay-module-conformance.md` |
| Place a prefab | `prefab-authoring-and-placement.md` → `prefab-instantiation.md` |
| Edit voxels | `workspace-authoring-facade.md` → `topics/authority/voxel.md` |
| Use the public Rust SDK | `gameplay-module-sdk.md` → `runtime-session-static-composition.md` |

## Archive

Historical task-specific audits, gap analyses, and round reviews live in Den under `asha/legacy/*`. They are not in this repo.
