// @asha/smoke — the canonical developer smoke harness (#2395/#2396/#2397/#2424).
//
// One entrypoint boots the ASHA runtime facade against an abstract fixture world,
// probes capability status, drives the load → projection → render path, and
// proposes a validated edit through the authority command path — emitting a single
// structured `SmokeResult` (see result.ts).
//
// Two intents share the flow (task #2424):
//   - `reference`: the deterministic mock/dev smoke. Proves the renderer upload
//     path by applying a local fixture frame directly. Always green offline.
//   - `authority`: the real loop. Reads render diffs *through the facade*
//     (`bridge.readRenderDiffs`) and submits contract-shaped commands. A missing
//     native capability is classified honestly, never downgraded to mock success.
import { createMockRuntimeBridge, createNativeRuntimeBridge, frameCursor, RuntimeBridgeError, } from '@asha/runtime-bridge';
import { ThreeRenderer } from '@asha/renderer-three';
import { FIXTURE_WORLD, fixtureCommandBatch, fixtureRenderFrame, fixtureWorldHash } from './fixtures.js';
export const SMOKE_COMMAND = 'pnpm --filter @asha/smoke dev:asha-smoke';
export const AUTHORITY_SMOKE_COMMAND = 'ASHA_SMOKE_MODE=authority pnpm --filter @asha/smoke dev:asha-smoke';
/**
 * Default boot: the canonical deterministic reference smoke on the mock facade,
 * while *probing* native availability for an honest capability readout. The native
 * addon today is a partial prototype (only initialize/step are wired), so the
 * reference smoke does not depend on it.
 */
export function defaultBootBridge() {
    return {
        bridge: createMockRuntimeBridge(),
        mode: 'mock',
        intent: 'reference',
        nativeAvailable: probeNativeAvailable(),
    };
}
/**
 * Authority boot: attempt the real native path. If the native addon is not
 * loadable, the boot fails *closed* with a classified error — the harness reports
 * an honest failure rather than silently downgrading to the mock.
 */
export function authorityBootBridge() {
    try {
        const bridge = createNativeRuntimeBridge();
        return { bridge, mode: 'native', intent: 'authority', nativeAvailable: true };
    }
    catch (cause) {
        const bootError = cause instanceof RuntimeBridgeError
            ? cause
            : new RuntimeBridgeError('native_unavailable', describeError(cause));
        return { bridge: null, mode: 'native', intent: 'authority', nativeAvailable: false, bootError };
    }
}
/** Pick a boot strategy from an explicit smoke mode (used by the CLI). */
export function bootForMode(mode) {
    return mode === 'authority' ? authorityBootBridge() : defaultBootBridge();
}
/** Whether the native addon is loadable, without depending on it for the run. */
function probeNativeAvailable() {
    try {
        createNativeRuntimeBridge();
        return true;
    }
    catch {
        // Any load failure (missing build, ABI mismatch) means it is unusable here.
        return false;
    }
}
/** Run the full smoke flow and return a deterministic structured result. */
export function runSmoke(options = {}) {
    const boot = (options.bootBridge ?? defaultBootBridge)();
    // ── Stage 1: boot + capability readout (#2395) ──
    if (boot.bridge === null) {
        // Boot failed closed (e.g. authority intent with no native addon). Honest,
        // classified, never a blank mock success.
        return bootFailedResult(boot);
    }
    const bridge = boot.bridge;
    const authority = boot.intent === 'authority';
    const stages = [];
    const failures = [];
    bridge.initializeEngine({ seed: 1 });
    stages.push({
        name: 'boot',
        ok: true,
        detail: `runtime facade up in ${boot.mode} mode, ${boot.intent} intent (nativeAvailable=${boot.nativeAvailable})`,
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
    // Authority intent reads diffs THROUGH the facade; the reference smoke applies a
    // local fixture frame directly (its explicit, mock-only job).
    let renderApplied = false;
    let sceneNodes = 0;
    const renderSource = authority ? 'bridge.readRenderDiffs' : 'fixtureRenderFrame';
    try {
        const frame = authority ? bridge.readRenderDiffs(frameCursor(0)) : fixtureRenderFrame();
        if (authority && frame.ops.length === 0) {
            // A facade that returns no diffs cannot prove projection — fail closed
            // rather than reporting an empty render as success.
            throw new RuntimeBridgeError('operation_unimplemented', 'readRenderDiffs returned no ops; authority projection is not wired');
        }
        const renderer = new ThreeRenderer();
        renderer.applyFrame(frame);
        sceneNodes = renderer.scene.getObjectByName('scene')?.children.length ?? 0;
        renderApplied = sceneNodes > 0;
        stages.push({
            name: 'render',
            ok: renderApplied,
            detail: `applied frame from ${renderSource}; scene nodes=${sceneNodes}`,
        });
        if (!renderApplied) {
            failures.push({
                category: 'projection_failure',
                subsystem: `renderer-three.applyFrame(${renderSource})`,
                message: 'render frame produced no scene nodes',
                nextStep: 'verify the render diff source and the renderer create path',
            });
        }
    }
    catch (cause) {
        failures.push(classifyBridgeFailure(`render(${renderSource})`, 'projection_failure', cause));
        stages.push({ name: 'render', ok: false, detail: describeError(cause) });
    }
    // ── Stage 4: contract-shaped edit + save/status through authority (#2397) ──
    let editOk = false;
    try {
        const batch = fixtureCommandBatch();
        const result = bridge.submitCommands(batch);
        const rejectedProbe = probeRejectedEdit();
        const save = bridge.saveCurrentWorld();
        // Authority intent additionally reads composition status back through the facade.
        const statusOk = authority ? !bridge.getCompositionStatus().blocksLoad : true;
        editOk =
            result.accepted === batch.commands.length &&
                result.rejected === 0 &&
                save.artifactsWritten > 0 &&
                statusOk;
        stages.push({
            name: 'edit-save',
            ok: editOk,
            detail: `proposed ${batch.commands.length} contract command(s) → accepted=${result.accepted} ` +
                `rejected=${result.rejected}; rejected-path visible=${rejectedProbe}; ` +
                `saved artifacts=${save.artifactsWritten}` +
                (authority ? `; composition status read=ok` : ''),
        });
        if (!editOk) {
            failures.push({
                category: 'ui_command_rejected',
                subsystem: 'runtime-bridge.submitCommands',
                message: 'proposed edit was not accepted, save wrote no artifact, or status blocked',
                nextStep: 'inspect command validation and the save/compaction path',
            });
        }
    }
    catch (cause) {
        failures.push(classifyBridgeFailure('runtime-bridge.submitCommands', 'ui_command_rejected', cause));
        stages.push({ name: 'edit-save', ok: false, detail: describeError(cause) });
    }
    const ok = failures.length === 0;
    const outcome = !ok
        ? 'failed'
        : authority
            ? 'native_authority_passed'
            : 'mock_reference_passed';
    return {
        ok,
        command: authority ? AUTHORITY_SMOKE_COMMAND : SMOKE_COMMAND,
        runtimeMode: boot.mode,
        smokeMode: boot.intent,
        outcome,
        nativeAvailable: boot.nativeAvailable,
        capabilities: {
            runtimeBridge: authority ? 'ok' : 'mock',
            worldLoad: worldLoadOk ? (authority ? 'ok' : 'mock') : 'unavailable',
            renderer: renderApplied ? 'ok' : 'unavailable',
            projection: renderApplied ? (authority ? 'ok' : 'mock') : 'unavailable',
        },
        fixture: { id: FIXTURE_WORLD.sceneId, worldHash: fixtureWorldHash(FIXTURE_WORLD) },
        diagnostics,
        render: { applied: renderApplied, sceneNodes },
        stages,
        failures,
    };
}
/** Build an honest, classified result for a boot that failed closed. */
function bootFailedResult(boot) {
    const error = boot.bootError ?? new RuntimeBridgeError('native_unavailable', 'bridge boot failed');
    const failure = {
        category: error.kind === 'native_unavailable' ? 'missing_native_bridge' : 'internal',
        subsystem: 'smoke.boot',
        message: error.message,
        nextStep: boot.intent === 'authority'
            ? 'build the napi addon (harness/ci/check-native.sh) or run the reference smoke'
            : 'inspect the classified boot error',
    };
    return {
        ok: false,
        command: boot.intent === 'authority' ? AUTHORITY_SMOKE_COMMAND : SMOKE_COMMAND,
        runtimeMode: boot.mode,
        smokeMode: boot.intent,
        outcome: 'failed',
        nativeAvailable: boot.nativeAvailable,
        capabilities: {
            runtimeBridge: 'unavailable',
            worldLoad: 'unavailable',
            renderer: 'unavailable',
            projection: 'unavailable',
        },
        fixture: { id: FIXTURE_WORLD.sceneId, worldHash: fixtureWorldHash(FIXTURE_WORLD) },
        diagnostics: { total: 0, fatal: 0, blocksLoad: false },
        render: { applied: false, sceneNodes: 0 },
        stages: [{ name: 'boot', ok: false, detail: error.message }],
        failures: [failure],
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
        // A real generated VoxelCommand batch (not a `{ kind }` placeholder) against an
        // uninitialized facade must fail closed with a classified not_initialized error.
        fresh.submitCommands(fixtureCommandBatch());
        return false;
    }
    catch (cause) {
        return cause instanceof RuntimeBridgeError && cause.kind === 'not_initialized';
    }
}
function classifyBridgeFailure(subsystem, fallback, cause) {
    if (cause instanceof RuntimeBridgeError) {
        // A loaded native facade that fail-closes (operation_unimplemented) is a
        // missing native capability, classified like an unavailable addon — never a
        // silent downgrade.
        const nativeGap = cause.kind === 'native_unavailable' || cause.kind === 'operation_unimplemented';
        const category = nativeGap ? 'missing_native_bridge' : fallback;
        return {
            category,
            subsystem,
            message: cause.message,
            nextStep: category === 'missing_native_bridge'
                ? 'wire the native operation or run the reference smoke'
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