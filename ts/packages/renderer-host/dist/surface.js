// Backend-neutral browser render surface host.
import { createGeneratedTunnelRoomFrame, RenderProjection, } from '@asha/render-projection';
import { createAshaRendererBrowserSurfaceFrame as createBackendBrowserSurfaceFrame, mountAshaRendererBrowserSurface as mountThreeBackedBrowserSurface, } from '@asha/renderer-three/backend';
import { BrowserFpsResolvedActionConsumer, BrowserInputHost, } from '@asha/runtime-bridge';
import { animationPlaybackReadout, loadRendererAnimatedMeshSource, } from './animated-mesh-host.js';
export const ASHA_RENDERER_HOST_COMPATIBILITY_VERSION = 'renderer-host.v0';
const THREE_BACKEND_DIAGNOSTICS = {
    family: 'threejs',
    implementation: 'engine-owned-renderer-backend',
    publicContract: 'asha-renderer-surface.v0',
};
export function createAshaRendererSurfaceProjection(frame) {
    const projection = new RenderProjection();
    const instructions = projection.applyFrame(frame);
    return {
        instructions,
        snapshot: projection.snapshot(),
    };
}
export function createAshaRendererDefaultSurfaceFrame() {
    return createBackendBrowserSurfaceFrame();
}
export function createAshaRendererGeneratedTunnelRoomSurfaceFrame(input) {
    return createGeneratedTunnelRoomFrame({
        ...(input.enemy === undefined ? {} : { enemy: input.enemy }),
        ...(input.materials === undefined ? {} : { materials: input.materials }),
        tunnel: input.tunnel,
    });
}
export function surfaceTargetProjectionFromRenderTarget(target, options = {}) {
    return {
        label: target.renderLabel,
        ...(options.lastEvent === undefined ? {} : { lastEvent: options.lastEvent }),
        position: target.position,
        ...(target.scale === null ? {} : { scale: target.scale }),
        visible: target.visible,
    };
}
export function mountAshaRendererSurface(canvas, options = {}) {
    return mountPreparedAshaRendererSurface(canvas, options);
}
export async function mountAshaRendererAnimatedMeshSurface(canvas, options) {
    const source = await loadRendererAnimatedMeshSource(options.animatedMeshManifest, options.resolveAnimatedMeshResource);
    return mountPreparedAshaRendererSurface(canvas, options, source);
}
function mountPreparedAshaRendererSurface(canvas, options, animatedMeshSource) {
    const frame = options.frame ?? createAshaRendererDefaultSurfaceFrame();
    const projection = new RenderProjection();
    projection.applyFrame(frame);
    const controls = createAshaRendererSurfaceFirstPersonControls(canvas, options.controls);
    const interactions = createAshaRendererSurfaceInteractionController(frame);
    const backendSurface = mountThreeBackedBrowserSurface(canvas, {
        autoStart: false,
        ...(animatedMeshSource === undefined ? {} : { animatedMeshSource }),
        camera: { initialPose: controls.cameraPose() },
        ...(options.clearColor === undefined ? {} : { clearColor: options.clearColor }),
        ...(options.pixelRatio === undefined ? {} : { pixelRatio: options.pixelRatio }),
        frame,
    });
    let animationFrame = null;
    let lastRenderTimeMs = null;
    const renderOnce = (timeMs = globalThis.performance?.now() ?? 0) => {
        const deltaSeconds = lastRenderTimeMs === null
            ? 0
            : Math.min(0.05, Math.max(0, (timeMs - lastRenderTimeMs) / 1000));
        lastRenderTimeMs = timeMs;
        controls.update(deltaSeconds);
        const camera = controls.cameraSnapshot();
        backendSurface.setCameraPose(camera.pose, camera.basis ?? undefined);
        backendSurface.renderOnce(timeMs);
    };
    const tick = (timeMs) => {
        renderOnce(timeMs);
        animationFrame = globalThis.requestAnimationFrame(tick);
    };
    const start = () => {
        if (animationFrame !== null) {
            return;
        }
        animationFrame = globalThis.requestAnimationFrame(tick);
    };
    const stop = () => {
        if (animationFrame === null) {
            return;
        }
        globalThis.cancelAnimationFrame(animationFrame);
        animationFrame = null;
    };
    const reset = () => {
        controls.resetCamera();
        interactions.reset((projectionUpdate) => backendSurface.projectObjectProjection(projectionUpdate));
        lastRenderTimeMs = null;
        renderOnce(0);
    };
    const applyFrame = (nextFrame) => {
        try {
            backendSurface.applyFrame(nextFrame);
            projection.applyFrame(nextFrame);
            return { applied: true, diagnostics: [] };
        }
        catch (cause) {
            return {
                applied: false,
                diagnostics: [{
                        code: 'animated_mesh_frame_rejected',
                        message: cause instanceof Error ? cause.message : String(cause),
                        asset: null,
                        handle: null,
                    }],
            };
        }
    };
    const animationProjection = {
        kind: 'asha_renderer_animated_mesh_projection.v0',
        applyFrame,
        // The mounted browser surface already advances mixer time in its render
        // loop. AshaAnimationHost still calls this port after updating weights, but
        // must not advance the same renderer a second time.
        advance: () => ({ applied: true, diagnostics: [] }),
        playback: (handle) => animationPlaybackReadout(handle, backendSurface.renderer.animatedMeshPlayback(handle)),
        snapshot: () => backendSurface.renderer.snapshot(),
        hasAnimationTarget: (handle) => backendSurface.renderer.has(handle),
        setAnimationControllerWeights: (handle, clips) => {
            backendSurface.renderer.setAnimationControllerWeights(handle, clips);
        },
        hasAnimationClips: (handle, clipIds) => backendSurface.renderer.hasAnimationControllerClips(handle, clipIds),
        clearAnimationControllerWeights: (handle) => {
            backendSurface.renderer.clearAnimationControllerWeights(handle);
        },
    };
    renderOnce(0);
    if (options.autoStart !== false) {
        start();
    }
    return {
        kind: 'asha_renderer_surface.v0',
        backend: THREE_BACKEND_DIAGNOSTICS,
        canvas,
        frame,
        animationProjection,
        animatedMeshPlayback: (handle) => animationPlaybackReadout(handle, backendSurface.animatedMeshPlayback(handle)),
        applyFrame,
        projectionSnapshot: () => projection.snapshot(),
        cameraPose: () => controls.cameraPose(),
        firePrimary: () => interactions.firePrimary((labels) => backendSurface.pickCenterObject({ labels }), (projectionUpdate) => backendSurface.projectObjectProjection(projectionUpdate)),
        interactionState: () => interactions.state(),
        lockPointer: () => controls.lockPointer(),
        movementState: () => controls.movementState(),
        pointerLocked: () => controls.pointerLocked(),
        inputReadout: () => controls.inputReadout(),
        projectRenderTargetProjection: (target, targetProjectionOptions) => interactions.projectRenderTargetProjection(surfaceTargetProjectionFromRenderTarget(target, targetProjectionOptions), (projectionUpdate) => backendSurface.projectObjectProjection(projectionUpdate)),
        projectTargetProjection: (targetProjection) => interactions.projectTargetProjection(targetProjection, (projectionUpdate) => backendSurface.projectObjectProjection(projectionUpdate)),
        reset,
        snapshot: () => backendSurface.snapshot(),
        renderOnce,
        start,
        stop,
        dispose: () => {
            stop();
            controls.dispose();
            backendSurface.dispose();
        },
    };
}
function createAshaRendererSurfaceFirstPersonControls(canvas, options) {
    const enabled = options?.enabled !== false && options?.inputSession !== undefined;
    const ownerDocument = canvas.ownerDocument;
    const moveSpeed = options?.moveSpeed ?? 5.8;
    const mouseSensitivity = options?.mouseSensitivity ?? 0.0021;
    const eyeHeight = options?.eyeHeight ?? 1.62;
    const initialPosition = options?.initialPosition ?? [0, eyeHeight, 8];
    const movementAuthority = options?.movementAuthority;
    const actionConsumer = new BrowserFpsResolvedActionConsumer();
    const inputHost = options?.inputSession === undefined
        ? null
        : new BrowserInputHost({
            session: options.inputSession,
            initialContexts: options.initialInputContexts ?? ['gameplay'],
            consumers: {
                'gameplay.move.forward': 'renderer.fpsCamera',
                'gameplay.move.backward': 'renderer.fpsCamera',
                'gameplay.move.left': 'renderer.fpsCamera',
                'gameplay.move.right': 'renderer.fpsCamera',
                'gameplay.look': 'renderer.fpsCamera',
                'gameplay.primaryFire': 'runtime.gameplay',
                'menu.open': 'shell.menu',
                'menu.close': 'shell.menu',
            },
            onResolvedAction: (action) => actionConsumer.accept(action),
            onContextChanged: () => actionConsumer.reset(),
        });
    let authorityBasis = null;
    let controlTick = 0;
    let pitchRadians = degreesToRadians(options?.initialPitchDegrees ?? 0);
    let position = [initialPosition[0], initialPosition[1], initialPosition[2]];
    let yawRadians = degreesToRadians(options?.initialYawDegrees ?? 0);
    let lastMovementState = {
        authority: movementAuthority === undefined ? 'free_camera' : 'external_collision',
        blockedAxes: [],
        collided: false,
        movementHash: null,
    };
    if (canvas.tabIndex < 0) {
        canvas.tabIndex = 0;
    }
    canvas.style.touchAction = 'none';
    const focusCanvas = () => {
        canvas.focus({ preventScroll: true });
    };
    const requestLock = (event) => {
        if (!enabled) {
            return;
        }
        event?.preventDefault();
        focusCanvas();
        if (ownerDocument.pointerLockElement !== canvas) {
            void canvas.requestPointerLock();
        }
    };
    const detachInput = inputHost?.attachDom({
        pointerTarget: canvas,
        keyboardTarget: ownerDocument,
        acceptsKeyboard: () => controlsHaveKeyboardFocus(canvas, inputHost.readout().pointerLocked),
        onPointerLockIntent: (intent, event) => {
            if (intent.kind === 'requestPointerLock')
                requestLock(event);
            else
                ownerDocument.exitPointerLock();
        },
    });
    const cameraPose = () => ({
        position: [round4(position[0]), round4(position[1]), round4(position[2])],
        pitchDegrees: round2(radiansToDegrees(pitchRadians)),
        yawDegrees: round2(radiansToDegrees(yawRadians)),
    });
    const resetCamera = () => {
        actionConsumer.reset();
        authorityBasis = null;
        controlTick = 0;
        pitchRadians = degreesToRadians(options?.initialPitchDegrees ?? 0);
        position = [initialPosition[0], initialPosition[1], initialPosition[2]];
        yawRadians = degreesToRadians(options?.initialYawDegrees ?? 0);
        lastMovementState = {
            authority: movementAuthority === undefined ? 'free_camera' : 'external_collision',
            blockedAxes: [],
            collided: false,
            movementHash: null,
        };
    };
    const onPointerLockChange = () => {
        const pointerLocked = ownerDocument.pointerLockElement === canvas;
        inputHost?.setPointerLockActive(pointerLocked);
        if (!pointerLocked) {
            actionConsumer.reset();
        }
    };
    const update = (deltaSeconds) => {
        if (!enabled || inputHost === null || !controlsHaveKeyboardFocus(canvas, inputHost.readout().pointerLocked)) {
            return;
        }
        const frame = actionConsumer.drain();
        const safeDeltaSeconds = Math.max(0, deltaSeconds);
        const forward = frame.moveForward;
        const strafe = frame.moveRight;
        const yawDeltaDegrees = frame.yawDeltaPixels * radiansToDegrees(mouseSensitivity);
        const pitchDeltaDegrees = -frame.pitchDeltaPixels * radiansToDegrees(mouseSensitivity);
        const hasLookDelta = yawDeltaDegrees !== 0 || pitchDeltaDegrees !== 0;
        if (forward === 0 && strafe === 0 && !hasLookDelta) {
            return;
        }
        if (movementAuthority !== undefined) {
            controlTick += 1;
            const authorityResult = movementAuthority({
                dtSeconds: safeDeltaSeconds,
                moveForward: forward,
                moveRight: strafe,
                moveSpeedUnitsPerSecond: moveSpeed,
                moveUp: 0,
                pitchDeltaDegrees,
                poseBefore: cameraPose(),
                tick: controlTick,
                yawDeltaDegrees,
            });
            position = authorityResult.pose.position;
            yawRadians = degreesToRadians(authorityResult.pose.yawDegrees);
            pitchRadians = degreesToRadians(authorityResult.pose.pitchDegrees);
            authorityBasis = authorityResult.basis ?? null;
            lastMovementState = {
                authority: 'external_collision',
                blockedAxes: authorityResult.blockedAxes ?? [],
                collided: authorityResult.collided ?? false,
                movementHash: authorityResult.movementHash ?? null,
            };
            return;
        }
        yawRadians += degreesToRadians(yawDeltaDegrees);
        pitchRadians = clamp(pitchRadians + degreesToRadians(pitchDeltaDegrees), degreesToRadians(-85), degreesToRadians(85));
        authorityBasis = null;
        if (safeDeltaSeconds <= 0) {
            return;
        }
        const movement = calculateCameraRelativeMovement(yawRadians, forward, strafe);
        if (movement === null) {
            return;
        }
        const step = moveSpeed * safeDeltaSeconds;
        position = [position[0] + movement[0] * step, eyeHeight, position[2] + movement[2] * step];
        lastMovementState = {
            authority: 'free_camera',
            blockedAxes: [],
            collided: false,
            movementHash: null,
        };
    };
    const cameraSnapshot = () => ({
        ...(authorityBasis === null ? {} : { basis: authorityBasis }),
        pose: cameraPose(),
    });
    const dispose = () => {
        ownerDocument.removeEventListener('pointerlockchange', onPointerLockChange);
        detachInput?.();
        if (ownerDocument.pointerLockElement === canvas) {
            ownerDocument.exitPointerLock();
        }
    };
    ownerDocument.addEventListener('pointerlockchange', onPointerLockChange);
    return {
        cameraPose,
        cameraSnapshot,
        dispose,
        lockPointer: () => requestLock(),
        movementState: () => lastMovementState,
        pointerLocked: () => inputHost?.readout().pointerLocked ?? false,
        inputReadout: () => inputHost?.readout() ?? null,
        resetCamera,
        update,
    };
}
function createAshaRendererSurfaceInteractionController(frame) {
    const targets = collectAshaRendererSurfaceTargets(frame);
    let hits = 0;
    let lastEvent = 'Ready';
    let shotsFired = 0;
    const state = () => ({
        hits,
        lastEvent,
        remainingTargets: targets.filter((target) => target.health > 0).length,
        shotsFired,
        totalTargets: targets.length,
    });
    const firePrimary = (pickCenterObject, projectObject) => {
        shotsFired += 1;
        const liveTargets = targets.filter((target) => target.health > 0);
        const picked = pickCenterObject(liveTargets.map((target) => target.label));
        if (picked === null) {
            lastEvent = 'Miss';
            return missFireResult(state().remainingTargets, shotsFired);
        }
        const target = liveTargets.find((candidate) => candidate.label === picked.label);
        if (target === undefined) {
            lastEvent = 'Miss';
            return missFireResult(state().remainingTargets, shotsFired);
        }
        target.health -= 1;
        hits += 1;
        if (target.health <= 0) {
            lastEvent = `Destroyed ${displayTargetLabel(target.label)}`;
            projectObject({ label: target.label, visible: false });
        }
        else {
            lastEvent = `Hit ${displayTargetLabel(target.label)}`;
            projectObject({ color: [1, 0.28, 0.18, 1], label: target.label, visible: true });
        }
        return {
            distance: picked.distance,
            hit: true,
            label: displayTargetLabel(target.label),
            remainingTargets: state().remainingTargets,
            shotsFired,
            targetHealth: Math.max(0, target.health),
        };
    };
    const reset = (projectObject) => {
        hits = 0;
        lastEvent = 'Reset';
        shotsFired = 0;
        for (const target of targets) {
            target.health = target.maxHealth;
            projectObject({ label: target.label, visible: true });
        }
    };
    const projectTargetProjection = (projection, projectObject) => {
        lastEvent = projection.lastEvent ?? lastEvent;
        for (const target of targets) {
            target.health = projection.visible ? target.maxHealth : 0;
            projectObject({
                label: target.label,
                ...(projection.position === undefined ? {} : { position: projection.position }),
                ...(projection.scale === undefined ? {} : { scale: projection.scale }),
                visible: projection.visible,
            });
        }
    };
    const projectRenderTargetProjection = (projection, projectObject) => {
        lastEvent = projection.lastEvent ?? lastEvent;
        const target = targets.find((candidate) => candidate.label === projection.label);
        if (target === undefined) {
            return;
        }
        target.health = projection.visible ? target.maxHealth : 0;
        projectObject({
            label: target.label,
            ...(projection.position === undefined ? {} : { position: projection.position }),
            ...(projection.scale === undefined ? {} : { scale: projection.scale }),
            visible: projection.visible,
        });
    };
    return {
        firePrimary,
        projectRenderTargetProjection,
        projectTargetProjection,
        reset,
        state,
    };
}
function collectAshaRendererSurfaceTargets(frame) {
    const targets = [];
    for (const op of frame.ops) {
        if (op.op !== 'create') {
            continue;
        }
        const label = op.node.metadata.label;
        if (label === null || label === undefined || !isAshaRendererSurfaceTargetLabel(label)) {
            continue;
        }
        targets.push({ health: 2, label, maxHealth: 2 });
    }
    return targets;
}
function isAshaRendererSurfaceTargetLabel(label) {
    return label.includes('generated-tunnel-enemy') || label.startsWith('asha-renderer-random-cube-');
}
function displayTargetLabel(label) {
    return label.replace('asha-renderer-random-cube-', 'cube ');
}
function missFireResult(remainingTargets, shotsFired) {
    return {
        distance: null,
        hit: false,
        label: null,
        remainingTargets,
        shotsFired,
        targetHealth: null,
    };
}
function calculateCameraRelativeMovement(yawRadians, forwardAxis, strafeAxis) {
    const forward = [-Math.sin(yawRadians), 0, -Math.cos(yawRadians)];
    const right = [Math.cos(yawRadians), 0, -Math.sin(yawRadians)];
    const movement = [
        forward[0] * forwardAxis + right[0] * strafeAxis,
        0,
        forward[2] * forwardAxis + right[2] * strafeAxis,
    ];
    const length = Math.hypot(movement[0], movement[2]);
    if (length === 0) {
        return null;
    }
    return [movement[0] / length, 0, movement[2] / length];
}
function controlsHaveKeyboardFocus(canvas, pointerLocked) {
    return pointerLocked || canvas.ownerDocument.activeElement === canvas;
}
function clamp(value, min, max) {
    return Math.min(max, Math.max(min, value));
}
function degreesToRadians(degrees) {
    return (degrees * Math.PI) / 180;
}
function radiansToDegrees(radians) {
    return (radians * 180) / Math.PI;
}
function round2(value) {
    return Number(value.toFixed(2));
}
function round4(value) {
    return Number(value.toFixed(4));
}
//# sourceMappingURL=surface.js.map