---
status: current
audience: agent
tags: [projection, channel, adr]
supersedes: []
see-also: []
---

# ADR: Shared non-scene projection channel

Status: **Accepted for Wave 1**  
Task: #5612  
Parent campaign: #5640

## Context

`RenderFrameDiff` is an established retained scene protocol. It owns scene
nodes, assets, transforms, and renderer handles. Audio, world-space UI,
particles, animation-controller realization, and the live telemetry overlay
also need a Rust-to-TypeScript projection path, but they must not grow five
unrelated bridge operations or turn `RenderDiff` into an all-purpose event bus.

Presentation is derived and disposable. Gameplay meaning must already exist as
an owner fact, gameplay event, decision outcome, or typed authority state before
it is projected. Renderer, audio, DOM, and animation callbacks never become
authority or replay truth.

## Decision

Use a **hybrid typed envelope**:

- Keep `RenderFrameDiff` unchanged for retained scene projection.
- Add a generated `RuntimeProjectionFrame` that carries one scene frame and one
  ordered `PresentationFrameDiff` for the same authority tick.
- Give every non-scene domain a closed typed operation enum under the shared
  presentation frame: audio, billboard, particle, animation, and telemetry
  overlay.
- Share lifecycle names and frame/evidence rules, but keep payloads and handle
  types domain-specific.

The presentation-domain union is deliberately closed. A new host capability is
a generated-contract change with decoder, host, fixture, and reachability
coverage. Downstream gameplay meaning remains open through the gameplay fabric;
renderer-host capabilities do not.

The recipe for preserving one accepted origin across several disposable
presentations is documented in
[Gameplay fabric growth recipes](gameplay-fabric-growth-recipes.md).
Presentation consumers retain causation/correlation identity but never route
callbacks back into authority.

## Protocol sketch

Names may change mechanically during implementation, but the shape and rules
below are fixed.

```rust
pub struct RuntimeProjectionFrame {
    pub schema_version: u16,
    pub authority_tick: u64,
    pub scene: RenderFrameDiff,
    pub presentation: PresentationFrameDiff,
}

pub struct PresentationFrameDiff {
    pub replay_scope: ProjectionReplayScope,
    pub ops: Vec<PresentationOp>,
}

pub enum ProjectionReplayScope {
    ExcludedFromReplayTruth,
}

pub enum PresentationOriginKind {
    OwnerFact,
    GameplayEvent,
    DecisionOutcome,
    CapabilityState,
}

pub struct PresentationOpMeta {
    pub sequence: u32,
    pub origin: Option<PresentationOriginRef>,
}

pub struct PresentationOriginRef {
    pub kind: PresentationOriginKind,
    pub id: String,
    pub authority_tick: u64,
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
}

pub enum PresentationOp {
    Audio { meta: PresentationOpMeta, op: AudioProjectionOp },
    Billboard { meta: PresentationOpMeta, op: BillboardProjectionOp },
    Particle { meta: PresentationOpMeta, op: ParticleProjectionOp },
    Animation { meta: PresentationOpMeta, op: AnimationProjectionOp },
    TelemetryOverlay { meta: PresentationOpMeta, op: TelemetryOverlayProjectionOp },
}
```

Each domain operation enum uses the valid subset of four shared verbs:

- `emit`: one-shot cue with a stable signal id; no retained handle;
- `create`: allocate a retained domain handle with a full descriptor;
- `update`: change a retained handle with a typed domain patch;
- `destroy`: retire a retained handle.

For example, audio may support all four verbs, a billboard uses
create/update/destroy, a particle burst uses emit while a continuous emitter
uses create/update/destroy, and a telemetry overlay uses create/update/destroy.
An animation descriptor may reference a scene `RenderHandle`, but its own
projection lifecycle uses an `AnimationProjectionHandle`.

Rust owns the source shapes in a new `protocol-presentation` crate and
`protocol-codegen` generates `@asha/contracts` TypeScript. Generated optional
fields use the existing nullable contract convention; handwritten
`undefined`-based transport variants are not permitted.

## Handle and diagnostic rules

- Use branded per-domain newtypes such as `AudioHandle`, `BillboardHandle`,
  `ParticleEmitterHandle`, `AnimationProjectionHandle`, and
  `TelemetryOverlayHandle`.
- Do not reuse `RenderHandle` as a universal presentation handle and do not use
  one untyped global integer registry.
- Cross-domain diagnostics use a composite address `(domain, raw_handle)` plus
  the operation sequence and optional origin reference.
- A handle is stable from create through destroy, cannot change domain, and is
  not reused within a Session generation.
- Duplicate create, unknown update/destroy, invalid domain payload, missing
  anchor, and unavailable host capability produce typed diagnostics.

## Ordering, batching, and failure

One `RuntimeProjectionFrame` represents one authority tick. Consumers apply it
in this order:

1. validate the outer frame and contiguous presentation `sequence` values;
2. apply `scene.ops` in their existing authored order;
3. dispatch `presentation.ops` in ascending sequence without regrouping by
   domain.

Scene-first application lets a presentation create resolve a scene handle or
entity projection created in the same tick. A presentation destroy needs only
its domain handle, so it remains valid after a scene target is removed.

Malformed outer framing rejects the whole frame. After framing is valid, a
missing audio/particle/font/overlay host or a domain-local bad resource fails
that operation visibly and does not prevent scene projection or other domains
from applying. Diagnostics retain origin/correlation data so the combined
feedback proof can explain partial degradation.

The runtime bridge should add an additive `readProjectionFrame(cursor)` path.
During migration, `readRenderDiffs(cursor)` may remain as a compatibility view
that returns `frame.scene`; it must not become a second source of projection
truth.

## Replay and authority boundary

`PresentationFrameDiff.replay_scope` is always
`excluded_from_replay_truth` in Wave 1. The marker lives once on the generated
channel, not independently on every domain descriptor.

- Presentation operations are not `DomainEvent`s and do not mutate Session
  state.
- Host completion, dropped playback, animation keyframes, and DOM callbacks do
  not propose gameplay mutation.
- Verification replay may rerun projectors from recorded authority and compare
  presentation diagnostics, but recorded presentation operations are never
  applied as canonical state.
- `PresentationOriginRef` is trace metadata, not an authority payload. It points
  back to the fact, event, decision, or typed state that already owns meaning.

## Existing surface alignment

- **Scene compatibility:** existing scene sprites, debug-layer nodes, mesh
  definitions, and retained `RenderHandle` operations remain in `RenderDiff`.
  They are scene projection, not precedents for adding audio or overlay payloads.
  The current `SetAnimatedMeshPlayback` variant is a named compatibility path
  for direct clip callers; #5649 migrates controller-driven playback to the G1
  animation domain and records the compatibility path's deletion condition.
- **HUD:** `@asha/ui-dom` remains a pure data-in/descriptor-out screen-space
  model with typed intents. G1 world-space UI uses the billboard domain and
  follows the same no-hidden-authority posture; HUD controls do not travel on
  the presentation channel.
- **Cosmetic:** `@asha/cosmetic` remains a documented TypeScript-local
  compatibility surface for `screen_flash`, `hit_spark`, and `view_kick` in
  Wave 1. A Rust-origin effect must enter through a typed G1 domain and may be
  adapted one-way into a cosmetic view model. Local UI effects do not travel
  backward across the Rust border. Migration is deferred until a child task
  needs it.
- **Telemetry:** the machine-readable telemetry snapshot remains observational
  data owned by its telemetry contract. Only its visible overlay descriptor and
  lifecycle use the presentation channel.

## Consequences for Wave 1 tasks

- #5595 defines typed audio descriptors/patches and its host under the audio
  domain; it does not add audio variants to `RenderDiff`.
- #5597 implements retained billboard create/update/destroy operations,
  domain-branded handles, explicit Font/Texture asset posture, localization
  keys, and an independently failing browser host.
- #5603 uses `emit` for bursts and retained operations for continuous emitters.
- #5606 keeps the machine-readable snapshot separate and projects the visible
  overlay through the telemetry-overlay domain.
- #5649 uses the animation domain and may reference scene render targets without
  inventing another frame or replay convention.
- #5654 proves one accepted origin can correlate several domain operations and
  that one unavailable host degrades independently.

## Rejected alternatives

- Adding audio, billboard, particle, telemetry, and animation variants directly
  to scene `RenderDiff`.
- One generic payload (`Any`, JSON, bytes, string method name, or plugin
  callback) interpreted by host registries.
- One universal presentation handle type that permits cross-domain updates.
- Independent per-domain frame ordering or replay markers.
- Treating presentation output, host timing, or callbacks as accepted gameplay
  facts.
