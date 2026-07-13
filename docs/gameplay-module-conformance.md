# Gameplay-module conformance

The public conformance kit lets a downstream agent prove its statically linked
Rust gameplay module without requesting a new engine-only test harness. It is a
support surface for real gameplay proofs, not a replacement for a playable
consumer slice.

## Public shape

Downstream modules keep their ordinary dependency on
`asha-gameplay-module-sdk`. Their development/conformance target may also depend
on:

```toml
asha-gameplay-module-conformance = { path = "../asha-engine/public-rust/gameplay-module-conformance" }
```

The caller supplies:

- committed ProjectBundle-shaped JSON containing Session identity, selected
  consumer-need ids, and the generated `GameplayModuleBindingRegistry`;
- the real role-scoped consumer-needs manifest and compile-time reachable public
  surface markers for the linked SDK/conformance crates;
- a function that builds the real `GameplayStaticComposition`; and
- one or more typed root event envelopes.

The runner does not load code dynamically. The downstream provider, behavior,
codecs, state/view adapters, and configuration metadata are ordinary compiled
Rust values.

## What one run proves

`run_gameplay_module_conformance` performs the following sequence:

1. Parse the authored ProjectBundle-shaped input and execute a real ordered
   ProjectBundle scene/bootstrap plan.
2. Resolve every binding against the closed compiled provider registry and
   initialize all module state atomically.
3. Invoke real downstream behavior, capture frozen-view/delivery evidence, and
   apply accepted module-local facts through the registered typed state adapter.
4. Match every selected consumer need against actual closed-registry providers,
   event publications/subscriptions, invocation families, read views, fields,
   selectors, quotas, ordering, proposal owners, authored bindings, delivered
   configuration, reaction evidence, and compile-time reachable public surfaces.
5. Require every authored declared-read request to appear in canonical frozen
   read evidence for its exact module and invocation. A valid event that does
   not select the invocation cannot satisfy read delivery.
6. Save the gameplay Session snapshot and restore it against a freshly built
   composition and ProjectBundle authority.
7. Restore the initial snapshot and apply only recorded accepted facts, proving
   playback reconstructs the same final state without re-invoking behavior.
8. Execute the entire case again and compare reaction frames, event/view/
   invocation/fact evidence, diagnostics, state, snapshot, and final hashes.

The schema-versioned `GameplayModuleConformanceReport` is machine-readable JSON.
It includes the canonical consumer-needs manifest hash, registry digest and
topology dump, module/artifact identities, binding and activation hashes,
reaction frames, checks, stable gap codes, and a compact human trace.

## Fail-closed behavior

The committed external fixture proves that provider drift and malformed
configuration reject before any initial or final state hash exists. It also
proves a declared module that receives no real invocation cannot pass. The
broader gameplay-fabric suites retain stable-code negatives for missing codecs,
providers and owners, foreign namespaces, cycles, undeclared events/reads/
queries/proposals, stale revisions, and budget exhaustion.

Consumer-needs ids are not labels copied into the report. The supplied manifest
is decoded and each selected requirement gets a `consumerNeed.<id>` check plus a
typed gap such as `consumerNeedMissingEvent`, `consumerNeedMissingField`,
`consumerNeedMissingSelector`, `consumerNeedMissingProposal`,
`consumerNeedMissingBinding`, `consumerNeedUnreachableSurface`, or
`consumerNeedUndelivered`. Changing a field, selector, quota, provider, binding
shape, or delivery requirement therefore fails the real conformance run as well
as the repository inventory gate.

## Commands

For the committed downstream fixture:

```bash
cargo run --locked --offline \
  --manifest-path harness/fixtures/gameplay-module-sdk/downstream-module/Cargo.toml \
  --bin conformance -- --json /tmp/gameplay-module-conformance.json
```

For the full public kit, including negative fixtures:

```bash
./harness/ci/check-gameplay-module-conformance.sh
```

The existing `harness/conformance/probe-results.json` is the repository-wide
machine-readable inventory. Per-module runners can serialize their returned
`GameplayModuleConformanceReport` with `to_pretty_json()` into artifacts owned by
`asha-testing`, `asha-demo`, or another approved consumer.
