import * as THREE from 'three';
import type { AnimatedMeshAsset, AnimatedMeshInstanceDescriptor, AnimatedMeshPlaybackCommand, RenderHandle } from '@asha/contracts';
export declare class AnimatedMeshApplyError extends Error {
    constructor(message: string);
}
export interface AnimatedMeshResource {
    readonly asset: string;
    readonly scene: THREE.Object3D;
    readonly clips: readonly THREE.AnimationClip[];
}
export interface AnimatedMeshAssetSource {
    getAnimatedMeshResource(asset: AnimatedMeshAsset): AnimatedMeshResource | undefined;
}
export interface AnimatedMeshPlaybackReadout {
    readonly handle: RenderHandle;
    readonly asset: string;
    readonly status: 'not_started' | 'playing' | 'paused' | 'stopped';
    readonly currentClip: string | null;
    readonly mixerTimeSeconds: number;
    readonly actionTimeSeconds: number | null;
    readonly running: boolean;
    readonly paused: boolean;
    readonly loop: 'once' | 'repeat' | 'pingPong' | null;
    readonly speed: number | null;
    readonly weight: number | null;
    readonly commandSelected: boolean;
    readonly poseSample: AnimatedMeshPoseSample;
    readonly diagnostics: readonly string[];
}
export interface AnimatedMeshPoseSample {
    readonly rootTranslation: readonly [number, number, number];
    readonly rootRotation: readonly [number, number, number, number];
    readonly rootScale: readonly [number, number, number];
}
interface AnimatedMeshInstanceRecord {
    readonly handle: RenderHandle;
    readonly asset: string;
    readonly object: THREE.Object3D;
    readonly mixer: THREE.AnimationMixer;
    readonly actions: ReadonlyMap<string, THREE.AnimationAction>;
    currentClip: string | null;
    commandSelected: boolean;
    status: AnimatedMeshPlaybackReadout['status'];
    loop: AnimatedMeshPlaybackReadout['loop'];
    speed: number | null;
    weight: number | null;
}
export declare class MapAnimatedMeshAssetSource implements AnimatedMeshAssetSource {
    #private;
    constructor(resources: readonly AnimatedMeshResource[]);
    getAnimatedMeshResource(asset: AnimatedMeshAsset): AnimatedMeshResource | undefined;
}
export declare function loadAnimatedMeshGlbResource(asset: string, data: ArrayBuffer): Promise<AnimatedMeshResource>;
export declare class AnimatedMeshRegistry {
    #private;
    constructor(assetSource: AnimatedMeshAssetSource | undefined);
    define(asset: AnimatedMeshAsset): void;
    create(handle: RenderHandle, instance: AnimatedMeshInstanceDescriptor): AnimatedMeshInstanceRecord;
    setPlayback(handle: RenderHandle, command: AnimatedMeshPlaybackCommand): void;
    advance(deltaSeconds: number): void;
    playback(handle: RenderHandle): AnimatedMeshPlaybackReadout | undefined;
    release(handle: RenderHandle): void;
}
export {};
//# sourceMappingURL=animated-mesh.d.ts.map