import type { RenderFrameDiff, RenderHandle } from '@asha/contracts';
import {
  MapAnimatedMeshAssetSource,
  ThreeRenderer,
  loadAnimatedMeshGlbResource,
  type AnimatedMeshAssetSource,
} from '@asha/renderer-three/backend';
import { rendererResourceContentHash } from './resource-content-hash.js';

export type AshaRendererHostDiagnosticCode =
  | 'animated_mesh_manifest_invalid'
  | 'animated_mesh_resource_unavailable'
  | 'animated_mesh_content_hash_mismatch'
  | 'animated_mesh_clip_unavailable'
  | 'animated_mesh_frame_rejected'
  | 'animated_mesh_handle_unavailable'
  | 'animation_not_started'
  | 'animation_paused'
  | 'animation_stopped';

export interface AshaRendererHostDiagnostic {
  readonly code: AshaRendererHostDiagnosticCode;
  readonly message: string;
  readonly asset: string | null;
  readonly handle: RenderHandle | null;
}

export class AshaRendererHostError extends Error {
  readonly diagnostics: readonly AshaRendererHostDiagnostic[];

  constructor(diagnostics: readonly AshaRendererHostDiagnostic[]) {
    super(diagnostics.map((diagnostic) => diagnostic.message).join('; '));
    this.name = 'AshaRendererHostError';
    this.diagnostics = diagnostics;
  }
}

export interface AshaRendererAnimatedMeshResourceDescriptor {
  readonly asset: string;
  readonly resourceUrl: string;
  readonly contentHash: string;
  readonly clipIds: readonly string[];
  readonly licenseUrl: string | null;
}

export interface AshaRendererAnimatedMeshResourceManifest {
  readonly kind: 'asha_renderer_animated_mesh_resources.v0';
  readonly resources: readonly AshaRendererAnimatedMeshResourceDescriptor[];
}

export type AshaRendererAnimatedMeshResourceResolver = (
  descriptor: AshaRendererAnimatedMeshResourceDescriptor,
) => Promise<ArrayBuffer>;

export const ASHA_RENDERER_HOST_KENNEY_ANIMATED_MESH_RESOURCE: AshaRendererAnimatedMeshResourceDescriptor = {
  asset: 'mesh-animation/kenney-retro-character-medium',
  resourceUrl: new URL('../assets/kenney-retro-character-medium.glb', import.meta.url).href,
  contentHash: 'sha256:c71255a41c0373f0d2ef52593369d5fd9d2f6220ae548aff8cd6bf5edb403674',
  clipIds: ['idle', 'run', 'jump'],
  licenseUrl: new URL('../assets/LICENSE.Kenney-Animated-Characters-Retro.txt', import.meta.url).href,
};

export const ASHA_RENDERER_HOST_ANIMATED_MESH_FIXTURE_MANIFEST: AshaRendererAnimatedMeshResourceManifest = {
  kind: 'asha_renderer_animated_mesh_resources.v0',
  resources: [ASHA_RENDERER_HOST_KENNEY_ANIMATED_MESH_RESOURCE],
};

export interface AshaRendererAnimatedMeshFrameReceipt {
  readonly applied: boolean;
  readonly diagnostics: readonly AshaRendererHostDiagnostic[];
}

export interface AshaRendererAnimatedMeshPoseSample {
  readonly rootTranslation: readonly [number, number, number];
  readonly rootRotation: readonly [number, number, number, number];
  readonly rootScale: readonly [number, number, number];
  readonly hierarchyNodeCount: number;
  readonly hierarchyTranslationSum: readonly [number, number, number];
  readonly hierarchyRotationSum: readonly [number, number, number, number];
  readonly hierarchyScaleSum: readonly [number, number, number];
}

export interface AshaRendererAnimatedMeshPlaybackReadout {
  readonly handle: RenderHandle;
  readonly asset: string | null;
  readonly status: 'unavailable' | 'not_started' | 'playing' | 'paused' | 'stopped';
  readonly selectedClip: string | null;
  readonly mixerTimeSeconds: number;
  readonly actionTimeSeconds: number | null;
  readonly commandSelected: boolean;
  readonly running: boolean;
  readonly paused: boolean;
  readonly loop: 'once' | 'repeat' | 'pingPong' | null;
  readonly speed: number | null;
  readonly weight: number | null;
  readonly poseSample: AshaRendererAnimatedMeshPoseSample | null;
  readonly diagnostics: readonly AshaRendererHostDiagnostic[];
  readonly projectionOnly: true;
  readonly controllerClips: readonly AshaRendererAnimationControllerClip[];
}

export interface AshaRendererAnimatedMeshProjection {
  readonly kind: 'asha_renderer_animated_mesh_projection.v0';
  readonly applyFrame: (frame: RenderFrameDiff) => AshaRendererAnimatedMeshFrameReceipt;
  readonly advance: (deltaSeconds: number) => AshaRendererAnimatedMeshFrameReceipt;
  readonly playback: (handle: RenderHandle) => AshaRendererAnimatedMeshPlaybackReadout;
  readonly snapshot: () => string;
  readonly hasAnimationTarget: (handle: RenderHandle) => boolean;
  readonly setAnimationControllerWeights: (
    handle: RenderHandle,
    clips: readonly AshaRendererAnimationControllerClip[],
  ) => void;
  readonly hasAnimationClips: (handle: RenderHandle, clipIds: readonly string[]) => boolean;
  readonly clearAnimationControllerWeights: (handle: RenderHandle) => void;
}

export interface AshaRendererAnimationControllerClip {
  readonly clip: string;
  readonly weight: number;
  readonly speed: number;
}

export interface AshaRendererAnimatedMeshProjectionOptions {
  readonly manifest: AshaRendererAnimatedMeshResourceManifest;
  readonly resolveResource?: AshaRendererAnimatedMeshResourceResolver;
}

export async function createAshaRendererAnimatedMeshProjection(
  options: AshaRendererAnimatedMeshProjectionOptions,
): Promise<AshaRendererAnimatedMeshProjection> {
  const source = await loadRendererAnimatedMeshSource(options.manifest, options.resolveResource);
  const renderer = new ThreeRenderer({ animatedMeshSource: source as AnimatedMeshAssetSource });
  return createProjectionController(renderer);
}

export async function loadRendererAnimatedMeshSource(
  manifest: AshaRendererAnimatedMeshResourceManifest,
  resolver: AshaRendererAnimatedMeshResourceResolver = resolveAnimatedMeshWithFetch,
): Promise<unknown> {
  validateManifest(manifest);
  const resources = await Promise.all(manifest.resources.map(async (descriptor) => {
    let data: ArrayBuffer;
    try {
      data = await resolver(descriptor);
    } catch (cause) {
      throw hostError('animated_mesh_resource_unavailable', descriptor.asset, null, cause);
    }
    const actualHash = await rendererResourceContentHash(data, descriptor.contentHash);
    if (actualHash !== descriptor.contentHash) {
      throw hostError(
        'animated_mesh_content_hash_mismatch',
        descriptor.asset,
        null,
        `expected ${descriptor.contentHash}, received ${actualHash}`,
      );
    }
    const resource = await loadAnimatedMeshGlbResource(descriptor.asset, data, descriptor.contentHash).catch((cause: unknown) => {
      throw hostError('animated_mesh_resource_unavailable', descriptor.asset, null, cause);
    });
    const availableClips = new Set(resource.clips.map((clip) => clip.name.toLowerCase()));
    const missingClip = descriptor.clipIds.find((clip) => !availableClips.has(clip.toLowerCase()));
    if (missingClip !== undefined) {
      throw hostError('animated_mesh_clip_unavailable', descriptor.asset, null, `missing clip ${missingClip}`);
    }
    return resource;
  }));
  return new MapAnimatedMeshAssetSource(resources);
}

export function animationPlaybackReadout(
  handle: RenderHandle,
  readout: BackendAnimatedMeshPlaybackReadout | undefined,
): AshaRendererAnimatedMeshPlaybackReadout {
  if (readout === undefined) {
    return {
      handle,
      asset: null,
      status: 'unavailable',
      selectedClip: null,
      mixerTimeSeconds: 0,
      actionTimeSeconds: null,
      commandSelected: false,
      running: false,
      paused: false,
      loop: null,
      speed: null,
      weight: null,
      poseSample: null,
      diagnostics: [diagnostic('animated_mesh_handle_unavailable', null, handle, `animated mesh handle ${handle} is unavailable`)],
      projectionOnly: true,
      controllerClips: [],
    };
  }
  return {
    handle,
    asset: readout.asset,
    status: readout.status,
    selectedClip: readout.currentClip,
    mixerTimeSeconds: readout.mixerTimeSeconds,
    actionTimeSeconds: readout.actionTimeSeconds,
    commandSelected: readout.commandSelected,
    running: readout.running,
    paused: readout.paused,
    loop: readout.loop,
    speed: readout.speed,
    weight: readout.weight,
    poseSample: readout.poseSample,
    diagnostics: readout.diagnostics.map((code) => diagnostic(animationDiagnosticCode(code), readout.asset, handle, code)),
    projectionOnly: true,
    controllerClips: readout.controllerClips,
  };
}

interface BackendAnimatedMeshPlaybackReadout {
  readonly asset: string;
  readonly status: 'not_started' | 'playing' | 'paused' | 'stopped';
  readonly currentClip: string | null;
  readonly mixerTimeSeconds: number;
  readonly actionTimeSeconds: number | null;
  readonly commandSelected: boolean;
  readonly running: boolean;
  readonly paused: boolean;
  readonly loop: 'once' | 'repeat' | 'pingPong' | null;
  readonly speed: number | null;
  readonly weight: number | null;
  readonly poseSample: AshaRendererAnimatedMeshPoseSample;
  readonly diagnostics: readonly string[];
  readonly controllerClips: readonly AshaRendererAnimationControllerClip[];
}

function createProjectionController(renderer: ThreeRenderer): AshaRendererAnimatedMeshProjection {
  return {
    kind: 'asha_renderer_animated_mesh_projection.v0',
    applyFrame: (frame) => applyRendererOperation(() => renderer.applyFrame(frame)),
    advance: (deltaSeconds) => applyRendererOperation(() => renderer.advanceAnimation(deltaSeconds)),
    playback: (handle) => animationPlaybackReadout(handle, renderer.animatedMeshPlayback(handle)),
    snapshot: () => renderer.snapshot(),
    hasAnimationTarget: (handle) => renderer.has(handle),
    setAnimationControllerWeights: (handle, clips) => {
      renderer.setAnimationControllerWeights(handle, clips);
    },
    hasAnimationClips: (handle, clipIds) => renderer.hasAnimationControllerClips(handle, clipIds),
    clearAnimationControllerWeights: (handle) => renderer.clearAnimationControllerWeights(handle),
  };
}

function applyRendererOperation(operation: () => void): AshaRendererAnimatedMeshFrameReceipt {
  try {
    operation();
    return { applied: true, diagnostics: [] };
  } catch (cause) {
    return {
      applied: false,
      diagnostics: [diagnostic('animated_mesh_frame_rejected', null, null, errorMessage(cause))],
    };
  }
}

async function resolveAnimatedMeshWithFetch(descriptor: AshaRendererAnimatedMeshResourceDescriptor): Promise<ArrayBuffer> {
  const response = await fetch(descriptor.resourceUrl);
  if (!response.ok) {
    throw new Error(`resource request failed with HTTP ${response.status}`);
  }
  return response.arrayBuffer();
}

function validateManifest(manifest: AshaRendererAnimatedMeshResourceManifest): void {
  if (manifest.kind !== 'asha_renderer_animated_mesh_resources.v0' || manifest.resources.length === 0) {
    throw hostError('animated_mesh_manifest_invalid', null, null, 'animated mesh resource manifest is empty or unsupported');
  }
  const assets = new Set<string>();
  for (const resource of manifest.resources) {
    const validHash = /^(?:sha256:[0-9a-f]{64}|[0-9a-f]{16})$/u.test(resource.contentHash);
    const validClips = resource.clipIds.length > 0 && new Set(resource.clipIds).size === resource.clipIds.length;
    if (resource.asset.length === 0 || resource.resourceUrl.length === 0 || !validHash || !validClips || assets.has(resource.asset)) {
      throw hostError('animated_mesh_manifest_invalid', resource.asset || null, null, 'animated mesh resource descriptor is invalid or duplicated');
    }
    assets.add(resource.asset);
  }
}

function animationDiagnosticCode(code: string): AshaRendererHostDiagnosticCode {
  switch (code) {
    case 'animation_not_started':
    case 'animation_paused':
    case 'animation_stopped':
      return code;
    default:
      return 'animated_mesh_frame_rejected';
  }
}

function hostError(
  code: AshaRendererHostDiagnosticCode,
  asset: string | null,
  handle: RenderHandle | null,
  cause: unknown,
): AshaRendererHostError {
  return new AshaRendererHostError([diagnostic(code, asset, handle, errorMessage(cause))]);
}

function diagnostic(
  code: AshaRendererHostDiagnosticCode,
  asset: string | null,
  handle: RenderHandle | null,
  message: string,
): AshaRendererHostDiagnostic {
  return { code, message, asset, handle };
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
