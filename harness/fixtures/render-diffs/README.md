# Render-diff fixtures

Committed `RenderFrameDiff` payloads (the encoded create/update/destroy stream a
renderer consumes). They are the shared cross-language fixtures: the Rust render
bridge emits them and `@asha/runtime-bridge` (decode) + `@asha/renderer-three`
consume them. Their rendered scene snapshots live in
`harness/goldens/render-diffs/` (see that dir's README).

| Fixture | Producer | Consumers |
|---|---|---|
| `bridge-sequence.json` | `render-bridge` (`RenderProjector`) | `render-bridge` lib test; `renderer-three` apply test |
| `sample-frame.json` | hand-authored create→update→destroy | `renderer-three` decode test |
| `scene-projection.json`, `scene-projection-sequence.json` | `render-bridge` `ScenePresentationProjector` | `renderer-three` golden snapshot |
| `scene-showcase.json`, `sprite-showcase.json`, `static-mesh-instances.json` | authored showcase frames | `renderer-three` golden snapshots |
| `sprite-atlas.json`, `voxel-materials.json` | Rust projector | `renderer-three` / projector goldens |

## Regenerate

Fixtures produced by the Rust projector are blessed from the projector itself:

```bash
BLESS=1 cargo test -p render-bridge --test scene_projection_golden
```

After re-blessing a fixture, re-run the renderer snapshot check and update the
matching `.snapshot` in `harness/goldens/render-diffs/` if its output intentionally
changed:

```bash
cd ts && pnpm --filter @asha/renderer-three test   # or: bash harness/ci/check-render-goldens.sh
```

Hand-authored fixtures (`sample-frame`, the `*-showcase` frames) are edited
directly; keep them minimal and abstract.
