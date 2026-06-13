# Diagnostic goldens — intentionally broken fixtures

Source doc: `scene-capability-06-scene-asset-devtools-diagnostics` (epic #2313, subtask #2332).

Each `.txt` here is the deterministic diagnostic output of one **intentionally
broken** scenario, rendered by `scene-diagnostics` (`text::report_set_to_text` /
`traces_to_text` / `resource_report_to_text`). They are both regression goldens
and agent-training examples: every failure class maps to a **stable
`DiagnosticCode`**, a **source ref**, and a **suggested remedy**, so an agent can
route a failure back to the owning lane without parsing prose.

Diagnostics are observational: producing these never mutates authority.

Regenerate after an intended change to the emitters or their text form:

```bash
BLESS=1 cargo test -p scene-diagnostics --test goldens
```

The builders live in `engine-rs/crates/tools/scene-diagnostics/tests/goldens.rs`.
Fixture nouns are abstract (`mesh/belt-straight`, `sprite/hard-hat`) — no
product-domain content.

## Severity → recovery policy

`fatal` stops the load (`blocksLoad=true`); `error` degrades one
node/entity/asset; `warning`/`info` never block. The header line of each
report-set golden states `count`, `maxSeverity`, and `blocksLoad`.

## Fixtures

| Fixture | Broken thing | Code(s) | Severity | Owning lane / routing |
|---|---|---|---|---|
| `duplicate-scene-id.txt` | Two scene nodes share an id | `duplicateSceneId` | error | scene authoring (`core-scene`) |
| `missing-static-mesh.txt` | Scene node references a mesh absent from the catalog | `sceneAssetMissing` | error | scene authoring + asset catalog |
| `missing-sprite-texture.txt` | A material depends on a texture absent from the catalog | `missingAsset` | error | asset catalog (`core-catalog`) |
| `wrong-kind-asset-ref.txt` | A material's texture slot points at a non-texture asset | `wrongKindAssetRef` | error | asset catalog (`core-catalog`) |
| `asset-dependency-cycle.txt` | A cycle in the asset dependency DAG (path included) | `assetCycle` | error | asset catalog (`core-catalog`) |
| `corrupt-bundle-artifact.txt` | A durable/generated artifact's bytes no longer match its recorded hash | `corruptBundleArtifact` | **fatal** | world-bundle serialization (`svc-serialization`) |
| `unsupported-manifest-version.txt` | Manifest schema/protocol newer than this build (fail closed) | `manifestProtocolMismatch` | **fatal** | world-bundle serialization (`svc-serialization`) |
| `stale-cache-warning.txt` | An optional cache artifact is absent (reproducible) | `missingCacheWarning` | warning | world-bundle serialization (rebuild cache) |
| `missing-render-source-trace.txt` | Render handles that can't be traced to authority / drew a fallback | `missingSourceTrace`, `fallbackUsed` | warning | render projection (`renderer-three`) |
| `source-trace.txt` | Snapshot of a render handle → scene node → entity → asset trace batch (one healthy, two broken) | — | — | render projection (observational) |
| `renderer-resources.txt` | Snapshot of a leaking renderer resource report + its diagnostics | `rendererResourceSummary`, `suspectedResourceLeak`, `fallbackUsed` | info/warning | renderer resources (observational) |
| `round-trip-equivalence.txt` | A clean save→reload round-trip equivalence report (zero diagnostics here; an equivalence loss emits `roundTripMismatch`) | — / `roundTripMismatch` | error | world composition (`rule-world-bundle`) |
| `composition-failures.txt` | A spread of world load/save composition failures (missing artifact, too-new version, voxel replay conflict, final-consistency mismatch) | `loadStageFailed`, `manifestProtocolMismatch`, `finalConsistencyMismatch` | **fatal** | world composition (`scene-diagnostics::composition`, #2364) |
| `bundle-equivalence.txt` | Full load→edit→save→reload bundle round-trip proving B==C (scene/entity hash, source traces, voxel fingerprint); a lost facet emits `roundTripMismatch` | — / `roundTripMismatch` | error | world composition (`scene-diagnostics::equivalence`, #2362) |

## No Den coupling

These artifacts are generic ASHA diagnostics: stable codes, source refs, and
remedies. An external workflow system (e.g. Den) may consume them, but nothing
here imports or names Den, and nothing should.
