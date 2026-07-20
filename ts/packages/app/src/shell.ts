// @asha/app — the transport-agnostic composition root for the launchable voxel shell
// (launchable-voxel-01-app-shell-dev-target / task #2439).
//
// `AppShell` is the single place that *assembles* the running application: it owns one
// `EditorStore` (through the `VoxelEditController`) and projects it for the renderer,
// the accessible UI control model, and the devtools read model — so there is exactly
// one editor state, never parallel copies. It receives host-specific capabilities
// (renderer port, bridge boot, fixtures) by injection and imports no Electron/browser
// globals, so the same composition runs under Electron, a browser, or headless tests.
//
// Authority stays in Rust: the shell reads render projections *through the facade* and
// submits commands/picks through the approved bridge path. A missing or unwired native
// capability surfaces as a visible `degraded`/`unavailable` runtime status — never a
// silent downgrade to mock success.

import type { PickRay, PickResult, RenderFrameDiff } from '@asha/contracts';
import {
  createRuntimeSessionFacade,
  frameCursor,
  RuntimeBridgeError,
  type RuntimeBridge,
} from '@asha/runtime-bridge';
import type {
  RuntimeSessionFacade,
  RuntimeSessionProjectLoadReceipt,
  RuntimeSessionProjectSource,
} from '@asha/runtime-session';
import {
  buildEditorControls,
  controlToAction,
  materialPalette,
  type EditorControl,
  type MaterialOption,
} from '@asha/ui-dom';
import { inspectEditor, type EditorInspection } from '@asha/devtools';
import { ThreeRenderer } from '@asha/renderer-three/backend';

import {
  VoxelEditController,
  bridgeCommandSink,
  bridgePicker,
  pickAndSelect,
  type CommandResultHandler,
} from './index.js';
import {
  AppEditorInputComposition,
  type EditorCameraInputPort,
} from './editor-input-composition.js';
import type { CommandResult } from '@asha/runtime-bridge';

// ── Injected host capabilities ────────────────────────────────────────────────

/**
 * Host-specific capabilities injected into the composition root. The shell never
 * imports Electron/browser globals directly; a host (Electron main, a browser entry,
 * or a headless test) supplies these so the composition stays transport-agnostic.
 */
export interface HostCapabilities {
  /** Host id for the readout: `electron` | `browser` | `headless`. */
  readonly name: string;
  /**
   * Whether the host exposes an accessibility tree the control model can drive. The
   * control descriptors are always accessible; this records whether the host renders
   * them into a real a11y tree (Electron/browser) vs. a headless model-only target.
   */
  readonly accessibility: boolean;
}

/**
 * The renderer seam the shell drives. A port (not `ThreeRenderer` directly) keeps the
 * composition root free of a hard three.js/GL dependency and trivially fakeable in
 * tests; {@link threeRendererPort} adapts the real renderer.
 */
export interface RendererPort {
  applyFrame(diff: RenderFrameDiff): void;
  /** Live count of nodes under the `scene` layer (projection evidence). */
  readonly sceneNodeCount: number;
}

/** Adapt the real `@asha/renderer-three` renderer to the {@link RendererPort} seam. */
export function threeRendererPort(renderer: ThreeRenderer = new ThreeRenderer()): RendererPort {
  return {
    applyFrame: (diff) => renderer.applyFrame(diff),
    get sceneNodeCount() {
      return renderer.scene.getObjectByName('scene')?.children.length ?? 0;
    },
  };
}

/**
 * How the shell obtains a runtime bridge. Mirrors the smoke harness's boot shape but is
 * defined here so `app` (the composition root) does not depend on `@asha/smoke`. A
 * `null` bridge means boot failed closed (e.g. native addon unavailable) and must be
 * reported as `unavailable`, never hidden.
 */
export interface AppBridgeBoot {
  readonly bridge: RuntimeBridge | null;
  readonly mode: 'native' | 'mock';
  readonly intent: 'authority' | 'reference';
  readonly nativeAvailable: boolean;
  /** Classified reason the bridge is null (required when `bridge` is null). */
  readonly bootError?: RuntimeBridgeError;
}

/**
 * A runtime-selectable canonical project source. Selection is data, not a
 * compile-time switch. `materials` seeds the accessible editor palette.
 */
export interface FixtureChoice {
  readonly id: string;
  readonly label: string;
  /** Catalog material ids this fixture exposes (drives the material palette). */
  readonly materials: readonly number[];
  readonly source: RuntimeSessionProjectSource;
}

/** Everything the host injects to compose the shell. */
export interface AppShellConfig {
  readonly host: HostCapabilities;
  readonly bootBridge: () => AppBridgeBoot;
  /** Non-empty runtime-selectable fixture catalog. */
  readonly fixtures: readonly FixtureChoice[];
  /** Injected renderer seam; omit for a UI-only composition (no projection). */
  readonly renderer?: RendererPort;
  /** Which fixture is active at boot (defaults to the first). */
  readonly initialFixtureId?: string;
  /** Observe each classified command result (diagnostics/UI). */
  readonly onCommandResult?: CommandResultHandler;
}

// ── Read models (the agent/human-navigable shell snapshot) ─────────────────────

/**
 * The runtime tier the shell is honestly running in:
 * - `native`: the real native authority addon is wired.
 * - `reference`: the deterministic mock/reference facade (faithful, offline).
 * - `degraded`: booted, but a needed capability failed closed at runtime
 *   (`operation_unimplemented` / `native_unavailable`) — visible, never hidden.
 * - `unavailable`: no bridge at all (boot failed closed).
 */
export type RuntimeAvailability = 'native' | 'reference' | 'degraded' | 'unavailable';

export interface RuntimeStatus {
  readonly availability: RuntimeAvailability;
  readonly mode: 'native' | 'mock';
  readonly intent: 'authority' | 'reference';
  readonly nativeAvailable: boolean;
  readonly detail: string;
  /** The classified error behind a `degraded`/`unavailable` status, if any. */
  readonly error?: { readonly kind: string; readonly message: string };
}

export interface WorldStatus {
  readonly fixtureId: string;
  readonly fixtureLabel: string;
  readonly loaded: boolean;
  readonly project: RuntimeSessionProjectLoadReceipt | null;
  readonly detail: string;
}

export interface RendererStatus {
  readonly present: boolean;
  readonly applied: boolean;
  readonly sceneNodes: number;
  readonly source: 'authority' | 'none';
  readonly detail: string;
}

export interface FixtureListing {
  readonly id: string;
  readonly label: string;
  readonly active: boolean;
}

/** The full, deterministic shell snapshot — one object an agent or human can read. */
export interface ShellReadout {
  readonly host: HostCapabilities;
  readonly runtime: RuntimeStatus;
  readonly world: WorldStatus;
  readonly renderer: RendererStatus;
  /** The devtools editor inspection — derived from the ONE editor store. */
  readonly editor: EditorInspection;
  /** The accessible control model — derived from the SAME editor store. */
  readonly controls: readonly EditorControl[];
  readonly fixtures: readonly FixtureListing[];
  /** The most recent classified command result, if any edit has been committed. */
  readonly lastCommandResult: CommandResult | null;
}

// ── The composition root ───────────────────────────────────────────────────────

/** Compose the application shell from injected host capabilities. */
export function composeAppShell(config: AppShellConfig): AppShell {
  return new AppShell(config);
}

/** Whether a classified bridge error signals a missing/unwired *native* capability. */
function isNativeGap(error: RuntimeBridgeError): boolean {
  return error.kind === 'native_unavailable' || error.kind === 'operation_unimplemented';
}

export class AppShell {
  readonly host: HostCapabilities;
  readonly controller: VoxelEditController;

  readonly #boot: AppBridgeBoot;
  readonly #bridge: RuntimeBridge | null;
  readonly #session: RuntimeSessionFacade | null;
  readonly #renderer: RendererPort | null;
  readonly #fixtures: readonly FixtureChoice[];

  #activeFixtureId: string;
  #project: RuntimeSessionProjectLoadReceipt | null = null;
  #projectLoaded = false;
  #renderApplied = false;
  #renderSource: 'authority' | 'none' = 'none';
  #renderDetail = 'no projection applied yet';
  /** A runtime-observed native gap that downgrades the status to `degraded`. */
  #degradation: RuntimeBridgeError | null = null;
  #lastCommandResult: CommandResult | null = null;

  constructor(config: AppShellConfig) {
    if (config.fixtures.length === 0) {
      throw new Error('app shell requires a non-empty fixture catalog');
    }
    this.host = config.host;
    this.#boot = config.bootBridge();
    this.#bridge = this.#boot.bridge;
    this.#session = this.#bridge
      ? createRuntimeSessionFacade({
          bridge: this.#bridge,
          mode: this.#boot.mode === 'native' ? 'rust' : 'reference',
        })
      : null;
    this.#renderer = config.renderer ?? null;
    this.#fixtures = config.fixtures;
    this.#activeFixtureId = config.initialFixtureId ?? config.fixtures[0]!.id;
    if (!this.#fixtures.some((f) => f.id === this.#activeFixtureId)) {
      throw new Error(`unknown initial fixture '${this.#activeFixtureId}'`);
    }

    // The ONE editor store lives in the controller; UI, devtools, and the renderer
    // overlay all read from it — there is no second state model.
    const sink = this.#bridge
      ? bridgeCommandSink(this.#bridge, (result) => {
          this.#lastCommandResult = result;
          config.onCommandResult?.(result);
        })
      : // No bridge → committing is not possible; record the gap rather than no-op.
        () => {
          this.#degradation =
            this.#boot.bootError ??
            new RuntimeBridgeError('native_unavailable', 'no runtime bridge to submit commands');
        };
    this.controller = new VoxelEditController(sink);

    if (this.#session) {
      try {
        this.#session.initialize({
          sessionId: `asha-shell.${this.host.name}`,
          seed: 1,
          project: { gameId: 'asha-shell', workspaceId: 'app-shell' },
        });
      } catch (cause) {
        this.#captureDegradation(cause);
      }
    }
  }

  /** The currently selected fixture. */
  get activeFixture(): FixtureChoice {
    return this.#fixtures.find((f) => f.id === this.#activeFixtureId)!;
  }

  /** Runtime-selectable fixture switch. Clears prior load state; does not auto-load. */
  selectFixture(id: string): void {
    if (!this.#fixtures.some((f) => f.id === id)) {
      throw new Error(`unknown fixture '${id}'`);
    }
    this.#activeFixtureId = id;
    if (this.#projectLoaded && this.#session) {
      this.#session.closeProject();
    }
    this.#project = null;
    this.#projectLoaded = false;
    this.#renderApplied = false;
    this.#renderSource = 'none';
    this.#renderDetail = 'fixture changed; reload to project';
  }

  /**
   * Load the active fixture through the facade. Captures the composition status for the
   * readout; a native-gap failure downgrades the runtime to `degraded` instead of
   * pretending the world loaded.
   */
  async loadActiveFixture(): Promise<WorldStatus> {
    if (!this.#session) {
      this.#projectLoaded = false;
      this.#project = null;
      return this.worldStatus();
    }
    try {
      if (this.#projectLoaded) {
        this.#session.closeProject();
      }
      this.#project = await this.#session.loadProject({ source: this.activeFixture.source });
      this.#projectLoaded = this.#project.accepted;
    } catch (cause) {
      this.#projectLoaded = false;
      this.#captureDegradation(cause);
    }
    return this.worldStatus();
  }

  /**
   * Read the authority render projection *through the facade* and apply it to the
   * injected renderer. Reference facades emit no authority diffs — that is reported
   * honestly (applied=false), not faked with a local frame. A native-gap failure
   * downgrades the runtime to `degraded`.
   */
  projectAuthority(): RendererStatus {
    if (!this.#renderer) {
      this.#renderDetail = 'no renderer injected';
      return this.rendererStatus();
    }
    if (!this.#bridge) {
      this.#renderDetail = 'no bridge to read authority diffs';
      return this.rendererStatus();
    }
    this.#renderSource = 'authority';
    try {
      const frame = this.#bridge.readRenderDiffs(frameCursor(0));
      this.#renderer.applyFrame(frame);
      this.#renderApplied = this.#renderer.sceneNodeCount > 0;
      this.#renderDetail = this.#renderApplied
        ? `applied authority projection; scene nodes=${this.#renderer.sceneNodeCount}`
        : 'authority facade emitted no render diffs (reference mode has no authority projection)';
    } catch (cause) {
      this.#renderApplied = false;
      this.#renderDetail = cause instanceof Error ? cause.message : String(cause);
      this.#captureDegradation(cause);
    }
    return this.rendererStatus();
  }

  /** Cast a pointer-built ray against authority and update selection (single path). */
  pick(ray: PickRay): PickResult {
    if (!this.#bridge) {
      this.controller.store.dispatch({ type: 'clearSelection' });
      return { outcome: 'miss', rejection: { reason: 'noHit' } };
    }
    try {
      return pickAndSelect(this.controller.store, bridgePicker(this.#bridge), ray);
    } catch (cause) {
      this.#captureDegradation(cause);
      this.controller.store.dispatch({ type: 'clearSelection' });
      return { outcome: 'miss', rejection: { reason: 'noHit' } };
    }
  }

  /**
   * Drive an accessible control by its stable id. Editor controls route through
   * `controlToAction` into the ONE store; the app-level command buttons map to the
   * controller (`commit` submits the proposal, `cancel` clears the draft).
   */
  applyControl(id: string, value: string): void {
    if (id === 'commit') {
      this.controller.commit();
      return;
    }
    if (id === 'cancel') {
      this.controller.cancel();
      return;
    }
    const action = controlToAction(id, value);
    if (action) {
      this.controller.store.dispatch(action);
    }
  }

  /**
   * Compose the browser-safe resolved editor input path against this shell's one
   * editor controller. Browser/Electron hosts attach the returned host to DOM and
   * drain it from their render/update loop; headless callers can drive it directly.
   */
  createEditorInput(camera: EditorCameraInputPort): AppEditorInputComposition | null {
    if (this.#bridge === null) {
      return null;
    }
    return new AppEditorInputComposition({
      session: this.#bridge,
      editor: this.controller,
      camera,
    });
  }

  // ── Read models ──────────────────────────────────────────────────────────────

  /** The accessible material palette for the active fixture's catalog materials. */
  palette(): MaterialOption[] {
    return materialPalette(this.activeFixture.materials);
  }

  runtimeStatus(): RuntimeStatus {
    if (!this.#bridge) {
      const err = this.#boot.bootError;
      return {
        availability: 'unavailable',
        mode: this.#boot.mode,
        intent: this.#boot.intent,
        nativeAvailable: this.#boot.nativeAvailable,
        detail: err ? err.message : 'runtime bridge failed to boot',
        ...(err ? { error: { kind: err.kind, message: err.message } } : {}),
      };
    }
    if (this.#degradation) {
      return {
        availability: 'degraded',
        mode: this.#boot.mode,
        intent: this.#boot.intent,
        nativeAvailable: this.#boot.nativeAvailable,
        detail: `runtime degraded: ${this.#degradation.message}`,
        error: { kind: this.#degradation.kind, message: this.#degradation.message },
      };
    }
    const native = this.#boot.mode === 'native' && this.#boot.intent === 'authority';
    return {
      availability: native ? 'native' : 'reference',
      mode: this.#boot.mode,
      intent: this.#boot.intent,
      nativeAvailable: this.#boot.nativeAvailable,
      detail: native
        ? 'native authority runtime'
        : 'reference (mock) runtime — deterministic, no native authority',
    };
  }

  worldStatus(): WorldStatus {
    const fixture = this.activeFixture;
    return {
      fixtureId: fixture.id,
      fixtureLabel: fixture.label,
      loaded: this.#projectLoaded,
      project: this.#project,
      detail: this.#projectLoaded
        ? `loaded project ${this.#project?.activeProject?.projectId ?? '?'}`
        : this.#bridge
          ? 'fixture not loaded'
          : 'no bridge to load fixture',
    };
  }

  rendererStatus(): RendererStatus {
    return {
      present: this.#renderer !== null,
      applied: this.#renderApplied,
      sceneNodes: this.#renderer?.sceneNodeCount ?? 0,
      source: this.#renderSource,
      detail: this.#renderDetail,
    };
  }

  /** The devtools inspection of the ONE editor context. */
  editorInspection(): EditorInspection {
    return inspectEditor(this.controller.store.getState());
  }

  /** The accessible control set for the ONE editor context. */
  controls(): EditorControl[] {
    return buildEditorControls(this.controller.store.getState(), this.palette());
  }

  fixtureListing(): FixtureListing[] {
    return this.#fixtures.map((f) => ({ id: f.id, label: f.label, active: f.id === this.#activeFixtureId }));
  }

  /** The full snapshot for an agent/human dashboard or a CI-safe launch report. */
  readout(): ShellReadout {
    return {
      host: this.host,
      runtime: this.runtimeStatus(),
      world: this.worldStatus(),
      renderer: this.rendererStatus(),
      editor: this.editorInspection(),
      controls: this.controls(),
      fixtures: this.fixtureListing(),
      lastCommandResult: this.#lastCommandResult,
    };
  }

  #captureDegradation(cause: unknown): void {
    if (cause instanceof RuntimeBridgeError && isNativeGap(cause)) {
      this.#degradation = cause;
    } else if (cause instanceof RuntimeBridgeError) {
      // A non-native classified error (invalid_input, not_initialized…) is still a
      // visible degradation of this run, not a silent pass.
      this.#degradation = cause;
    } else {
      this.#degradation = new RuntimeBridgeError('internal', cause instanceof Error ? cause.message : String(cause));
    }
  }
}

/** Render a {@link ShellReadout} as a stable, multi-line text report (CLI/launch). */
export function formatReadout(readout: ShellReadout): string {
  const lines: string[] = [];
  lines.push(`asha-shell: host=${readout.host.name} accessibility=${readout.host.accessibility}`);
  lines.push(
    `runtime: ${readout.runtime.availability} (mode=${readout.runtime.mode} intent=${readout.runtime.intent} ` +
      `nativeAvailable=${readout.runtime.nativeAvailable}) — ${readout.runtime.detail}`,
  );
  lines.push(`world: ${readout.world.fixtureId} loaded=${readout.world.loaded} — ${readout.world.detail}`);
  lines.push(
    `renderer: present=${readout.renderer.present} applied=${readout.renderer.applied} ` +
      `nodes=${readout.renderer.sceneNodes} source=${readout.renderer.source} — ${readout.renderer.detail}`,
  );
  lines.push(
    `editor: tool=${readout.editor.tool} material=${readout.editor.material} ` +
      `brushShape=${readout.editor.brushShape} affectedCells=${readout.editor.affectedCells}`,
  );
  lines.push(`fixtures: ${readout.fixtures.map((f) => `${f.id}${f.active ? '*' : ''}`).join(' ')}`);
  lines.push(`controls: ${readout.controls.map((c) => `${c.id}[${c.role}]${c.disabled ? ':disabled' : ''}`).join(' ')}`);
  if (readout.lastCommandResult) {
    lines.push(
      `lastCommand: accepted=${readout.lastCommandResult.accepted} rejected=${readout.lastCommandResult.rejected}`,
    );
  }
  return lines.join('\n') + '\n';
}
