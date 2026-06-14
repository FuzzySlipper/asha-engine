# Render snapshot goldens (structural)

Each `<name>.snapshot` is the deterministic, text **structural** snapshot of the
Three.js scene after `@asha/renderer-three` applies the matching
`harness/fixtures/render-diffs/<name>.json` fixture. It captures per-handle layer,
shape/asset, transform, visibility, material colours, and sprite framing — built
without a GL context, so it is a pure data snapshot, not pixels.

Consumer: `ts/packages/renderer-three/src/golden.test.ts` (run via
`harness/ci/check-render-goldens.sh`).

## Regenerate

These snapshots are committed and compared by string equality. When the renderer's
structural output changes intentionally:

1. If the *input* fixture is Rust-generated, re-bless it first (see the
   `harness/fixtures/render-diffs/` README).
2. Re-run the snapshot test; update the `<name>.snapshot` file to the new
   `renderer.snapshot()` output shown in the mismatch, and review the diff.

```bash
cd ts && pnpm --filter @asha/renderer-three test
```

## Deferred: pixel/screenshot goldens

These structural snapshots are the **current** render gate. True pixel/screenshot
goldens (a real WebGL/offscreen render) are deferred — `harness/goldens/screenshots/`
is an intentional placeholder (see its README). Structural snapshots are chosen
because they are deterministic and GL-free in CI; a renderer correctness change is
caught here without flaky image diffs.
