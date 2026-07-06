// Backend-neutral browser render surface host.
import { RenderProjection, } from '@asha/render-projection';
import { createAshaRendererBrowserSurfaceFrame as createBackendBrowserSurfaceFrame, createAshaRendererGeneratedTunnelRoomSurfaceFrame as createBackendGeneratedTunnelRoomSurfaceFrame, mountAshaRendererBrowserSurface as mountThreeBackedBrowserSurface, } from '@asha/renderer-three/backend';
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
    return createBackendGeneratedTunnelRoomSurfaceFrame({
        ...(input.enemy === undefined ? {} : { enemy: input.enemy }),
        ...(input.materials === undefined ? {} : { materials: input.materials }),
        tunnel: input.tunnel,
    });
}
export function mountAshaRendererSurface(canvas, options = {}) {
    const frame = options.frame ?? createAshaRendererDefaultSurfaceFrame();
    const projection = new RenderProjection();
    projection.applyFrame(frame);
    const controls = createAshaRendererSurfaceFirstPersonControls(canvas, options.controls);
    const interactions = createAshaRendererSurfaceInteractionController(frame);
    const backendSurface = mountThreeBackedBrowserSurface(canvas, {
        autoStart: false,
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
    renderOnce(0);
    if (options.autoStart !== false) {
        start();
    }
    return {
        kind: 'asha_renderer_surface.v0',
        backend: THREE_BACKEND_DIAGNOSTICS,
        canvas,
        frame,
        projectionSnapshot: () => projection.snapshot(),
        cameraPose: () => controls.cameraPose(),
        firePrimary: () => interactions.firePrimary((labels) => backendSurface.pickCenterObject({ labels }), (projectionUpdate) => backendSurface.projectObjectProjection(projectionUpdate)),
        interactionState: () => interactions.state(),
        lockPointer: () => controls.lockPointer(),
        movementState: () => controls.movementState(),
        pointerLocked: () => controls.pointerLocked(),
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
    const enabled = options?.enabled !== false;
    const ownerDocument = canvas.ownerDocument;
    const moveSpeed = options?.moveSpeed ?? 5.8;
    const mouseSensitivity = options?.mouseSensitivity ?? 0.0021;
    const eyeHeight = options?.eyeHeight ?? 1.62;
    const initialPosition = options?.initialPosition ?? [0, eyeHeight, 8];
    const movementAuthority = options?.movementAuthority;
    const pressedKeys = new Set();
    let authorityBasis = null;
    let controlTick = 0;
    let pendingPitchDeltaDegrees = 0;
    let pendingYawDeltaDegrees = 0;
    let pitchRadians = degreesToRadians(options?.initialPitchDegrees ?? 0);
    let pointerLocked = false;
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
    const cameraPose = () => ({
        position: [round4(position[0]), round4(position[1]), round4(position[2])],
        pitchDegrees: round2(radiansToDegrees(pitchRadians)),
        yawDegrees: round2(radiansToDegrees(yawRadians)),
    });
    const resetCamera = () => {
        pressedKeys.clear();
        authorityBasis = null;
        controlTick = 0;
        pendingPitchDeltaDegrees = 0;
        pendingYawDeltaDegrees = 0;
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
        pointerLocked = ownerDocument.pointerLockElement === canvas;
        if (!pointerLocked) {
            pressedKeys.clear();
        }
    };
    const onPointerDown = (event) => {
        if (event.button === 0) {
            requestLock(event);
        }
    };
    const onClick = (event) => {
        requestLock(event);
    };
    const onMouseMove = (event) => {
        if (!pointerLocked) {
            return;
        }
        const yawDeltaRadians = event.movementX * mouseSensitivity;
        const pitchDeltaRadians = -event.movementY * mouseSensitivity;
        yawRadians += yawDeltaRadians;
        pitchRadians = clamp(pitchRadians + pitchDeltaRadians, degreesToRadians(-85), degreesToRadians(85));
        authorityBasis = null;
        pendingYawDeltaDegrees += radiansToDegrees(yawDeltaRadians);
        pendingPitchDeltaDegrees += radiansToDegrees(pitchDeltaRadians);
    };
    const onKeyDown = (event) => {
        if (event.key === 'Escape') {
            ownerDocument.exitPointerLock();
            pressedKeys.clear();
            return;
        }
        if (!controlsHaveKeyboardFocus(canvas, pointerLocked) || !isFirstPersonMovementKey(event.code)) {
            return;
        }
        event.preventDefault();
        pressedKeys.add(event.code);
    };
    const onKeyUp = (event) => {
        if (!isFirstPersonMovementKey(event.code)) {
            return;
        }
        event.preventDefault();
        pressedKeys.delete(event.code);
    };
    const update = (deltaSeconds) => {
        if (!enabled || !controlsHaveKeyboardFocus(canvas, pointerLocked)) {
            return;
        }
        const forward = movementAxis(pressedKeys, 'KeyW', 'ArrowUp', 'KeyS', 'ArrowDown');
        const strafe = movementAxis(pressedKeys, 'KeyD', 'ArrowRight', 'KeyA', 'ArrowLeft');
        const hasLookDelta = pendingYawDeltaDegrees !== 0 || pendingPitchDeltaDegrees !== 0;
        if (forward === 0 && strafe === 0 && !hasLookDelta) {
            return;
        }
        if (movementAuthority !== undefined) {
            controlTick += 1;
            const authorityResult = movementAuthority({
                dtSeconds: Math.max(0, deltaSeconds),
                moveForward: forward,
                moveRight: strafe,
                moveSpeedUnitsPerSecond: moveSpeed,
                moveUp: 0,
                pitchDeltaDegrees: pendingPitchDeltaDegrees,
                poseBefore: cameraPose(),
                tick: controlTick,
                yawDeltaDegrees: pendingYawDeltaDegrees,
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
            pendingPitchDeltaDegrees = 0;
            pendingYawDeltaDegrees = 0;
            return;
        }
        pendingPitchDeltaDegrees = 0;
        pendingYawDeltaDegrees = 0;
        if (deltaSeconds <= 0) {
            return;
        }
        const movement = calculateCameraRelativeMovement(yawRadians, forward, strafe);
        if (movement === null) {
            return;
        }
        const step = moveSpeed * deltaSeconds;
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
        canvas.removeEventListener('pointerdown', onPointerDown);
        canvas.removeEventListener('click', onClick);
        ownerDocument.removeEventListener('pointerlockchange', onPointerLockChange);
        ownerDocument.removeEventListener('mousemove', onMouseMove);
        ownerDocument.removeEventListener('keydown', onKeyDown);
        ownerDocument.removeEventListener('keyup', onKeyUp);
        if (ownerDocument.pointerLockElement === canvas) {
            ownerDocument.exitPointerLock();
        }
    };
    canvas.addEventListener('pointerdown', onPointerDown);
    canvas.addEventListener('click', onClick);
    ownerDocument.addEventListener('pointerlockchange', onPointerLockChange);
    ownerDocument.addEventListener('mousemove', onMouseMove);
    ownerDocument.addEventListener('keydown', onKeyDown);
    ownerDocument.addEventListener('keyup', onKeyUp);
    return {
        cameraPose,
        cameraSnapshot,
        dispose,
        lockPointer: () => requestLock(),
        movementState: () => lastMovementState,
        pointerLocked: () => pointerLocked,
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
    return {
        firePrimary,
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
function isFirstPersonMovementKey(code) {
    return (code === 'KeyW' ||
        code === 'KeyA' ||
        code === 'KeyS' ||
        code === 'KeyD' ||
        code === 'ArrowUp' ||
        code === 'ArrowDown' ||
        code === 'ArrowLeft' ||
        code === 'ArrowRight');
}
function movementAxis(keys, positivePrimary, positiveSecondary, negativePrimary, negativeSecondary) {
    const positive = keys.has(positivePrimary) || keys.has(positiveSecondary) ? 1 : 0;
    const negative = keys.has(negativePrimary) || keys.has(negativeSecondary) ? 1 : 0;
    return positive - negative;
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