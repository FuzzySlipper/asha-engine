# ASHA Agent Code Atlas

Status: V1 navigation and governance layer for agents. Current task planning
still lives in Den; this atlas describes committed repo surfaces and routes
agents to the right owner lane, public boundary, and proof gate.

## How To Use This Atlas

1. Read live Den guidance first for current work state.
2. Use this atlas to choose the owning lane before opening files.
3. Follow links to code, docs, manifests, and proof gates.
4. Treat [generated inventory](code-map/generated-inventory.md) as volatile
   repo inventory, not as architecture prose.
5. If this atlas disagrees with code/tests, fix the atlas or create a task; do
   not force code to match stale prose.

## Maps

- [Rust authority and RuntimeSession](code-map/rust-authority-runtime-session.md)
- [Protocol and generated contracts](code-map/protocol-generated-contracts.md)
- [Runtime bridge and native/WASM facades](code-map/runtime-bridge-facades.md)
- [ECRP content and capability authority](code-map/ecrp-content-capability.md)
- [Render projection and renderer host](code-map/render-projection-renderer-host.md)
- [Downstream repo roles](code-map/downstream-repos.md)
- [Testing, conformance, fixtures, and goldens](code-map/testing-conformance-goldens.md)
- [Generated inventory](code-map/generated-inventory.md)

## Validation

Run the atlas validator from the repo root:

```bash
python3 harness/code-map/check-agent-code-atlas.py --check
```

Regenerate the inventory after crate/package/public-surface changes:

```bash
python3 harness/code-map/check-agent-code-atlas.py --write
```

The validator is also part of:

```bash
./harness/ci/check-depgraph.sh
```

It checks Markdown path links, required code-map sections, and whether
`docs/code-map/generated-inventory.md` matches current ownership, actual and
allowed dependency edges, fan-in/fan-out, public consumers, source hotspots,
committed-path classifications, public surfaces, bridge operations, fixtures,
and goldens.

## Stable Sources

- [README.md](../README.md)
- [AGENTS.md](../AGENTS.md)
- [design.md](design.md)
- [ownership.toml](../governance/ownership.toml)
- [public-surface manifest](../harness/public-surface/ts-packages.json)
- [runtime bridge manifest](../engine-rs/crates/bridge/runtime-bridge-api/bridge-manifest.toml)

## Non-Claims

- This is not a Den planning mirror.
- This is not a generated AI code summary.
- This is not a replacement for source code, tests, manifests, or ownership
  gates.
- This does not make private paths public. If a consumer needs a missing
  surface, add the upstream public/facade surface first.
