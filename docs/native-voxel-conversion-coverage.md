# Native Voxel Conversion Coverage Matrix

Status: planning record for Den task #4554.

## Purpose

The current native voxel conversion lane proves a narrow authority route:

```text
generated voxel-conversion DTOs
  -> RuntimeSessionFacade / runtime bridge
  -> svc-voxel-conversion Rust authority
  -> generated voxel command apply
  -> typed receipts, diagnostics, evidence refs, and model-info readouts
```

That route is the durable path. Studio may present controls, diagnostics, and
evidence, but must not own mesh voxelization, raw native calls, renderer-buffer
authority, copied VoxelForge logic, or private generated-contract imports.

This document records the next production-grade proof matrix. It is not a claim
that ASHA already supports arbitrary conversion workloads.

## Current Baseline

Already covered in the engine lane:

- Static mesh sources are authority-visible data with positions, triangles,
  material slots, source refs, and source hashes.
- Plan/preview/apply are generated protocol DTOs and bounded runtime bridge
  operations.
- Source hash mismatch, unsupported sources, invalid material maps, stale
  plan/preview hashes, output limit overflow, and unregistered targets fail
  closed with typed diagnostics or bridge errors.
- Transform, voxel size, fit policy, origin policy, target origin, resolution,
  material map, and max output voxels are part of the Rust settings fingerprint.
- Surface conversion maps triangle vertices through Rust coordinate mapping.
- Material assignment is currently Rust-owned source-material-slot mapping with
  an optional Rust-owned default voxel material fallback.
- Solid conversion is limited to closed-manifold sources and currently fills the
  mapped bounds of accepted sources.
- RuntimeSession exposes conversion planning, preview, apply, evidence export,
  and model-info readout through public package roots.
- Texture/UV sampling is not implemented. Renderer buffers, Three.js state,
  Studio image memory, or copied VoxelForge texture paths are not conversion
  authority.

## Coverage Matrix

| Slice | Question | Owner repo | Engine work | Evidence surface | Studio posture |
|---|---|---|---|---|---|
| Larger deterministic meshes | Can the Rust service convert moderately large authored meshes without drift, accidental O(n^3) blowups, or output hash instability? | `asha-engine`, then `asha-testing` | Add generated fixture meshes, larger surface/solid cases, stable output summaries, and explicit runtime/resource budgets. | `asha-testing` public RuntimeSession/conformance runs pin receipts, hashes, diagnostics, and model-info readouts. | Display evidence only. |
| Ambiguous and non-manifold solids | Are holes, duplicate faces, bow-tie edges, flipped/ambiguous topology, and degenerate triangles rejected or classified before output is trusted? | `asha-engine`, then `asha-testing` | Expand topology validation beyond the current undirected-edge count gate and add classified diagnostic cases. | Negative fixtures prove `non_manifold_or_ambiguous_solid` or sharper future diagnostic codes. | Display rejection diagnostics; no local repair/voxelizer. |
| Material and texture posture | What material data is authoritative today, and how will texture/UV sampling become authority-owned later? | `asha-engine` | Keep current material-slot mapping explicit; design/add UV and texture sample DTOs only when Rust owns sampling. Do not let renderer/Studio texture buffers become authority. | Contract and service tests for material slot fallback now; later texture sampling fixtures through public DTOs after #4596. | Author/display source material bindings and diagnostics only. |
| Output limits and quotas | Do conversion requests fail predictably when output size, resolution, or memory/time budgets exceed sensible limits? | `asha-engine`, then `asha-testing` | Add request/result guardrails for max voxels, max source vertices/triangles, max resolution, and bounded work estimates. | Boundary tests and public conformance fixtures for accepted and rejected budgets. | Surface budgets and rejection reasons. |
| Performance/resource guardrails | Do accepted representative conversions stay inside documented local budgets, and do regressions look like failures rather than silent slowness? | `asha-engine`, `asha-testing` | Add deterministic timing/size summaries where stable enough; keep wall-clock evidence in testing/perf lanes, not semantic receipts. | `asha-testing` records perf/resource evidence with host labels; engine unit tests pin semantic bounds. | Optional readout only after testing evidence exists. |
| Model info/readback | Can consumers inspect authoritative converted model identity, bounds, counts, source refs, hashes, and diagnostics without private state access? | `asha-engine` | #4553 added the first bounded RuntimeSession readout. Future work may add chunk residency details once Rust owns that query. | Runtime bridge and consumer tests use `readVoxelModelInfo`. | Consume public readout only. |
| Public consumer proof | Can downstream repos use the lane through approved package roots only? | `asha-testing` | Engine supplies public surfaces and generated contracts. | Synthetic consumer proof imports package roots and runs conversion evidence without Studio internals. | Studio can later reuse the same public surfaces. |

## Follow-Up Decomposition

Created from this matrix:

- Den #4591: larger mesh conversion fixtures and semantic budget coverage.
- Den #4592: robust non-manifold/ambiguous solid diagnostics.
- Den #4593: material/texture sampling authority posture and DTO plan.
- Den #4594: output quota and resource guardrail enforcement.
- Den #4595: public consumer evidence matrix in `asha-testing`.
- Den #4596: Rust-owned texture and UV sampling design for voxel conversion.

## Material And Texture Authority

Current material authority is intentionally narrow:

- A static mesh source declares source material slots.
- A conversion request supplies a generated `VoxelConversionMaterialMap`.
- Rust validates that all source slots are mapped unless a default voxel
  material is provided.
- Rust assigns voxel material ids from the explicit slot map or default fallback.
- Receipts, previews, and model-info readouts expose resulting voxel/material
  counts as projection/evidence, not as downstream mutation authority.

Texture and UV sampling remain out of scope for the current implementation.
Future texture support must first define authority-visible texture/source refs,
UV data, sampling policy, hashes, and diagnostics in Rust/generated contracts
(tracked by #4596). Studio may author references or display diagnostics, but it
must not sample renderer textures locally and feed those samples back as if they
were Rust authority.

No Studio implementation task is created from this planning slice. Studio remains
a consumer/evidence surface until the engine and testing lanes expose the needed
public behavior.

## Acceptance Gates For Future Slices

Each implementation slice should provide:

- Rust authority tests at the service/bridge layer closest to the behavior.
- Generated protocol updates only through Rust protocol crates and codegen.
- Public RuntimeSession or package-root evidence when a downstream consumer is
  expected to use the behavior.
- Negative tests for unsupported or over-budget inputs.
- A clear non-claim when a production case remains intentionally unsupported.

If a proof requires Studio-owned conversion logic, private generated imports,
renderer buffer inspection as authority, or copied VoxelForge runtime behavior,
the proof is invalid for this lane.
