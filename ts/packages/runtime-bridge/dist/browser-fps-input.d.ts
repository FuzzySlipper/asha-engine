import type { CameraHandle, FirstPersonCameraInputEnvelope } from '@asha/contracts';
export type BrowserFpsKeyCode = 'KeyW' | 'KeyA' | 'KeyS' | 'KeyD' | 'Escape';
export interface BrowserFpsKeyboardInput {
    readonly code: string;
    readonly repeat?: boolean;
    preventDefault?(): void;
}
export interface BrowserFpsMouseMoveInput {
    readonly movementX: number;
    readonly movementY: number;
}
export interface BrowserFpsPointerButtonInput {
    readonly button: number;
    preventDefault?(): void;
}
export type BrowserFpsPointerLockIntent = {
    readonly kind: 'request_pointer_lock';
    readonly reason: 'primary_button' | 'programmatic';
} | {
    readonly kind: 'release_pointer_lock';
    readonly reason: 'escape_key' | 'programmatic';
};
export interface BrowserFpsUnsupportedIntent {
    readonly kind: 'unsupported_primary_fire';
    readonly pressed: boolean;
    readonly triggered: boolean;
    readonly reason: 'no_public_runtime_action_protocol';
}
export interface BrowserFpsInputReadout {
    readonly pointerLocked: boolean;
    readonly releaseRequestedByEscape: boolean;
    readonly pressedKeys: readonly BrowserFpsKeyCode[];
    readonly moveForward: number;
    readonly moveRight: number;
    readonly pendingMouseDelta: readonly [number, number];
    readonly primaryFirePressed: boolean;
    readonly primaryFireTriggered: boolean;
}
export type BrowserFpsRuntimeCommand = {
    readonly kind: 'runtime.apply_first_person_camera_input';
    readonly envelope: FirstPersonCameraInputEnvelope;
};
export interface BrowserFpsCommandFrame {
    readonly tick: number;
    readonly runtimeCommand: BrowserFpsRuntimeCommand;
    readonly pointerLockIntents: readonly BrowserFpsPointerLockIntent[];
    readonly unsupportedIntents: readonly BrowserFpsUnsupportedIntent[];
    readonly readout: BrowserFpsInputReadout;
}
export interface BrowserFpsInputCollectorOptions {
    readonly camera: CameraHandle;
    readonly moveSpeedUnitsPerSecond: number;
    readonly mouseSensitivityDegreesPerPixel: number;
    readonly pointerLocked?: boolean;
}
export interface BrowserFpsDrainInput {
    readonly tick: number;
    readonly dtSeconds: number;
}
export declare class BrowserFpsInputCollector {
    #private;
    constructor(options: BrowserFpsInputCollectorOptions);
    setPointerLockActive(active: boolean): BrowserFpsInputReadout;
    requestPointerLock(): readonly BrowserFpsPointerLockIntent[];
    releasePointerLock(): readonly BrowserFpsPointerLockIntent[];
    handleKeyDown(event: BrowserFpsKeyboardInput): readonly BrowserFpsPointerLockIntent[];
    handleKeyUp(event: BrowserFpsKeyboardInput): void;
    handleMouseMove(event: BrowserFpsMouseMoveInput): void;
    handlePointerDown(event: BrowserFpsPointerButtonInput): readonly BrowserFpsPointerLockIntent[];
    handlePointerUp(event: BrowserFpsPointerButtonInput): void;
    drainFrame(input: BrowserFpsDrainInput): BrowserFpsCommandFrame;
    readout(): BrowserFpsInputReadout;
}
//# sourceMappingURL=browser-fps-input.d.ts.map