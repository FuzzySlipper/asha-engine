import type { Face, VoxelCoord } from './voxel.js';
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
export interface CameraCollisionShape {
    readonly halfExtents: readonly [number, number, number];
}
export type CameraCollisionPolicyMode = 'axis_separable_slide';
export interface CameraCollisionPolicy {
    readonly mode: CameraCollisionPolicyMode;
    readonly maxIterations: number;
}
export type GeneratedTunnelPreset = 'tiny-enclosed';
export interface GeneratedTunnelRuntimeApplyRequest {
    readonly preset: GeneratedTunnelPreset;
    readonly seed: number;
}
export interface GeneratedTunnelRuntimeApplyReceipt {
    readonly preset: GeneratedTunnelPreset;
    readonly seed: number;
    readonly grid: number;
    readonly configHash: string;
    readonly outputHash: string;
    readonly collisionSourceHash: string;
    readonly collisionProjectionHash: string;
}
export interface CollisionConstrainedCameraInputEnvelope {
    readonly camera: CameraHandle;
    readonly grid: number;
    readonly input: FirstPersonCameraInput;
    readonly tick: number;
    readonly shape: CameraCollisionShape;
    readonly policy: CameraCollisionPolicy;
}
export interface CollisionAabbEvidence {
    readonly min: readonly [number, number, number];
    readonly max: readonly [number, number, number];
}
export type CollisionAxis = 'x' | 'y' | 'z';
export interface CameraCollisionEvidence {
    readonly grid: number;
    readonly shape: CameraCollisionShape;
    readonly policy: CameraCollisionPolicy;
    readonly collided: boolean;
    readonly blockedAxes: readonly CollisionAxis[];
    readonly correction: readonly [number, number, number];
    readonly queriedAabb: CollisionAabbEvidence;
    readonly collisionSourceHash: string;
    readonly collisionProjectionHash: string;
}
export interface CameraCollisionSnapshot {
    readonly camera: CameraHandle;
    readonly tick: number;
    readonly before: CameraSnapshot;
    readonly attempted: CameraSnapshot;
    readonly after: CameraSnapshot;
    readonly collision: CameraCollisionEvidence;
    readonly movementHash: string;
}
export type ScreenPointSpace = 'normalized_0_1' | 'pixel';
export interface ScreenPoint {
    readonly x: number;
    readonly y: number;
    readonly space: ScreenPointSpace;
}
export interface ScreenPointToPickRayRequest {
    readonly camera: CameraHandle;
    readonly grid: number;
    readonly viewport: ViewportSize | null;
    readonly screenPoint: ScreenPoint;
    readonly maxDistance: number;
}
export interface PickRaySnapshot {
    readonly camera: CameraHandle;
    readonly tick: number;
    readonly grid: number;
    readonly screenPoint: ScreenPoint;
    readonly origin: readonly [number, number, number];
    readonly direction: readonly [number, number, number];
    readonly maxDistance: number;
    readonly cameraProjectionHash: string;
    readonly rayHash: string;
}
export type VoxelSelectionOutcome = 'hit' | 'miss';
export interface VoxelSelectionSnapshot {
    readonly pickRay: PickRaySnapshot;
    readonly outcome: VoxelSelectionOutcome;
    readonly selectedVoxel: VoxelCoord | null;
    readonly selectedFace: Face | null;
    readonly editAnchor: VoxelCoord | null;
    readonly selectionHash: string;
}
//# sourceMappingURL=view.d.ts.map