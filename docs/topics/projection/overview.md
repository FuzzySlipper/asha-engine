---
status: current
audience: agent
tags: [projection, render, telemetry, particles, audio, hud, materials, feedback]
supersedes: []
see-also: [render-protocol.md, render-lighting.md, integrated-feedback-projection.md]
---

# Projection Overview

Projection is the derived display/tooling output plane. Rust emits render diffs, telemetry, and feedback descriptors; TypeScript renderer/UI consumes them. Projection is never authority.

## Non-Scene Projection Channel

The G1 presentation channel composes game-facing feedback through a shared non-scene projection surface. It is not a collection of independent overlays. See `docs/non-scene-projection-channel.md` for the ADR.

## Integrated Feedback Projection

The integrated gameplay feedback projection is a game-facing composition path, not a collection of independent effects. See `docs/integrated-feedback-projection.md`.

## Material Feedback

Typed, retained render projection for communicating gameplay state through tint and emission. Gameplay authority chooses state; the renderer visualizes. See `docs/material-feedback.md`.

## HUD / Menu Projection

Reusable UI projection surface for health/status/menu readouts and typed UI intents. Does not execute runtime authority commands. See `docs/hud-menu-projection.md`.

## Live Telemetry Overlay

Machine-readable live telemetry with unavailable-counter posture and G1 overlay lifecycle. See `docs/live-telemetry-overlay.md`.

## Particle Projection

Typed burst/retained particle projection with renderer-owned simulation and budgets. See `docs/particle-projection.md`.

## Billboard Projection

Billboard projection for labels, markers, and screen-space elements. See `docs/billboard-projection.md`.

## Audio Projection

Audio projection for spatial and UI audio events. See `docs/audio-projection.md`.

## Developer Console

Bounded generated-contract snapshot for game pull-down consoles and authoring activity/status presentation. Observational, not authority. See `docs/developer-console-projection.md`.
