---
status: deprecated
audience: agent
tags: [wave1, compatibility, quarantine, migration]
supersedes: []
see-also: []
---

# Wave 1 Compatibility Quarantine

Wave 1 compatibility is a finite migration lane, not a second public design.
The machine-readable source of truth is
`harness/public-surface/wave1-compatibility.json`; this document explains how
to consume it.

Public Rust surfaces have one of two dispositions:

- **preferred** — suitable for new consumers within the role policy;
- **quarantined** — reachable only by the named existing consumers while a
  deletion task moves them to the preferred path.

`harness/public-surface/rust-crates.json` keeps those lists separate. A public
status means a path is governed and checked; it does not make a quarantined
path recommended. Consumer-needs and conformance reports carry the same
disposition so a passing probe cannot quietly turn migration evidence into a
new-consumer endorsement.

## Active inventory

| Concern | Existing consumers | Preferred replacement | Deletion owner |
|---|---|---|---|
| Legacy weapon-effect hook and bridge verb | `asha-demo` | Native `GameplayModuleBehavior` Transform inside `asha-runtime-session-composition` | #5734 |
| Standalone `asha-gameplay-runtime-host` product host | `asha-demo`, `asha-rulebench`, quarantine fixture `asha-testing` | `DeferredRuntimeSessionBuilder` from `asha-runtime-session-composition` | #5734 and Rulebench #5715 |

The inventory records the compatibility version, owning lane, exact code
boundary, real consumers, diagnostic, fail-closed evidence, and deletion
condition for each row. Adding an unlisted consumer or reclassifying a
quarantined Rust root as preferred fails the public-boundary gate.

The Demo-specific native provider kind/global alias is no longer active
upstream. #5732 removed it as part of #5734's migration to
`asha.runtime_bridge.native_rust_provider.v1` at `globalThis.ashaRuntimeBridge`.

## Structural rules

- New gameplay modules use the preferred SDK and one-cell RuntimeSession
  composition. They do not add another `GameRuleModule` hook or instantiate a
  second gameplay host.
- The retained weapon adapter is available only as
  `rule_gameplay_fabric::compatibility::run_legacy_weapon_effect_transform`.
  It is not root-re-exported. The duplicate SDK adapter was removed.
- The SDK temporarily retains an unused Cargo resolution edge to
  `game-rule-extension` so the active Demo/Rulebench `--locked` builds do not
  acquire a surprise lockfile edit during the upstream-only campaign. No SDK
  source imports or exports that crate; remove the edge with #5734/#5715 lock
  migration.
- Compatibility code validates through the same owner, codec, registry,
  continuation, and rollback gates as its preferred replacement. It may adapt
  shapes; it may not bypass authority.
- Removal happens only after the named downstream proof is live. The
  quarantine is allowed to be awkward and visible; it is not allowed to break
  an active consumer merely to make the inventory look clean.

## Known limitations

Until the deletion tasks close, the repository deliberately carries the
standalone Rust gameplay host quarantine. That is migration debt, not a claim
of interchangeable authority. The
standalone host cannot be used to justify a second browser endpoint, shadow
EntityStore, or TypeScript semantic-event ferry.

Run `python3 harness/public-surface/check-public-boundary.py` to validate the
quarantine inventory and role policy.
