---
status: current
audience: agent
tags: [projection, telemetry, diagnostics]
supersedes: []
see-also: []
---

# Live telemetry snapshot and overlay

Status: **Implemented baseline**  
Task: #5606  
Parent campaign: #5599

## Purpose

ASHA exposes one low-frequency, machine-readable view of useful runtime and
projection counters and can render that same view through a disposable
developer overlay. The snapshot exists for headless consumers, tests, tools,
and browser hosts; the overlay is only one presentation of it.

This is observability in service of building expressive gameplay. It is not a
second authority store, a replay input, or a requirement that a product ship a
debug UI.

## Public contract

`protocol-telemetry` owns the generated `LiveTelemetrySnapshot` contract:

- `authorityTick` identifies the latest authority point observed by the host;
- `sampleSequence` identifies the low-frequency observation sample;
- `metrics` contains only values an owner adapter actually supplied;
- `frameTimeHistoryMs` is bounded history for live trend readout;
- `diagnostics` names invalid or unavailable counters.

`@asha/renderer-host` exports `AshaLiveTelemetryCollector`. A consumer supplies
explicit owner-derived samples and may read the latest snapshot without
mounting a renderer or overlay. The collector preserves stable counter order,
bounds history to 1-240 samples, rejects invalid authority ticks, and never
turns an unavailable value into a plausible zero.

The initial counter vocabulary is deliberately small:

- frame time;
- entity and active-capability counts;
- resident and dirty chunk counts;
- current render-diff and render-handle counts;
- draw-call count;
- active audio-source, billboard, and particle counts;
- dropped feedback count.

An adapter declares which counters it expects to provide. Expected counters
that cannot be obtained from the current public owner surface are omitted from
`metrics` and reported as `counterUnavailable`. This makes partial support
inspectable and prevents downstream projects from inventing private imports.

## Overlay projection

The visible overlay uses the accepted G1 presentation frame, not a telemetry
variant in scene `RenderDiff`. Rust projects a generated
`TelemetryOverlayProjectionOp` with a domain-branded handle and an atomic
create/update/destroy lifecycle. Its descriptor controls title, corner,
visibility, refresh interval (100-5000 ms), and bounded frame-time history
(1-240 samples).

`AshaTelemetryOverlayHost` realizes those operations against an injected,
renderer-neutral sink. At the requested low-frequency cadence it gives the
sink the exact `LiveTelemetrySnapshot` returned to machine consumers. A local
visibility toggle changes only the retained presentation descriptor; it does
not resample, mutate authority, or alter the snapshot.

Scene application and other presentation domains do not depend on the overlay
host. A missing overlay host produces a typed `unavailableHost` diagnostic
after scene application and leaves headless telemetry readable.

## Owner adapters and semantic alignment

The collector is intentionally a host-side aggregation seam because these
counters have different owners. A browser composition may combine:

- authority tick, entity count, and capability count from runtime readouts;
- render-diff and retained-handle counts from projection readouts;
- active audio, billboard, and particle counts from their domain hosts;
- frame time from the render loop;
- chunk and draw-call counters only when an approved public owner exposes them.

Names overlap the offline perf harness only where their concepts are useful to
compare, not where their aggregation is identical. `frameTimeMs` has the same
millisecond unit, while live `renderHandleCount` is a current gauge rather than
offline `peakHandles`, and live `renderDiffCount` is a current sample rather
than cumulative `renderOpsApplied`. The live contract must not relabel those
offline aggregates as equivalent values.

## Authority, replay, and input posture

- Telemetry snapshots and overlay operations are observational and excluded
  from replay truth.
- Renderer timing and host resource counts cannot accept gameplay mutation.
- A recorded authority run may regenerate telemetry for comparison, but a
  telemetry snapshot is never applied as canonical state.
- The baseline toggle is projection-local. Shared browser input routing remains
  owned by #5642; migrating a keyboard binding later must not give the overlay
  input or gameplay authority.

## Delivered proof

- Rust validates typed, bounded overlay lifecycle transitions atomically.
- Public host tests prove headless sampling, unavailable-counter diagnostics,
  bounded history, exact snapshot reuse, local toggle isolation, and missing
  host failure isolation.
- `asha-demo` consumes only public generated/renderer-host surfaces, samples
  real runtime and projection readouts, displays the overlay after a gameplay
  interaction, and preserves screenshot plus live browser assertions.

## Current limits

The baseline is a stats overlay, not a system profiler. It does not yet provide
per-system timing, memory accounting, asset reference counts, GPU timing, or a
shared F3 binding. Adding a counter requires a stable owner and unit semantics;
adding an overlay capability requires the normal closed G1 contract, decoder,
host/provider regressions, and downstream visible acceptance.
