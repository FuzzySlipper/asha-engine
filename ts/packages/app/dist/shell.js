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
import { frameCursor, RuntimeBridgeError, } from '@asha/runtime-bridge';
import { buildEditorControls, controlToAction, materialPalette, } from '@asha/ui-dom';
import { inspectEditor } from '@asha/devtools';
import { ThreeRenderer } from '@asha/renderer-three/backend';
import { VoxelEditController, bridgeCommandSink, bridgePicker, pickAndSelect, } from './index.js';
/** Adapt the real `@asha/renderer-three` renderer to the {@link RendererPort} seam. */
export function threeRendererPort(renderer = new ThreeRenderer()) {
    return {
        applyFrame: (diff) => renderer.applyFrame(diff),
        get sceneNodeCount() {
            return renderer.scene.getObjectByName('scene')?.children.length ?? 0;
        },
    };
}
// ── The composition root ───────────────────────────────────────────────────────
/** Compose the application shell from injected host capabilities. */
export function composeAppShell(config) {
    return new AppShell(config);
}
/** Whether a classified bridge error signals a missing/unwired *native* capability. */
function isNativeGap(error) {
    return error.kind === 'native_unavailable' || error.kind === 'operation_unimplemented';
}
export class AppShell {
    host;
    controller;
    #boot;
    #bridge;
    #renderer;
    #fixtures;
    #activeFixtureId;
    #composition = null;
    #worldLoaded = false;
    #renderApplied = false;
    #renderSource = 'none';
    #renderDetail = 'no projection applied yet';
    /** A runtime-observed native gap that downgrades the status to `degraded`. */
    #degradation = null;
    #lastCommandResult = null;
    constructor(config) {
        if (config.fixtures.length === 0) {
            throw new Error('app shell requires a non-empty fixture catalog');
        }
        this.host = config.host;
        this.#boot = config.bootBridge();
        this.#bridge = this.#boot.bridge;
        this.#renderer = config.renderer ?? null;
        this.#fixtures = config.fixtures;
        this.#activeFixtureId = config.initialFixtureId ?? config.fixtures[0].id;
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
        // Boot the engine if a bridge is present (idempotent seed; mirrors the smoke boot).
        if (this.#bridge) {
            try {
                this.#bridge.initializeEngine({ seed: 1 });
            }
            catch (cause) {
                this.#captureDegradation(cause);
            }
        }
    }
    /** The currently selected fixture. */
    get activeFixture() {
        return this.#fixtures.find((f) => f.id === this.#activeFixtureId);
    }
    /** Runtime-selectable fixture switch. Clears prior load state; does not auto-load. */
    selectFixture(id) {
        if (!this.#fixtures.some((f) => f.id === id)) {
            throw new Error(`unknown fixture '${id}'`);
        }
        this.#activeFixtureId = id;
        this.#composition = null;
        this.#worldLoaded = false;
        this.#renderApplied = false;
        this.#renderSource = 'none';
        this.#renderDetail = 'fixture changed; reload to project';
    }
    /**
     * Load the active fixture through the facade. Captures the composition status for the
     * readout; a native-gap failure downgrades the runtime to `degraded` instead of
     * pretending the world loaded.
     */
    loadActiveFixture() {
        if (!this.#bridge) {
            this.#worldLoaded = false;
            this.#composition = null;
            return this.worldStatus();
        }
        try {
            const status = this.#bridge.loadWorldBundle(this.activeFixture.request);
            this.#composition = status;
            this.#worldLoaded = status.loadedWorld === this.activeFixture.request.sceneId && !status.blocksLoad;
        }
        catch (cause) {
            this.#worldLoaded = false;
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
    projectAuthority() {
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
        }
        catch (cause) {
            this.#renderApplied = false;
            this.#renderDetail = cause instanceof Error ? cause.message : String(cause);
            this.#captureDegradation(cause);
        }
        return this.rendererStatus();
    }
    /** Cast a pointer-built ray against authority and update selection (single path). */
    pick(ray) {
        if (!this.#bridge) {
            this.controller.store.dispatch({ type: 'clearSelection' });
            return { outcome: 'miss', rejection: { reason: 'noHit' } };
        }
        try {
            return pickAndSelect(this.controller.store, bridgePicker(this.#bridge), ray);
        }
        catch (cause) {
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
    applyControl(id, value) {
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
    // ── Read models ──────────────────────────────────────────────────────────────
    /** The accessible material palette for the active fixture's catalog materials. */
    palette() {
        return materialPalette(this.activeFixture.materials);
    }
    runtimeStatus() {
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
    worldStatus() {
        const fixture = this.activeFixture;
        return {
            fixtureId: fixture.id,
            fixtureLabel: fixture.label,
            loaded: this.#worldLoaded,
            composition: this.#composition,
            detail: this.#worldLoaded
                ? `loaded world ${this.#composition?.loadedWorld ?? '?'}`
                : this.#bridge
                    ? 'fixture not loaded'
                    : 'no bridge to load fixture',
        };
    }
    rendererStatus() {
        return {
            present: this.#renderer !== null,
            applied: this.#renderApplied,
            sceneNodes: this.#renderer?.sceneNodeCount ?? 0,
            source: this.#renderSource,
            detail: this.#renderDetail,
        };
    }
    /** The devtools inspection of the ONE editor context. */
    editorInspection() {
        return inspectEditor(this.controller.store.getState());
    }
    /** The accessible control set for the ONE editor context. */
    controls() {
        return buildEditorControls(this.controller.store.getState(), this.palette());
    }
    fixtureListing() {
        return this.#fixtures.map((f) => ({ id: f.id, label: f.label, active: f.id === this.#activeFixtureId }));
    }
    /** The full snapshot for an agent/human dashboard or a CI-safe launch report. */
    readout() {
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
    #captureDegradation(cause) {
        if (cause instanceof RuntimeBridgeError && isNativeGap(cause)) {
            this.#degradation = cause;
        }
        else if (cause instanceof RuntimeBridgeError) {
            // A non-native classified error (invalid_input, not_initialized…) is still a
            // visible degradation of this run, not a silent pass.
            this.#degradation = cause;
        }
        else {
            this.#degradation = new RuntimeBridgeError('internal', cause instanceof Error ? cause.message : String(cause));
        }
    }
}
/** Render a {@link ShellReadout} as a stable, multi-line text report (CLI/launch). */
export function formatReadout(readout) {
    const lines = [];
    lines.push(`asha-shell: host=${readout.host.name} accessibility=${readout.host.accessibility}`);
    lines.push(`runtime: ${readout.runtime.availability} (mode=${readout.runtime.mode} intent=${readout.runtime.intent} ` +
        `nativeAvailable=${readout.runtime.nativeAvailable}) — ${readout.runtime.detail}`);
    lines.push(`world: ${readout.world.fixtureId} loaded=${readout.world.loaded} — ${readout.world.detail}`);
    lines.push(`renderer: present=${readout.renderer.present} applied=${readout.renderer.applied} ` +
        `nodes=${readout.renderer.sceneNodes} source=${readout.renderer.source} — ${readout.renderer.detail}`);
    lines.push(`editor: tool=${readout.editor.tool} material=${readout.editor.material} ` +
        `brushShape=${readout.editor.brushShape} affectedCells=${readout.editor.affectedCells}`);
    lines.push(`fixtures: ${readout.fixtures.map((f) => `${f.id}${f.active ? '*' : ''}`).join(' ')}`);
    lines.push(`controls: ${readout.controls.map((c) => `${c.id}[${c.role}]${c.disabled ? ':disabled' : ''}`).join(' ')}`);
    if (readout.lastCommandResult) {
        lines.push(`lastCommand: accepted=${readout.lastCommandResult.accepted} rejected=${readout.lastCommandResult.rejected}`);
    }
    return lines.join('\n') + '\n';
}
//# sourceMappingURL=shell.js.map