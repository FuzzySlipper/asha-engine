import type { PickRay, PickResult, RenderFrameDiff } from '@asha/contracts';
import { RuntimeBridgeError, type CompositionStatus, type RuntimeBridge, type ProjectBundleLoadRequest } from '@asha/runtime-bridge';
import { type EditorControl, type MaterialOption } from '@asha/ui-dom';
import { type EditorInspection } from '@asha/devtools';
import { ThreeRenderer } from '@asha/renderer-three/backend';
import { VoxelEditController, type CommandResultHandler } from './index.js';
import { AppEditorInputComposition, type EditorCameraInputPort } from './editor-input-composition.js';
import type { CommandResult } from '@asha/runtime-bridge';
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
export declare function threeRendererPort(renderer?: ThreeRenderer): RendererPort;
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
 * A runtime-selectable fixture/world the shell can load. Selection is data, not a
 * compile-time switch: the host offers a catalog and the user/agent picks one at
 * runtime. `materials` seeds the accessible material palette for the editor.
 */
export interface FixtureChoice {
    readonly id: string;
    readonly label: string;
    /** Catalog material ids this fixture exposes (drives the material palette). */
    readonly materials: readonly number[];
    readonly request: ProjectBundleLoadRequest;
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
    readonly error?: {
        readonly kind: string;
        readonly message: string;
    };
}
export interface WorldStatus {
    readonly fixtureId: string;
    readonly fixtureLabel: string;
    readonly loaded: boolean;
    readonly composition: CompositionStatus | null;
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
/** Compose the application shell from injected host capabilities. */
export declare function composeAppShell(config: AppShellConfig): AppShell;
export declare class AppShell {
    #private;
    readonly host: HostCapabilities;
    readonly controller: VoxelEditController;
    constructor(config: AppShellConfig);
    /** The currently selected fixture. */
    get activeFixture(): FixtureChoice;
    /** Runtime-selectable fixture switch. Clears prior load state; does not auto-load. */
    selectFixture(id: string): void;
    /**
     * Load the active fixture through the facade. Captures the composition status for the
     * readout; a native-gap failure downgrades the runtime to `degraded` instead of
     * pretending the world loaded.
     */
    loadActiveFixture(): WorldStatus;
    /**
     * Read the authority render projection *through the facade* and apply it to the
     * injected renderer. Reference facades emit no authority diffs — that is reported
     * honestly (applied=false), not faked with a local frame. A native-gap failure
     * downgrades the runtime to `degraded`.
     */
    projectAuthority(): RendererStatus;
    /** Cast a pointer-built ray against authority and update selection (single path). */
    pick(ray: PickRay): PickResult;
    /**
     * Drive an accessible control by its stable id. Editor controls route through
     * `controlToAction` into the ONE store; the app-level command buttons map to the
     * controller (`commit` submits the proposal, `cancel` clears the draft).
     */
    applyControl(id: string, value: string): void;
    /**
     * Compose the browser-safe resolved editor input path against this shell's one
     * editor controller. Browser/Electron hosts attach the returned host to DOM and
     * drain it from their render/update loop; headless callers can drive it directly.
     */
    createEditorInput(camera: EditorCameraInputPort): AppEditorInputComposition | null;
    /** The accessible material palette for the active fixture's catalog materials. */
    palette(): MaterialOption[];
    runtimeStatus(): RuntimeStatus;
    worldStatus(): WorldStatus;
    rendererStatus(): RendererStatus;
    /** The devtools inspection of the ONE editor context. */
    editorInspection(): EditorInspection;
    /** The accessible control set for the ONE editor context. */
    controls(): EditorControl[];
    fixtureListing(): FixtureListing[];
    /** The full snapshot for an agent/human dashboard or a CI-safe launch report. */
    readout(): ShellReadout;
}
/** Render a {@link ShellReadout} as a stable, multi-line text report (CLI/launch). */
export declare function formatReadout(readout: ShellReadout): string;
//# sourceMappingURL=shell.d.ts.map