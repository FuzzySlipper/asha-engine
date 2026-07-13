# Public capability reachability gate

Reachability proves that public catalog entries join without a dead end. Real
execution is the next, separate gate; see [Real conformance probes](real-conformance-probes.md).

The reachability gate joins the existing protocol, bridge, public-surface,
gameplay-provider, binding, and consumer-needs catalogs. It does not authorize
runtime behavior and is not another capability registry. Its job is to fail CI
when a public promise no longer has a continuous route from contract to provider
to selector/field support to real delivery evidence.

The reviewed join is `harness/reachability/manifest.json`. Each public capability
records:

- the generated or Rust protocol symbol;
- the concrete provider evidence;
- every advertised field and selector with provider evidence;
- a stable bridge operation where a bridge operation is actually required;
- the public TypeScript package or Rust facade and exported symbol;
- the independently governed consumer-needs requirement for that capability, when one exists; and
- a real delivery/conformance proof.

Catalog assertions pin the complete current counts for generated contract
exports, bridge operations and stability classifications, public TS/Rust
surfaces, and consumer requirements. Their existing specialist gates remain the
deep validators for every catalog entry; the reachability report hash-binds those
catalogs and makes any addition/removal an explicit review event. The individual
capability joins then pressure-test the cross-catalog paths where simple catalog
validation is insufficient.

The validator owns the complete current capability-to-consumer mapping separately
from the authored manifest. Omitting a capability or relinking it to an unrelated
need therefore fails even if the manifest remains structurally valid. Prose and
README token matches are not accepted as provider or delivery evidence.

Generic gameplay events and declared reads deliberately do not name a dedicated
bridge operation. Their reachability ends at the public Rust gameplay-module SDK,
closed provider registry, declared read assembler, and execution evidence. This
preserves the fabric shape: adding a gameplay question does not require adding a
new facade verb.

Every public gameplay-read route must also carry an `exactlyOne` closed-registry
provider constraint and namespace-ownership evidence. Where a gameplay consumer
need participates in the compiled conformance case, reachability additionally
requires that exact semantic check to pass.

Authored gameplay bindings additionally require a bootstrap-adapter proof. The
gate therefore catches a generated binding/configuration schema that still exists
after its compiled provider or typed initialization adapter disappears.

## Internal-only surfaces

Construction and store-aware machinery can be intentionally internal, but every
exemption names its owner, a specific reason, and evidence at the public facade.
The current exemptions cover the store-aware read assembler and mutable registry
builder. Vague or ownerless exemptions fail validation.

## Durable report and failures

`harness/reachability/validation-report.json` includes hashes of the joined
catalogs, per-capability reachability, and deterministic gap codes. Downstream
task acceptance can consume this file without scraping CI text.

Negative fixtures prove failures for a changed public catalog count, absent
generated contract or provider, unsupported field/selector/quota, missing binding
bootstrap adapter, absent bridge operation, and unjustified internal exemption.
Adversarial checks also mutate the canonical manifest to prove that capability
omission, unrelated consumer linkage, prose-only evidence, and removal of read
provider cardinality fail with precise gap codes.

Run:

```bash
./harness/ci/check-reachability.sh
```

The check is part of `./harness/ci/check-all.sh`.
