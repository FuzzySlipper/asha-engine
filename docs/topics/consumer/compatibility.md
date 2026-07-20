---
status: current
audience: consumer
tags: [consumer, compatibility, public-surface, packages]
supersedes: []
see-also: [runtime-session-facade.md, consumer-compatibility.md, repo-family-deployment.md]
---

# Consumer Compatibility

ASHA remains in-house engine substrate work, but downstream consumers need durable answers about which surfaces they can use.

## Public Surface Manifest

`harness/public-surface/ts-packages.json` and `rust-crates.json` record every package as `public`, `unstable`, or `internal`. Consumer repos validate allowlists against this manifest.

## Package Statuses

| Surface | Status | Role |
|---|---|---|
| `@asha/contracts` | `public` | Generated semantic DTO/type border |
| `@asha/runtime-bridge` | `public` | Transport-neutral runtime facade |
| `@asha/runtime-session` | `unstable` | Transport-neutral RuntimeSession semantic readouts |
| `@asha/browser-host` | `unstable` | Browser/dev static UI host |
| `@asha/catalog-core` | `unstable` | Typed gameplay preset/catalog validation |
| `@asha/command-registry` | `unstable` | Studio command/evidence metadata |
| `@asha/devtools` | `unstable` | Observational attach/readout protocol |
| `@asha/game-workspace` | `unstable` | Typed game/workspace manifest validation |
| `@asha/render-projection` | `unstable` | Renderer-neutral retained render-diff application |
| `@asha/renderer-host` | `unstable` | Backend-neutral browser render and projection-only inspection host |
| `@asha/ui-dom` | `unstable` | Render-agnostic UI projection/control descriptors |

## Fail-Closed Policy

Unavailable native/reference backend support reports `RuntimeBridgeError` with `operation_unimplemented`. Unsupported source assets, invalid material maps, oversized output, stale hashes, and replay mismatches are typed diagnostics, not best-effort partial output.

See `topics/consumer/consumer-compatibility.md` for the full changelog and migration notes.
The `downstream-visual-authoring` role is the narrow renderer-enabled authoring
role; see `topics/projection/renderer-inspection-surface.md` for its package
boundary and viewer lifecycle.
