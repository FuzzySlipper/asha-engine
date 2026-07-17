---
status: current
audience: agent
tags: [voxel, ui, editor]
supersedes: []
see-also: []
---

# Voxel UI / editor architecture

> Companion ADR: `governance/adr/0008-ui-editor-architecture.md`.
> Source design: Den `voxel-capability-09-camera-controls-ui-interaction`.

The coherent UI/editor-state structure that must exist **before** voxel UI implementation
lands, so agents don't scatter editor state across DOM components or leak authority. It fixes
package boundaries, the persistent editor-tool-context model, the command submission path, the
inspector read model, and import rules. Cold-start reference for #2265 (command path / preview)
and #2266 (camera / inspectors / overlays).

## 1. Three state categories (the core distinction)

| Category | Owner | Examples | Rule |
|---|---|---|---|
| **Authoritative** | Rust | voxel/chunk/spatial session state | UI **never** mutates directly |
| **Transient DOM/render** | UI components | pointer hover, button press, in-flight drag | throwaway, component-local |
| **Persistent editor tool context** | `@asha/editor-tools` | current tool, brush, size, material, snapping, selection mode, preview settings, current selection | durable TS state; devtools-inspectable; **not** a shadow copy of authority |

The middle category is the trap: editor tool context is neither Rust authority nor throwaway DOM
state. It gets a **dedicated package** so it is one inspectable model, not implicit fields sprayed
across panels.

## 2. Package boundaries

```
contracts ◄── editor-tools ◄── ui-dom ──► app ──► electron-main
                   ▲              (panels)    (composition + command submission)
                   └──────────── devtools (read-only inspection)
```

- **`@asha/editor-tools`** — NEW. The persistent editor-tool-context model: a small observable
  store (state + actions + subscribe). **Pure TS, no DOM, no `three`, no policy, no bridge.**
  Imports `@asha/contracts` only. Produces protocol-typed command *proposals* (`VoxelCommand`);
  it does not submit them. The single home for editor state.
- **`@asha/ui-dom`** — DOM panels / tool palettes / inspectors. Reads `editor-tools` state +
  projected render data; dispatches `editor-tools` actions and surfaces command proposals.
  Imports `@asha/contracts`, `@asha/runtime-bridge`, `@asha/editor-tools`. No policy, no native bridge.
- **`@asha/app`** — composition + the **command submission path**: wires `editor-tools` proposals
  → `@asha/runtime-bridge` (`submitCommands`), and the renderer/UI together.
- **`@asha/devtools`** — read-only inspection of `editor-tools` state + collision/replay diagnostics.

No new framework: plain DOM + a thin internal store (decision 1). A heavier UI framework is a later,
justified choice.

## 3. Persistent editor tool context (`EditorContext`)

```ts
type ToolMode = 'place' | 'remove' | 'select' | 'inspect';
type SelectionMode = 'voxel' | 'face';

interface VoxelSelection {            // from picking (#2244), contract-typed
  readonly voxel: VoxelCoord;
  readonly face: Face;
}

interface EditorContext {
  readonly tool: ToolMode;
  readonly brushSize: number;         // >= 1, in voxels
  readonly material: number;          // VoxelMaterialId raw (place tool)
  readonly snapping: boolean;
  readonly selectionMode: SelectionMode;
  readonly preview: { readonly enabled: boolean };
  readonly selection: VoxelSelection | null;   // current picked anchor
}
```

- An **observable store** (`getState()`, `dispatch(action)`, `subscribe(listener)`) — Redux-shaped
  but dependency-free. Actions are explicit (`setTool`, `setBrushSize`, `setMaterial`,
  `setSelection`, …); reducers are pure and validate (e.g. `brushSize >= 1`).
- **Persistence**: lives once in `app` (not per-component), so it survives camera movement, panel
  remounts, and tab switches. Devtools subscribes for visibility. It is **not** durable-to-disk in
  this phase (that is a later concern), but it is durable across the session UI.
- **Not a shadow of authority**: it holds *what the user is about to do*, never a copy of voxel
  data. `selection` is a picked anchor (coord+face), not voxel contents.

## 4. Command submission path (#2265)

```
user action + EditorContext + selection
  → editor-tools proposeCommand()  → VoxelCommand  (generated contract type)
  → app submits via @asha/runtime-bridge.submitCommands(...)   ← the ONLY mutation route
  → Rust validates + applies  → events  → render diffs  → renderer
```

- `editor-tools.proposeCommand(ctx)` maps `(tool, selection, material, brushSize)` to a
  `VoxelCommand` proposal (place → `setVoxel` at the face-neighbour anchor; remove → `setVoxel`
  Empty at the selection; box brush → `fillRegion`). It **returns** the proposal; it never submits.
- `app` is the only package that calls `submitCommands`. UI never mutates authority.

## 5. Preview vs commit (#2265)

- **Preview** is a non-authoritative render overlay on the `debug` `RenderLayer`, computed from
  `EditorContext` + selection — visually distinct from committed terrain (e.g. translucent / wireframe
  / debug colour). It mutates **nothing**; it is render-only and disappears on commit/deselect.
- **Commit** submits the proposed command through §4. Preview state living in `editor-tools` proves
  it is not authority.

## 6. Inspector read model (#2266)

Inspectors read **projections** (decoded render frames / coordinate readouts) and **editor-tools
diagnostics** — never a hidden authoritative copy. Inspector panels are pure functions of
`(EditorContext, projected data)`. Camera collision/clipping uses the shared collision query
service (`svc-collision` via the bridge) when wired, not a UI-only terrain collision.

## 7. Import rules (enforced by depgraph)

- `editor-tools` imports `@asha/contracts` only — no DOM, no `three`, no policy, no bridge, no
  renderer.
- `ui-dom` / `app` / `devtools` may import `@asha/editor-tools`; policy/catalog may not.
- No UI package imports `@asha/native-bridge` or policy internals (already enforced; extended to
  cover `editor-tools`).
- Registered now (forward-declared) in `ownership.toml` + `dependency-policy.toml`; the package dir
  lands in #2265.

## 8. Test plan (for #2265 / #2266)

1. **Editor state**: actions produce expected state; invalid inputs (brushSize < 1) rejected;
   `subscribe` notified on change; state persists across simulated component remounts.
2. **Import boundaries**: `check-depgraph` rejects `editor-tools`→DOM/three/policy/bridge and any
   UI→native-bridge/policy import.
3. **Command path**: `proposeCommand` returns generated `VoxelCommand` types; only `app` calls
   `submitCommands`; a unit test asserts a place/remove/box action → the expected proposal.
4. **Preview ≠ authority**: enabling preview / changing selection mutates no authoritative state
   (no `submitCommands` call); preview diffs target the `debug` layer.
5. **Inspector read model**: inspector output is a pure function of `(EditorContext, projection)`;
   no hidden state copy.

## 9. Non-goals (this epic)

No full UI implementation in #2264 (design only). Deferred beyond this epic: durable-to-disk editor
prefs; undo/redo (decision deferred — likely authoritative command history / replay inversion, not a
UI shadow); a UI framework; multi-selection/marquee; gizmos; material-atlas picker.
