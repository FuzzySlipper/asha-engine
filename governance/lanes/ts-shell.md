# Lane: ts-shell

## Owns
- `ts/packages/runtime-bridge` — transport-agnostic runtime facade: render-diff decode/stream,
  buffer handles, error taxonomy, mock; selects native/mock/wasm impl (ADR 0006)
- `ts/packages/native-bridge` — raw napi-rs addon wrapper, imported only by `runtime-bridge`
- `ts/packages/wasm-replay-bridge` — replay/golden WASM path, imported by tests/devtools only
- `ts/packages/renderer-three` — Three.js scene, handle registry, geometry/material registries, diff application
- `ts/packages/ui-dom` — DOM panels, inspectors, command palette, state view-models
- `ts/packages/cosmetic` — non-authoritative particles, transient animation, screen effects
- `ts/packages/electron-main` — window/process/IPC/platform integration (main process only)
- `ts/packages/app` — runtime loop, wiring of render diffs, UI commands, policy host

## May import
- `@asha/contracts` in all packages
- `runtime-bridge` may import contracts + `native-bridge`; `native-bridge`/`wasm-replay-bridge`
  may import contracts only
- `renderer-three`, `ui-dom`, `cosmetic`, `app` may import `@asha/runtime-bridge` (the facade) —
  never `@asha/native-bridge` or `@asha/wasm-replay-bridge`
- `app` may import `@asha/script-host`
- `electron-main` runs in its own process; it may not import runtime packages

## Must never import (policy boundary)
- `@asha/policy-core`, `@asha/policy-examples` directly into renderer or UI
- Policy packages may only reach the runtime through `app` → `script-host` wiring
- Renderer packages may not inspect `StateStore` — consume render diffs only

## Required tests
- `runtime-bridge`: facade conformance vs manifest, mock smoke, render-diff decode round-trip.
- `native-bridge`/`wasm-replay-bridge`: transport smoke (built via `check-native.sh` / `check-wasm-replay.sh`).
- `renderer-three`: render diff fixture test — apply a diff batch, assert handle registry state.
- `ui-dom`: command palette emits correct command type on user action.
- `app`: runtime loop wiring test (headless, no renderer).

## Required fixtures
- `harness/fixtures/render-diffs/` — diff batches for renderer fixture tests.
- `harness/goldens/screenshots/` — headless screenshot goldens once renderer is active (Phase 5+).

## Drift smells reviewers should flag
- Renderer package importing a policy package.
- UI package maintaining a shadow copy of authoritative state.
- `app` accumulating feature logic instead of wiring.
- Electron main/preload gaining policy execution or product-domain logic.
- `cosmetic` package influencing replay truth or simulation output.
- `runtime-bridge` exposing raw addon/WASM memory pointers to policy packages.
- A shell package importing `native-bridge`/`wasm-replay-bridge` instead of the `runtime-bridge` facade.

## Launchable-voxel boundaries (reviewer checklist)
- **App shell composition** (`@asha/app` `composeAppShell`): the renderer, UI control
  model, and devtools inspection must read the ONE `EditorStore` — no parallel editor
  state. Host capabilities, bridge boot, fixtures, and the renderer are injected; the
  shell must not import Electron/browser globals. Electron main (`@asha/electron-main`)
  stays window/process only and must not import runtime/renderer/app packages.
- **Accessibility**: editor controls (`ui-dom` `buildEditorControls`) must carry a stable
  id + ARIA role + accessible label so agents (Playwright `getByRole`/`getByLabel`) and
  users can drive them. Flag any control rendered without these.
- **Picking authority path**: a pick must cross the facade (`pickVoxel`) to Rust
  (`svc-collision`); the renderer may only supply a hint that authority re-validates
  (`revalidatePickHint`). Selection is keyed on authoritative voxel coords, never a
  render handle. Flag any renderer-owned coordinate used as truth.
- **Runtime mode honesty**: native/reference/degraded/unavailable must be surfaced — no
  silent native→mock downgrade. The smoke/shell must classify a missing native op, not
  pass on mock behaviour.
- **Preview must not remesh**: editor preview emits debug-layer overlay diffs only
  (handles `≥ OVERLAY_HANDLE_BASE`); it must never mutate authoritative scene geometry or
  submit a command (smoke's preview-remesh guardrail).

## Public API changes that require escalation
- Changes to the render diff stream / decode API in `runtime-bridge` — affects renderer.
- Changes to the bridge manifest (`bridge-manifest.toml`) — boundary change, regenerate glue.
- Changes to Electron IPC surface — affects preload and renderer process boundary.
