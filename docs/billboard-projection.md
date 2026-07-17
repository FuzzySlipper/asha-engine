---
status: current
audience: agent
tags: [projection, billboard, render]
supersedes: []
see-also: []
---

# World-space billboard projection

Status: implemented Wave 1 surface  
Task: #5597  
Envelope decision: [Shared non-scene projection channel](non-scene-projection-channel.md)

Billboards turn authority/read-view state into disposable world-space labels,
values, and icons. Rust validates the descriptor, catalog references, retained
handle lifecycle, and origin. The browser host decides how those descriptors
become pixels. A label never gains click-driven gameplay authority and its DOM
or renderer state is never replay truth.

## End-to-end path

```text
authority CapabilityState / accepted owner fact
  -> render-billboard validates descriptor, catalog refs, and retained handle
  -> protocol-presentation BillboardProjectionOp in RuntimeProjectionFrame
  -> stable RuntimeBridge.readProjectionFrame(cursor)
  -> RuntimeSessionFacade.readProjection().runtimeFrame
  -> applyAshaRuntimeProjectionFrame (scene first, then closed domains in sequence)
  -> AshaBillboardHost resolves anchors, localization/assets, culling, and pixels
```

The first live route projects two entity-linked cues after accepted primary
fire: a player identity cue and an enemy health value. Both retain
`capabilityState` origins and Session correlation. Later fire frames update the
enemy handle rather than inventing a second authoritative health source.

## Generated contract

`protocol-presentation` owns:

- `BillboardHandle`, separate from audio and scene render handles;
- `BillboardAnchor`: fixed world position or entity plus local offset;
- `BillboardContent`: localized text with bounded arguments, localized value,
  or catalog-hash-bound texture icon;
- `BillboardFontRef`: explicit system-family or catalog-hash-bound Font asset;
- fixed pixel height, maximum distance, color/background, visibility, and
  `alwaysOnTop` / `depthTested` / `occluded` layer policy;
- `create`, `update`, and `destroy` operations plus typed diagnostics/readout.

The contract contains no DOM node, canvas texture, Three.js object, CSS class,
or callback. It describes presentation intent and provenance, not a renderer
implementation.

## Font, icon, and localization posture

`Font` is a first-class `core-assets::AssetKind` with the `font/...` prefix.
Catalog fonts carry a content hash; the browser hashes resolved bytes with
SHA-256 before calling `FontFace`. System font families are an explicit host
fallback posture, not a claim of identical glyph metrics across machines.

Icons use existing `texture/...` assets and the same projected/catalog/actual
hash agreement. Wave 1 does not create a second icon asset kind or embed an
atlas convention in billboard contracts.

Durable semantics live in localization keys and typed values. Fallback strings
make missing localization visible and keep fixtures readable; they are not
stable gameplay identifiers. Text arguments are named and bounded. Value
descriptors keep the authority-derived value separate from localized label and
unit keys.

## Rust validation and lifecycle

`render-billboard::BillboardProjector` rejects an operation before it enters the
public frame when:

- world positions, offsets, colors, pixel size, or culling distance are invalid;
- localization keys, fallback/value text, or argument collections exceed bounds;
- a Font or Texture id has the wrong kind, is missing, or has a stale hash;
- a retained handle is duplicated, updated after destruction, or destroyed
  while unknown.

Updates are atomic: Rust applies the patch to a copy, validates the complete
result, and replaces retained projection state only after acceptance. Restart
clears every billboard handle and the latest disposable frame.

## Browser realization and layer semantics

`AshaBillboardHost` is exported from `@asha/renderer-host`. The current browser
adapter uses non-interactive positioned elements over the engine canvas, but
consumers depend on generated descriptors and the host package root rather than
that choice.

The host receives two read-only adapters:

- entity id to current projected world position;
- world position to screen coordinates, distance, depth, viewport membership,
  and occlusion evidence.

`alwaysOnTop` ignores scene occlusion and uses the highest presentation order.
`depthTested` uses projected depth ordering. `occluded` additionally hides when
the renderer adapter reports scene occlusion. All layers obey explicit
visibility, frustum, and maximum-distance culling. `refreshLayout()` follows
moving entities/cameras without changing authority or emitting commands.

Missing anchors, font/icon loads, byte-hash drift, unavailable hosts, and local
host failures become billboard-domain diagnostics. Scene and audio application
continue independently after the shared frame itself passes validation.

## Downstream visible acceptance

`asha-demo` imports only `@asha/renderer-host` and other public roots. Its live
Chromium test fires one accepted game-rule action, observes two native
billboard creates, verifies both entity-linked cues are visible, checks the
enemy health text is the current authority value, and confirms restart removes
the disposable elements. The bounded readout reports active/cull/resource
counts and typed diagnostics.

Demo owns this visible acceptance. Engine regressions cover the generated G1
border, `read_projection_frame`, and the public host without treating those
checks as a product-delivery verdict.

## Wave 1 limits

Interactive billboards, rich text, layout widgets, progress bars, damage-number
burst helpers, automatic batching/instancing, a localization catalog service,
and engine-owned occlusion queries are deferred. Consumers may inject their
renderer-owned world projection/occlusion adapter, but may not use billboard
events as an authority path.
