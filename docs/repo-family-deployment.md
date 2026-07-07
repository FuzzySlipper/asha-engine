# ASHA repo family deployment map

This page is the cold-start map for agents or users setting up the ASHA repo
family from scratch. It describes current repository roles and local checkout
shape. Historical planning lives in Den under project `asha`; repo docs should
describe durable architecture, setup, and verification surfaces.

## Repositories

| Repo | Local path | Owns | Does not own |
|---|---|---|---|
| `FuzzySlipper/asha-engine` | `/home/dev/asha-engine` | Rust authority, generated contracts, public TypeScript package surfaces, runtime bridge, render projection, governance, CI fixtures, and public-surface manifests | Product/demo content or consumer-specific proof harness identity |
| `FuzzySlipper/asha-demo` | `/home/dev/asha-demo` | Human-facing playable/demo content built through public ASHA surfaces | Synthetic conformance identity, private engine imports, proof factories as the product path |
| `FuzzySlipper/asha-studio` | `/home/dev/asha-studio` | Studio/editor UI, command composition, authoring workflows, and visual/debug read models over public ASHA surfaces | Rust authority, raw runtime/native transports, private ASHA internals |
| `FuzzySlipper/asha-testing` | `/home/dev/asha-testing` | Boundary proofs, conformance harnesses, negative smokes, reference consumer evidence, and temporary-adapter quarantine | Product/demo identity or engine feature implementation |

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
./harness/ci/check-all.sh
```

Use focused gates first when the edited surface is narrow. Run `check-all.sh`
for broad architecture, generated contract, bridge, replay, or cross-language
work.

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

`pnpm run test` is a broader Studio suite. Run it after the required
`asha-testing` publish/workspace cockpit evidence artifacts exist; it is not the
fresh-checkout deployment gate.

`asha-testing`:

```sh
npm install
npm run ci
```

`npm run ci` is the focused boundary gate for fresh checkouts. Broader
conformance, native backend, publish, and aggregate evidence commands require
current native runtime bridge exports plus generated sibling evidence artifacts.

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
