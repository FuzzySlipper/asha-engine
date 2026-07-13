import type { AudioClipRef, AudioProjectionDiagnostic, AudioProjectionReadout, PresentationFrameDiff, RuntimeProjectionFrame } from '@asha/contracts';
import type { AshaBillboardFrameReceipt, AshaBillboardHost } from './billboard-host.js';
import type { AshaParticleFrameReceipt, AshaParticleHost } from './particle-host.js';
import type { AshaTelemetryOverlayFrameReceipt, AshaTelemetryOverlayHost } from './telemetry-host.js';
import type { AshaAnimationFrameReceipt, AshaAnimationHost } from './animation-host.js';
export interface AshaAudioResource {
    readonly bytes: ArrayBuffer;
    readonly contentHash: string;
}
export type AshaAudioResourceResolver = (clip: AudioClipRef) => Promise<AshaAudioResource>;
export type AshaAudioEntityPositionResolver = (entity: number) => readonly [number, number, number] | null;
interface AshaAudioParam {
    setValueAtTime(value: number, time: number): void;
}
interface AshaAudioNode {
    connect(destination: AshaAudioNode): unknown;
    disconnect(): void;
}
interface AshaGainNode extends AshaAudioNode {
    readonly gain: AshaAudioParam;
}
interface AshaStereoPannerNode extends AshaAudioNode {
    readonly pan: AshaAudioParam;
}
interface AshaPannerNode extends AshaAudioNode {
    distanceModel: DistanceModelType;
    maxDistance: number;
    panningModel: PanningModelType;
    refDistance: number;
    rolloffFactor: number;
    readonly positionX: AshaAudioParam;
    readonly positionY: AshaAudioParam;
    readonly positionZ: AshaAudioParam;
}
interface AshaAudioListener {
    readonly forwardX: AshaAudioParam;
    readonly forwardY: AshaAudioParam;
    readonly forwardZ: AshaAudioParam;
    readonly positionX: AshaAudioParam;
    readonly positionY: AshaAudioParam;
    readonly positionZ: AshaAudioParam;
    readonly upX: AshaAudioParam;
    readonly upY: AshaAudioParam;
    readonly upZ: AshaAudioParam;
}
interface AshaBufferSourceNode extends AshaAudioNode {
    buffer: unknown;
    loop: boolean;
    onended: (() => void) | null;
    readonly playbackRate: AshaAudioParam;
    start(): void;
    stop(): void;
}
export interface AshaAudioContext {
    readonly currentTime: number;
    readonly destination: AshaAudioNode;
    readonly listener: AshaAudioListener;
    readonly state: AudioContextState;
    close(): Promise<void>;
    createBufferSource(): AshaBufferSourceNode;
    createGain(): AshaGainNode;
    createPanner(): AshaPannerNode;
    createStereoPanner(): AshaStereoPannerNode;
    decodeAudioData(bytes: ArrayBuffer): Promise<unknown>;
    resume(): Promise<void>;
}
export interface AshaAudioHostOptions {
    readonly createContext?: () => AshaAudioContext;
    readonly resolveEntityPosition?: AshaAudioEntityPositionResolver;
    readonly resolveResource: AshaAudioResourceResolver;
}
export interface AshaAudioListenerPose {
    readonly position: readonly [number, number, number];
    readonly forward: readonly [number, number, number];
    readonly up: readonly [number, number, number];
}
export interface AshaAudioFrameReceipt {
    readonly applied: number;
    readonly diagnostics: readonly AudioProjectionDiagnostic[];
    readonly readout: AudioProjectionReadout;
}
export declare class AshaAudioHost {
    #private;
    constructor(options: AshaAudioHostOptions);
    resume(): Promise<readonly AudioProjectionDiagnostic[]>;
    updateListener(pose: AshaAudioListenerPose): readonly AudioProjectionDiagnostic[];
    applyPresentation(presentation: PresentationFrameDiff): Promise<AshaAudioFrameReceipt>;
    readout(): AudioProjectionReadout;
    refreshLayout(): readonly AudioProjectionDiagnostic[];
    dispose(): Promise<void>;
}
export interface AshaRuntimeProjectionApplicationPorts {
    readonly applyScene: (frame: RuntimeProjectionFrame['scene']) => void;
    readonly audioHost?: AshaAudioHost;
    readonly billboardHost?: AshaBillboardHost;
    readonly particleHost?: AshaParticleHost;
    readonly telemetryOverlayHost?: AshaTelemetryOverlayHost;
    readonly animationHost?: AshaAnimationHost;
}
export interface AshaRuntimeProjectionApplicationReceipt {
    readonly authorityTick: number;
    readonly sceneApplied: boolean;
    readonly audio: AshaAudioFrameReceipt;
    readonly billboard: AshaBillboardFrameReceipt;
    readonly particle: AshaParticleFrameReceipt;
    readonly telemetryOverlay: AshaTelemetryOverlayFrameReceipt;
    readonly animation: AshaAnimationFrameReceipt;
}
export declare function applyAshaRuntimeProjectionFrame(frame: RuntimeProjectionFrame, ports: AshaRuntimeProjectionApplicationPorts): Promise<AshaRuntimeProjectionApplicationReceipt>;
export declare function validateRuntimeProjectionFrame(frame: RuntimeProjectionFrame): void;
export {};
//# sourceMappingURL=audio-host.d.ts.map