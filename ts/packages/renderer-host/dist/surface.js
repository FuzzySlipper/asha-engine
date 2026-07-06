// Backend-neutral browser render surface host.
import { RenderProjection, } from '@asha/render-projection';
import { createAshaRendererBrowserSurfaceFrame as createBackendBrowserSurfaceFrame, createAshaRendererGeneratedTunnelRoomSurfaceFrame as createBackendGeneratedTunnelRoomSurfaceFrame, mountAshaRendererBrowserSurface as mountThreeBackedBrowserSurface, } from '@asha/renderer-three';
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
    const backendSurface = mountThreeBackedBrowserSurface(canvas, {
        ...(options.autoStart === undefined ? {} : { autoStart: options.autoStart }),
        ...(options.clearColor === undefined ? {} : { clearColor: options.clearColor }),
        ...(options.pixelRatio === undefined ? {} : { pixelRatio: options.pixelRatio }),
        ...(options.controls === undefined ? {} : { controls: toBackendControls(options.controls) }),
        frame,
    });
    return {
        kind: 'asha_renderer_surface.v0',
        backend: THREE_BACKEND_DIAGNOSTICS,
        canvas,
        frame,
        projectionSnapshot: () => projection.snapshot(),
        cameraPose: () => backendSurface.cameraPose(),
        firePrimary: () => backendSurface.firePrimary(),
        interactionState: () => backendSurface.interactionState(),
        lockPointer: () => backendSurface.lockPointer(),
        movementState: () => backendSurface.movementState(),
        pointerLocked: () => backendSurface.pointerLocked(),
        projectTargetProjection: (targetProjection) => backendSurface.projectTargetProjection(targetProjection),
        reset: () => backendSurface.reset(),
        snapshot: () => backendSurface.snapshot(),
        renderOnce: (timeMs) => backendSurface.renderOnce(timeMs),
        start: () => backendSurface.start(),
        stop: () => backendSurface.stop(),
        dispose: () => backendSurface.dispose(),
    };
}
function toBackendControls(options) {
    const movementAuthority = options.movementAuthority;
    return {
        ...(options.enabled === undefined ? {} : { enabled: options.enabled }),
        ...(options.eyeHeight === undefined ? {} : { eyeHeight: options.eyeHeight }),
        ...(options.initialPitchDegrees === undefined ? {} : { initialPitchDegrees: options.initialPitchDegrees }),
        ...(options.initialPosition === undefined ? {} : { initialPosition: options.initialPosition }),
        ...(options.initialYawDegrees === undefined ? {} : { initialYawDegrees: options.initialYawDegrees }),
        ...(options.mouseSensitivity === undefined ? {} : { mouseSensitivity: options.mouseSensitivity }),
        ...(options.moveSpeed === undefined ? {} : { moveSpeed: options.moveSpeed }),
        ...(movementAuthority === undefined
            ? {}
            : { movementAuthority: (input) => movementAuthority(input) }),
    };
}
//# sourceMappingURL=surface.js.map