# Screenshot goldens — deferred (intentional placeholder)

This directory is an intentional placeholder (`.gitkeep` only). Pixel/screenshot
goldens — diffing a real WebGL/offscreen render against committed images — are
**deferred**.

The current render gate is the **structural** snapshot in
`harness/goldens/render-diffs/` (per-handle scene-graph state, built without a GL
context). It is deterministic and CI-friendly; image diffs would add flakiness
(GPU/driver/AA differences) without catching more *logic* regressions than the
structural snapshot already does.

Adding pixel goldens is a separate, explicit task: it needs a headless GL/offscreen
renderer, a stable capture environment, and a tolerance/diff strategy. Until then,
do not add binary images here — extend the structural snapshots instead.
