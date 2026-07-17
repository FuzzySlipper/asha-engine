---
status: current
audience: consumer
tags: [consumer, deployment, setup]
supersedes: []
see-also: []
---

# ASHA repo family deployment map

This page is the cold-start map for agents or users setting up the ASHA repo
family from scratch. It describes current repository roles and local checkout
shape. Historical planning lives in Den under project `asha`; repo docs should
describe durable architecture, setup, and verification surfaces.

## Repositories

| Repo | Local path | Owns | Does not own |
|---|---|---|---|
| `FuzzySlipper/asha-engine` | `/home/dev/asha-engine` | Rust authority, generated contracts, public TypeScript package surfaces, runtime bridge, render projection, governance, local guardrails, and provider regressions | Product/demo content or downstream acceptance |
| `FuzzySlipper/asha-demo` | `/home/dev/asha-demo` | Human-facing playable/demo content and visible acceptance built through public ASHA surfaces | Synthetic conformance identity or private engine imports |
| `FuzzySlipper/asha-studio` | `/home/dev/asha-studio` | Studio/editor UI, command composition, authoring workflows, and visual/debug read models over public ASHA surfaces | Rust authority, raw runtime/native transports, private ASHA internals |
| `FuzzySlipper/asha-testing` | `/home/dev/asha-testing` | Focused synthetic public-surface regressions and strict package/path boundary negatives | Product/demo identity, visible acceptance, or engine feature implementation |

The Den project id remains `asha`. The package scope remains `@asha/*`.

## Fresh checkout shape

Clone sibling repos so local package links and boundary checks resolve:

```sh
mkdir -p /home/dev
cd /home/dev
git clone git@github.com:FuzzySlipper/asha-engine.git asha-engine
git clone git@github.com:FuzzySlipper/asha-demo.git asha-demo
git clone git@github.com:FuzzySlipper/asha-studio.git asha-studio
git clone git@github.com:FuzzySlipper/asha-testing.git asha-testing
```

Use the current repo docs in each checkout for package-manager commands. Do not
infer active work from historical roadmap documents; resolve Den guidance and
tasks before substantial implementation work:

```text
get_agent_guidance(project_id="asha")
```

## Engine checks

Run from `/home/dev/asha-engine`:

```sh
./harness/ci/check-depgraph.sh
./harness/ci/check-contracts.sh
./harness/ci/check-ts.sh
./harness/ci/check-rust.sh
./harness/ci/check-fast.sh
./harness/ci/check-all.sh
```

Use `check-fast.sh` as the normal changed-surface loop. Run `check-all.sh` for
campaign/release closure and manually dispatched comprehensive verification.
The full inventory includes the native/browser-host claim; use
`check-native.sh` only for focused native iteration.

## Consumer checks

`asha-demo`:

```sh
npm install
npm run check:dependencies
npm test
npm run build
```

`asha-studio`:

```sh
pnpm install
pnpm run check:boundaries
pnpm run check:docs-scripts
pnpm run build
```

`pnpm run test` is the broader Studio-local product regression suite.

`asha-testing`:

```sh
npm install
npm run ci
```

`npm run ci` runs the focused boundary and synthetic public-contract suite.
`npm run synthetic:native` explicitly exercises the current native provider.

Consumer repos must use public ASHA package roots or published public artifacts.
If a consumer needs a missing capability, add or request a public engine surface
in `asha-engine`; do not import package `src/*` paths, generated files by path,
Rust crate internals, raw native/WASM transports, or arbitrary JSON command
hatches.

## Current engine docs to keep live

- `README.md` for repo layout, command index, and durable surface map.
- `docs/design.md` for canonical architecture.
- `docs/architecture-overview.md` for short orientation.
- `docs/consumer-compatibility.md` for public package surfaces and consumer
  role policy.
- `docs/runtime-bridge-boundary.md` and `docs/runtime-session-facade.md` for
  runtime facade behavior.
- `docs/contract-governance.md` for Rust protocol to TypeScript contract flow.
- `docs/game-agent-code-organization.md` for downstream game repo structure.
- `docs/github-check-gates.md` for Den GitHub check gate registration.

Historical architecture analysis documents may remain as dated audit artifacts,
but they are not setup instructions or the current work queue.
