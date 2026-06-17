export type CameraHandle = number & {
    readonly __brand: 'CameraHandle';
};
export declare const cameraHandle: (raw: number) => CameraHandle;
export interface CameraPose {
    readonly position: readonly [number, number, number];
    readonly yawDegrees: number;
    readonly pitchDegrees: number;
}
export interface CameraBasis {
    readonly forward: readonly [number, number, number];
    readonly right: readonly [number, number, number];
    readonly up: readonly [number, number, number];
}
export interface PerspectiveProjection {
    readonly fovYDegrees: number;
    readonly near: number;
    readonly far: number;
}
export interface ViewportSize {
    readonly width: number;
    readonly height: number;
}
export interface CameraCreateRequest {
    readonly initialPose: CameraPose;
    readonly projection: PerspectiveProjection;
    readonly viewport: ViewportSize;
}
export interface FirstPersonCameraInput {
    readonly moveForward: number;
    readonly moveRight: number;
    readonly moveUp: number;
    readonly yawDeltaDegrees: number;
    readonly pitchDeltaDegrees: number;
    readonly dtSeconds: number;
    readonly moveSpeedUnitsPerSecond: number;
}
export interface FirstPersonCameraInputEnvelope {
    readonly camera: CameraHandle;
    readonly input: FirstPersonCameraInput;
    readonly tick: number;
}
export interface CameraProjectionRequest {
    readonly camera: CameraHandle;
    readonly viewport: ViewportSize | null;
}
export interface CameraSnapshot {
    readonly camera: CameraHandle;
    readonly tick: number;
    readonly pose: CameraPose;
    readonly basis: CameraBasis;
    readonly projection: PerspectiveProjection;
    readonly viewport: ViewportSize;
}
export interface CameraProjectionSnapshot {
    readonly camera: CameraHandle;
    readonly tick: number;
    readonly pose: CameraPose;
    readonly basis: CameraBasis;
    readonly projection: PerspectiveProjection;
    readonly viewport: ViewportSize;
    readonly viewMatrix: readonly [number, number, number, number, number, number, number, number, number, number, number, number, number, number, number, number];
    readonly projectionMatrix: readonly [number, number, number, number, number, number, number, number, number, number, number, number, number, number, number, number];
    readonly viewProjectionMatrix: readonly [number, number, number, number, number, number, number, number, number, number, number, number, number, number, number, number];
    readonly projectionHash: string;
}
//# sourceMappingURL=view.d.ts.map