// @asha/smoke — the canonical developer smoke harness (#2395/#2396/#2397).
//
// One entrypoint boots the ASHA runtime facade against an abstract fixture world,
// probes capability status, drives the real load → projection → render path, and
// proposes a validated edit through the authority command path — emitting a single
// structured `SmokeResult` (see result.ts). It uses the real facade contracts and
// never invents a parallel state path; a missing native/WASM capability is
// classified, never a silent blank success.
import { createMockRuntimeBridge, createNativeRuntimeBridge, RuntimeBridgeError, } from '@asha/runtime-bridge';
import { ThreeRenderer } from '@asha/renderer-three';
import { FIXTURE_WORLD, fixtureRenderFrame, fixtureWorldHash } from './fixtures.js';
export const SMOKE_COMMAND = 'pnpm --filter @asha/smoke dev:asha-smoke';
/**
 * Default boot: run the canonical smoke on the fully-wired mock facade (the
 * deterministic reference), while *probing* native availability for the capability
 * readout. The native addon today is a partial prototype (only initialize/step are
 * wired), so the canonical dev smoke does not depend on it; native mode is opt-in
 * by injecting a `bootBridge`. Reporting `nativeAvailable` keeps the readout honest.
 */
export function defaultBootBridge() {
    return {
        bridge: createMockRuntimeBridge(),
        mode: 'mock',
        nativeAvailable: probeNativeAvailable(),
    };
}
/** Whether the native addon is loadable, without depending on it for the run. */
function probeNativeAvailable() {
    try {
        createNativeRuntimeBridge();
        return true;
    }
    catch (cause) {
        if (cause instanceof RuntimeBridgeError && cause.kind === 'native_unavailable') {
            return false;
        }
        // A non-load error still means the addon is not usable for the smoke.
        return false;
    }
}
/** Run the full smoke flow and return a deterministic structured result. */
export function runSmoke(options = {}) {
    const boot = (options.bootBridge ?? defaultBootBridge)();
    const stages = [];
    const failures = [];
    // ── Stage 1: boot + capability readout (#2395) ──
    const bridge = boot.bridge;
    bridge.initializeEngine({ seed: 1 });
    stages.push({
        name: 'boot',
        ok: true,
        detail: `runtime facade up in ${boot.mode} mode (nativeAvailable=${boot.nativeAvailable})`,
    });
    // ── Stage 2: load the abstract fixture world through the real facade path ──
    let worldLoadOk = false;
    let diagnostics = { total: 0, fatal: 0, blocksLoad: false };
    try {
        const status = bridge.loadWorldBundle(FIXTURE_WORLD);
        diagnostics = {
            total: status.totalCount,
            fatal: status.fatalCount,
            blocksLoad: status.blocksLoad,
        };
        worldLoadOk = status.loadedWorld === FIXTURE_WORLD.sceneId && !status.blocksLoad;
        stages.push({
            name: 'load',
            ok: worldLoadOk,
            detail: worldLoadOk
                ? `loaded world ${status.loadedWorld}`
                : `load did not settle (loadedWorld=${status.loadedWorld}, blocksLoad=${status.blocksLoad})`,
        });
        if (!worldLoadOk) {
            failures.push({
                category: 'load_failure',
                subsystem: 'runtime-bridge.loadWorldBundle',
                message: `world ${FIXTURE_WORLD.sceneId} did not load cleanly`,
                nextStep: 'inspect composition diagnostics for the failing artifact',
            });
        }
    }
    catch (cause) {
        failures.push(classifyBridgeFailure('runtime-bridge.loadWorldBundle', 'load_failure', cause));
        stages.push({ name: 'load', ok: false, detail: describeError(cause) });
    }
    // ── Stage 3: projection → render through renderer-three (#2396) ──
    let renderApplied = false;
    let sceneNodes = 0;
    try {
        const renderer = new ThreeRenderer();
        renderer.applyFrame(fixtureRenderFrame());
        sceneNodes = renderer.scene.getObjectByName('scene')?.children.length ?? 0;
        renderApplied = sceneNodes > 0;
        stages.push({
            name: 'render',
            ok: renderApplied,
            detail: `applied fixture frame; scene nodes=${sceneNodes}`,
        });
        if (!renderApplied) {
            failures.push({
                category: 'projection_failure',
                subsystem: 'renderer-three.applyFrame',
                message: 'fixture frame produced no scene nodes',
                nextStep: 'verify the fixture render frame and renderer create path',
            });
        }
    }
    catch (cause) {
        failures.push({
            category: 'render_init_failure',
            subsystem: 'renderer-three',
            message: describeError(cause),
            nextStep: 'check renderer-three contract compatibility / GL-free build path',
        });
        stages.push({ name: 'render', ok: false, detail: describeError(cause) });
    }
    // ── Stage 4: proposal-only edit + save through authority (#2397) ──
    let editOk = false;
    try {
        const batch = { commands: [{ kind: 'smoke-edit' }] };
        const result = bridge.submitCommands(batch);
        const rejectedProbe = probeRejectedEdit();
        const save = bridge.saveCurrentWorld();
        editOk = result.accepted === 1 && result.rejected === 0 && save.artifactsWritten > 0;
        stages.push({
            name: 'edit-save',
            ok: editOk,
            detail: `proposed 1 command → accepted=${result.accepted} rejected=${result.rejected}; ` +
                `rejected-path visible=${rejectedProbe}; saved artifacts=${save.artifactsWritten}`,
        });
        if (!editOk) {
            failures.push({
                category: 'ui_command_rejected',
                subsystem: 'runtime-bridge.submitCommands',
                message: 'proposed edit was not accepted or save wrote no artifact',
                nextStep: 'inspect command validation and the save/compaction path',
            });
        }
    }
    catch (cause) {
        failures.push(classifyBridgeFailure('runtime-bridge.submitCommands', 'ui_command_rejected', cause));
        stages.push({ name: 'edit-save', ok: false, detail: describeError(cause) });
    }
    const ok = failures.length === 0;
    return {
        ok,
        command: SMOKE_COMMAND,
        runtimeMode: boot.mode,
        nativeAvailable: boot.nativeAvailable,
        capabilities: {
            runtimeBridge: boot.mode === 'native' ? 'ok' : 'mock',
            worldLoad: worldLoadOk ? (boot.mode === 'native' ? 'ok' : 'mock') : 'unavailable',
            renderer: renderApplied ? 'ok' : 'unavailable',
            projection: renderApplied ? (boot.mode === 'native' ? 'ok' : 'mock') : 'unavailable',
        },
        fixture: { id: FIXTURE_WORLD.sceneId, worldHash: fixtureWorldHash(FIXTURE_WORLD) },
        diagnostics,
        render: { applied: renderApplied, sceneNodes },
        stages,
        failures,
    };
}
/**
 * Prove the rejected command path is observable: a fresh, uninitialized facade
 * rejects a submission with a classified `not_initialized` error. Returns whether
 * the rejection was visible (it always should be).
 */
function probeRejectedEdit() {
    const fresh = createMockRuntimeBridge();
    try {
        fresh.submitCommands({ commands: [{ kind: 'rejected-edit' }] });
        return false;
    }
    catch (cause) {
        return cause instanceof RuntimeBridgeError && cause.kind === 'not_initialized';
    }
}
function classifyBridgeFailure(subsystem, fallback, cause) {
    if (cause instanceof RuntimeBridgeError) {
        const category = cause.kind === 'native_unavailable' ? 'missing_native_bridge' : fallback;
        return {
            category,
            subsystem,
            message: cause.message,
            nextStep: category === 'missing_native_bridge'
                ? 'build the napi addon or run in mock mode'
                : 'inspect the classified runtime-bridge error',
        };
    }
    return {
        category: 'internal',
        subsystem,
        message: describeError(cause),
        nextStep: 'unexpected error — capture the stack and file a bug',
    };
}
function describeError(cause) {
    return cause instanceof Error ? cause.message : String(cause);
}
//# sourceMappingURL=harness.js.map