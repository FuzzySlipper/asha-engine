# Integrated gameplay feedback projection

## Purpose

The G1 presentation channel is a game-facing composition path, not a collection
of unrelated renderer demonstrations. One accepted Rust owner fact can drive a
semantic animation transition and several disposable realizations without
creating a second authority event for each host.

The public `RuntimeSession` primary-fire proof emits this ordered frame after
the scene diff:

1. one spatial audio cue;
2. one particle burst;
3. player and target world-space billboards;
4. animation-controller create and fixed-tick updates;
5. telemetry-overlay create or update.

Every operation retains the same `PresentationOriginRef`: the accepted
primary-fire owner-fact ID, authority tick, causation ID, and session
correlation ID. Operation sequence identifies ordering within the frame. Host
diagnostics retain the operation sequence, handle when applicable, and origin,
so a downstream agent can trace a failed realization without treating it as an
authority failure.

## Authority and disposal boundary

Rust owns the accepted primary-fire outcome, controller state, fixed timing
facts, and replay. Audio playback, particle simulation, billboard DOM nodes,
animation mixer weights, and telemetry overlay DOM are projection-only and are
marked `excludedFromReplayTruth`.

A consumer may tear down all presentation hosts and rebuild them from the last
public projection frame. Rebuild must leave the RuntimeSession hash, gameplay
interaction readout, and projected controller state unchanged. The downstream
`asha-demo` live proof performs this teardown and reconstruction after the
combined feedback frame, then verifies all five domains are visible or active
again.

## Independent degradation

Host failures are isolated after scene application. They produce typed,
origin-preserving diagnostics and do not reject the accepted gameplay fact or
prevent unrelated hosts from processing their operations.

| Missing capability/resource | Diagnostic evidence | Authority consequence |
| --- | --- | --- |
| audio resource or audio host | `hostFailure` or `unavailableHost` | none |
| particle sprite or particle host | `spriteLoadFailed` or `unavailableHost` | none |
| billboard font or billboard host | `fontLoadFailed` or `unavailableHost` | none |
| telemetry overlay realization | `hostFailure` | live snapshot remains readable |
| animation target/clip or animation host | `unknownTarget`, `clipMissing`, or `unavailableHost` | controller state remains authoritative |

The focused renderer-host tests cover each degraded path. The downstream
browser proof also invokes four separate public probes for a missing audio
resource, particle sprite, billboard font, and overlay realization. Each probe
returns an origin-preserving typed diagnostic, verifies the RuntimeSession hash
and interaction state did not change, and appends its result to the visible
game HUD. The healthy composition, exact shared origin, operation ordering,
cleanup, and reconstruction remain independently asserted before degradation.

## Downstream evidence

`asha-demo` exposes an `integratedFeedbackEvidence` readout with:

- authority tick and replay scope;
- presentation-host generation;
- exact operation-domain ordering;
- the shared origin and an `originConsistent` check;
- per-domain applied and diagnostic counts;
- merged origin-preserving diagnostics.

The live screenshots and readout are written under `artifacts/5654`. This is a
proof surface for downstream consumption, not a new authority API or a private
bridge.
