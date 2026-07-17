---
status: current
audience: all
tags: [navigation, index, documentation]
supersedes: []
see-also: [design.md, agent-code-atlas.md]
---

# ASHA Docs

> New here? Read `design.md` first for architecture principles, then `agent-code-atlas.md` for lane routing.
> All technical docs live in `topics/`. The root has only global architecture and navigation.

## Root Docs

| File | Audience | Purpose |
|---|---|---|
| `design.md` | all | Canonical architecture principles, layer model, language strategy |
| `agent-code-atlas.md` | agent | Lane routing, code maps, governance links |
| `agents-template.md` | consumer | AGENTS.md template for downstream repos |
| `README.md` | all | This index |

## Topic Directories

| Directory | Contents |
|---|---|
| `topics/authority/` | Rust authority: ECRP, voxel, gameplay fabric, combat, camera, input, time, replay, prefabs, workspace authoring, animation, nav |
| `topics/projection/` | Render/projection: render protocol, lighting, feedback, HUD, materials, particles, billboards, audio, telemetry, devtools |
| `topics/bridge/` | Runtime bridge: boundary, static composition, native browser host |
| `topics/consumer/` | Consumer setup: compatibility, deployment, game code organization |
| `topics/contracts/` | Contract governance and vocabulary |
| `topics/ci/` | CI guardrails, feedback, perf baseline |
| `topics/expression/` | Policy authoring and catalog expression |

## By Status

- `status: current` — durable, trust it
- `status: draft` — WIP, verify against code
- `status: deprecated` — superseded, see `supersedes` link
- `status: historical` — ignore unless researching (lives in Den legacy, not this repo)

## "I need to..."

| I need to... | Start here |
|---|---|
| Add a gameplay event | `topics/authority/gameplay-fabric-growth-recipes.md` |
| Add a RuntimeSession method | `topics/authority/runtime-session-facade.md` |
| Change a contract | `topics/contracts/contract-governance.md` |
| Add a render projection | `topics/projection/overview.md` |
| Set up a downstream repo | `topics/consumer/repo-family-deployment.md` |
| Understand voxel authority | `topics/authority/voxel.md` |
| Debug replay divergence | `topics/authority/replay.md` |
| Add CI gate | `topics/ci/guardrail-policy.md` |
| Author a gameplay module | `topics/authority/gameplay-module-sdk.md` |
| Place a prefab | `topics/authority/prefab-authoring-and-placement.md` |
| Edit voxels | `topics/authority/workspace-authoring-facade.md` |

## Archive

Historical task-specific audits, gap analyses, and round reviews live in Den under `asha/legacy/*`. They are not in this repo.
