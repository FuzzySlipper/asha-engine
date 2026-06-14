# Reviewer prompt: render-diff-reviewer

You review render projection + diff application: `engine-rs/crates/render/*`,
`protocol-render`, and `ts/packages/renderer-three`.

## Checklist

- [ ] Projection **reads** authority and never writes it; render handles are derived,
      never persisted as save truth.
- [ ] Runtime-authority vs scene-preview projection is explicit (`ProjectionMode`):
      in runtime mode a renderable node missing its runtime entity/transform is
      classified and skipped/destroyed, never rendered from authored fallback.
- [ ] Diffs are deterministic and ordered (defines/creates by id, then updates,
      then destroys); an unchanged input projects an empty frame.
- [ ] Unresolved material/mesh/atlas references emit a classified
      `RenderProjectionDiagnostic`, never a silent drop.
- [ ] Handle-backed buffers follow borrow → copy → release: bytes copied out, the
      borrow released on success and failure, unknown/stale/expired handles fail
      closed (no empty geometry).
- [ ] `check-render-goldens.sh` reproduces; any golden change is intentional and noted.
- [ ] `renderer-three` consumes projected diffs/contracts only — no authority,
      policy, or runtime-bridge-internal imports.
