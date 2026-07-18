# Project content authoring

ASHA exposes one Rust-owned workspace-authoring boundary for durable project
content that is not a `SceneDocument` or environment materialization artifact.
The boundary is intentionally a closed document union, not a property-path API
or JSON value bus.

## Stored document kinds

`ProjectContentDocument` admits these reusable categories:

- `entityDefinition` — the existing typed entity definition and capabilities;
- `assetCatalog` — stored asset, material, version, and dependency records;
- `prefabRegistry` — prefab definitions, stable named variants, roles, and
  typed overrides;
- `gameplayConfiguration` — provider-selected typed configuration values,
  bindings, stable scene-instance overrides, and trigger definitions; and
- `presentationCatalog` — renderer-neutral resources and animation, audio,
  particle, or overlay cue records.

Scene nodes remain in the existing scene-document codec. A workspace retains
only scenes accepted by that codec, and project-content validation resolves
prefab placements, trigger volumes, and per-instance overrides against that
Engine-owned scene set. Project-content requests carry no scene index.

## Provider-owned configuration

Gameplay providers export immutable `ProjectConfigurationSchema` descriptors
from their statically linked Rust composition. Every codec and authoring result
returns the complete read-only `providerSchemas` catalog, including module,
provider, contract, codec, typed field, reference-picker, and numeric-bound
descriptors. This includes an empty document set, so Studio can create the
first provider configuration without inventing schema data. Per-document field
metadata links existing values back to their configuration and schema. Neither
decode nor authoring requests can supply or edit this catalog. Stored gameplay
content contains only the selected schema id and typed field values.

Rust invokes the selected provider's registered typed codec, verifies module,
provider, state/read/output contracts and configuration ownership, and resolves
typed references against the project content set. Provider and
product-specific combat or weapon vocabulary does not enter the generic
document contract.

## Public workflow

The workspace-authoring facade exposes:

1. `decodeProjectContent`, which strictly decodes source text, rejects unknown
   fields, resolves cross-document references, and returns canonical files,
   content identities, and field metadata;
2. `encodeProjectContent`, which performs the same validation over typed
   documents before canonical encoding; and
3. `applyProjectContentAuthoring`, which applies one typed upsert or delete.

The first accepted decode installs an opaque validated document-set artifact in
the Rust workspace cell. An authoring request is bound to the workspace id,
workspace generation, working revision, and that Engine-owned set hash; it no
longer resubmits current documents. A stale request cannot
invoke the edit or create a save candidate. An accepted edit increments the
Rust workspace revision and registers its returned set hash as the only hash a
trusted host may confirm as stored.

File selection and persistence are trusted-host responsibilities. The host
writes the returned `canonicalFiles` only after Rust acceptance, then calls the
ordinary workspace stored-confirmation operation with the accepted set hash.
Browser code never accepts an edit or promotes a file itself.

Downstream native addons install separate runtime and project-authoring bridge
constructors. `StaticProjectAuthoringBuilder` consumes the static gameplay
composition but retains only immutable registry/schema/codec authority. It does
not load a ProjectBundle or activate a gameplay host. Consequently an invalid
project can open for diagnostics before any `RuntimeSession` exists, and
runtime operations such as `readComposedRuntimeSession` are unavailable on its
authoring handle.

## Non-claims

This surface does not start a `RuntimeSession`, register runtime callbacks,
expose arbitrary mutation paths, or make presentation code authoritative. It
also does not materialize procedural environments; that remains a separate
recipe-to-artifact workflow. Runtime composition consumes these canonical
documents later through ProjectBundle loading.
